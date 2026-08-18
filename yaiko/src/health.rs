//! Health check utilities for Yaiko applications
//!
//! Provides a standard health check endpoint.

use crate::{Handler, Request, Response};
use async_trait::async_trait;

/// Health check handler returning {"status": "ok"}
pub struct HealthCheck;

impl HealthCheck {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for HealthCheck {
    async fn handle(
        &self,
        _req: Request,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Response::new()
            .status(hyper::StatusCode::OK)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .text(r#"{"status":"ok"}"#))
    }
}

/// Detailed health check with custom checks
pub struct DetailedHealthCheck {
    #[allow(clippy::type_complexity)]
    checks: Vec<(&'static str, Box<dyn Fn() -> bool + Send + Sync>)>,
}

impl DetailedHealthCheck {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Return whether every registered dependency check currently passes.
    pub fn is_ready(&self) -> bool {
        self.checks.iter().all(|(_, check)| check())
    }

    /// Add a health check
    pub fn add_check<F>(mut self, name: &'static str, check: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.checks.push((name, Box::new(check)));
        self
    }
}

impl Default for DetailedHealthCheck {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Handler for DetailedHealthCheck {
    async fn handle(
        &self,
        _req: Request,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let mut all_ok = true;
        let mut results = Vec::new();

        for (name, check) in &self.checks {
            let ok = check();
            if !ok {
                all_ok = false;
            }
            results.push(format!(
                r#""{}":{}"#,
                name,
                if ok { "true" } else { "false" }
            ));
        }

        let status = if all_ok { "ok" } else { "degraded" };
        let body = format!(
            r#"{{"status":"{}","checks":{{{}}}}}"#,
            status,
            results.join(",")
        );

        let http_status = if all_ok {
            hyper::StatusCode::OK
        } else {
            hyper::StatusCode::SERVICE_UNAVAILABLE
        };

        Ok(Response::new()
            .status(http_status)
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .text(&body))
    }
}

/// Liveness is a process-level check with no dependency probes.
pub type LivenessCheck = HealthCheck;

/// Readiness is a dependency-aware detailed health check.
pub type ReadinessCheck = DetailedHealthCheck;
