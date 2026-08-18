//! Logging middleware for Yaiko applications
//!
//! Provides request/response logging using tracing.

use crate::{Handler, Middleware, Request, Response};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

/// Logging middleware that logs all requests
pub struct LoggingMiddleware {
    /// Log request bodies (default: false)
    log_body: bool,
}

impl LoggingMiddleware {
    /// Create a new logging middleware
    pub fn new() -> Self {
        Self { log_body: false }
    }

    /// Enable logging request bodies
    pub fn with_body(mut self) -> Self {
        self.log_body = true;
        self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        let method = req.method.clone();
        let path = req.uri.path().to_string();
        let query = req
            .uri
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();

        // Log request
        tracing::info!(
            method = %method,
            path = %path,
            query = %query,
            "Request started"
        );

        // Process request
        let result = next.handle(req).await;

        let duration = start.elapsed();

        match &result {
            Ok(response) => {
                let status = response.status.as_u16();
                if status >= 400 {
                    tracing::warn!(
                        method = %method,
                        path = %path,
                        status = status,
                        duration_ms = duration.as_millis() as u64,
                        "Request failed"
                    );
                } else {
                    tracing::info!(
                        method = %method,
                        path = %path,
                        status = status,
                        duration_ms = duration.as_millis() as u64,
                        "Request completed"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    method = %method,
                    path = %path,
                    error = %e,
                    duration_ms = duration.as_millis() as u64,
                    "Request error"
                );
            }
        }

        result
    }
}

/// Initialize tracing with a default subscriber
///
/// Call this at the start of your application:
/// ```rust
/// yaiko_core::init_tracing();
/// ```
pub fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();
}
