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

impl Default for SecurityHeaders {
    fn default() -> Self {
        Self::new()
    }
}

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
            .header("Permissions-Policy", "geolocation=(), microphone=(), camera=()")
            .header("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'")
            .header("Strict-Transport-Security", "max-age=31536000; includeSubDomains");

        Ok(response)
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    requests_per_window: u32,
    window_duration: Duration,
    buckets: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
    trust_proxy_headers: bool,
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
                    map.retain(|_, &mut (_, timestamp)| {
                        now.duration_since(timestamp) <= window_duration
                    });
                }
            }
        });

        Self {
            requests_per_window,
            window_duration,
            buckets,
            trust_proxy_headers: false,
        }
    }

    pub fn trust_proxy_headers(mut self, trust: bool) -> Self {
        self.trust_proxy_headers = trust;
        self
    }

    fn get_client_ip(&self, req: &Request) -> String {
        if self.trust_proxy_headers {
            if let Some(ip) = req
                .headers
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or("unknown").trim().to_string())
                .filter(|s| !s.is_empty())
            {
                return ip;
            }

            if let Some(ip) = req
                .headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
            {
                return ip;
            }
        }

        req.remote_addr
            .map(|addr| addr.ip().to_string())
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
        let client_ip = self.get_client_ip(&req);

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
                .header(
                    "Retry-After",
                    self.window_duration.as_secs().to_string().as_str(),
                )
                .text(r#"{"error": "Rate limit exceeded"}"#));
        }

        next.handle(req).await
    }
}

/// CSRF protection middleware using double-submit cookie pattern
pub struct CsrfProtection {
    cookie_name: String,
    header_name: String,
    form_field_name: String,
}

impl Default for CsrfProtection {
    fn default() -> Self {
        Self::new()
    }
}

impl CsrfProtection {
    pub fn new() -> Self {
        Self {
            cookie_name: "csrf_token".to_string(),
            header_name: "X-CSRF-Token".to_string(),
            form_field_name: "_csrf".to_string(),
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
        mut req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let method = req.method.as_str();
        let cookie_token = extract_cookie_value(req.header("cookie"), &self.cookie_name);

        // Skip CSRF check for safe methods
        if method == "GET" || method == "HEAD" || method == "OPTIONS" {
            let mut response = next.handle(req).await?;

            // Reuse the existing token so cached pages and multi-tab flows remain valid.
            let token = cookie_token.unwrap_or_else(Self::generate_token);
            response = response.header(
                "Set-Cookie",
                &format!("{}={}; Path=/; SameSite=Strict", self.cookie_name, token),
            );

            return Ok(response);
        }

        // For unsafe methods, verify CSRF token
        let header_token = req.header(&self.header_name).map(|s| s.to_string());
        let form_token = if req
            .header("content-type")
            .map(|ct| ct.starts_with("application/x-www-form-urlencoded"))
            .unwrap_or(false)
        {
            req.form_data().await?.get(&self.form_field_name).cloned()
        } else {
            None
        };

        let request_token = header_token.or(form_token);

        match (cookie_token, request_token) {
            (Some(cookie), Some(header)) if cookie == header => next.handle(req).await,
            _ => Ok(Response::new()
                .status(StatusCode::FORBIDDEN)
                .text(r#"{"error": "CSRF token validation failed"}"#)),
        }
    }
}

fn extract_cookie_value(cookie_header: Option<&str>, cookie_name: &str) -> Option<String> {
    cookie_header.and_then(|cookies| {
        cookies.split(';').find_map(|cookie| {
            let (name, value) = cookie.trim().split_once('=')?;

            (name == cookie_name).then(|| value.to_string())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Body;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn csrf_cookie_is_reusable_and_not_httponly() {
        let middleware = CsrfProtection::new();
        let next = Arc::new(|_req: Request| async { Ok(Response::new().text("ok")) });

        let get_req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let first = middleware.handle(get_req, next.clone()).await.unwrap();
        let set_cookie = first.headers.get("Set-Cookie").unwrap().clone();

        assert!(!set_cookie.contains("HttpOnly"));

        let post_req = Request::from_hyper(
            hyper::Request::builder()
                .method("POST")
                .uri("/")
                .header("cookie", &set_cookie)
                .header(
                    "X-CSRF-Token",
                    extract_cookie_value(Some(&set_cookie), "csrf_token").unwrap(),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        let response = middleware.handle(post_req, next).await.unwrap();

        assert_eq!(response.status, StatusCode::OK);
    }

    #[tokio::test]
    async fn rate_limiter_ignores_spoofed_proxy_headers_by_default() {
        let limiter = RateLimiter::new(1, 60);
        let next = Arc::new(|_req: Request| async { Ok(Response::new().text("ok")) });
        let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000);

        let first = Request::from_hyper_with_addr(
            hyper::Request::builder()
                .method("GET")
                .uri("/")
                .header("x-forwarded-for", "1.1.1.1")
                .body(Body::empty())
                .unwrap(),
            Some(remote_addr),
        )
        .await
        .unwrap();
        assert_eq!(
            limiter.handle(first, next.clone()).await.unwrap().status,
            StatusCode::OK
        );

        let second = Request::from_hyper_with_addr(
            hyper::Request::builder()
                .method("GET")
                .uri("/")
                .header("x-forwarded-for", "8.8.8.8")
                .body(Body::empty())
                .unwrap(),
            Some(remote_addr),
        )
        .await
        .unwrap();
        assert_eq!(
            limiter.handle(second, next).await.unwrap().status,
            StatusCode::TOO_MANY_REQUESTS
        );
    }
}
