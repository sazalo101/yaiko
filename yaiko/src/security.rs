//! Security middleware for Yaiko applications
//!
//! Provides CSRF protection, rate limiting, and security headers.

use crate::{Handler, Middleware, Request, Response, StatusCode};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Security headers middleware
/// Adds common security headers to all responses
pub struct SecurityHeaders;

impl SecurityHeaders {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Middleware for SecurityHeaders {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let mut response = next.handle(req).await?;
        
        // Add security headers
        response = response
            .header("X-Content-Type-Options", "nosniff")
            .header("X-Frame-Options", "DENY")
            .header("X-XSS-Protection", "1; mode=block")
            .header("Referrer-Policy", "strict-origin-when-cross-origin")
            .header("Permissions-Policy", "geolocation=(), microphone=(), camera=()");
        
        Ok(response)
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    requests_per_window: u32,
    window_duration: Duration,
    buckets: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl RateLimiter {
    pub fn new(requests_per_window: u32, window_secs: u64) -> Self {
        let window_duration = Duration::from_secs(window_secs);
        let buckets = Arc::new(Mutex::new(HashMap::new()));
        
        // Background cleanup task
        let cleanup_buckets = buckets.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                if let Ok(mut map) = cleanup_buckets.lock() {
                    let now = Instant::now();
                    map.retain(|_, &mut (_, timestamp)| now.duration_since(timestamp) <= window_duration);
                }
            }
        });

        Self {
            requests_per_window,
            window_duration,
            buckets,
        }
    }

    fn get_client_ip(req: &Request) -> String {
        // Try X-Forwarded-For first, then X-Real-IP, then remote addr
        req.headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or("unknown").trim().to_string())
            .or_else(|| {
                req.headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    }
}

#[async_trait]
impl Middleware for RateLimiter {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let client_ip = Self::get_client_ip(&req);
        
        let allowed = {
            let mut buckets = self.buckets.lock().unwrap();
            let now = Instant::now();
            
            let entry = buckets.entry(client_ip.clone()).or_insert((0, now));
            
            // Reset if window expired
            if now.duration_since(entry.1) > self.window_duration {
                *entry = (0, now);
            }
            
            if entry.0 < self.requests_per_window {
                entry.0 += 1;
                true
            } else {
                false
            }
        };
        
        if !allowed {
            return Ok(Response::new()
                .status(StatusCode::TOO_MANY_REQUESTS)
                .header("Retry-After", self.window_duration.as_secs().to_string().as_str())
                .text(r#"{"error": "Rate limit exceeded"}"#));
        }
        
        next.handle(req).await
    }
}

/// CSRF protection middleware using double-submit cookie pattern
pub struct CsrfProtection {
    cookie_name: String,
    header_name: String,
}

impl CsrfProtection {
    pub fn new() -> Self {
        Self {
            cookie_name: "csrf_token".to_string(),
            header_name: "X-CSRF-Token".to_string(),
        }
    }

    fn generate_token() -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[async_trait]
impl Middleware for CsrfProtection {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let method = req.method.as_str();
        
        // Skip CSRF check for safe methods
        if method == "GET" || method == "HEAD" || method == "OPTIONS" {
            let mut response = next.handle(req).await?;
            
            // Set CSRF token cookie on safe requests
            let token = Self::generate_token();
            response = response.header(
                "Set-Cookie",
                &format!("{}={}; Path=/; HttpOnly; SameSite=Strict", self.cookie_name, token),
            );
            
            return Ok(response);
        }
        
        // For unsafe methods, verify CSRF token
        let cookie_token = req
            .headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .find_map(|c| {
                        let mut parts = c.trim().splitn(2, '=');
                        let name = parts.next()?;
                        let value = parts.next()?;
                        if name == self.cookie_name {
                            Some(value.to_string())
                        } else {
                            None
                        }
                    })
            });
        
        let header_token = req
            .headers
            .get(&self.header_name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        
        match (cookie_token, header_token) {
            (Some(cookie), Some(header)) if cookie == header => {
                next.handle(req).await
            }
            _ => {
                Ok(Response::new()
                    .status(StatusCode::FORBIDDEN)
                    .text(r#"{"error": "CSRF token validation failed"}"#))
            }
        }
    }
}
