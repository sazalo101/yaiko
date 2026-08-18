//! Async dependency health registry for readiness and operational diagnostics.

use crate::{Handler, Request, Response};
use async_trait::async_trait;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

pub type ProbeFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
pub type ProbeFn = Box<dyn Fn() -> ProbeFuture + Send + Sync>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProbeResult {
    pub name: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HealthReport {
    pub status: String,
    pub ready: bool,
    pub checks: Vec<ProbeResult>,
}

impl HealthReport {
    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

struct Probe {
    name: String,
    check: ProbeFn,
}

pub struct DependencyHealthRegistry {
    probes: Vec<Probe>,
    timeout: Duration,
}

impl DependencyHealthRegistry {
    pub fn new() -> Self {
        Self {
            probes: Vec::new(),
            timeout: Duration::from_secs(2),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn add_probe<F, Fut>(mut self, name: impl Into<String>, check: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.probes.push(Probe {
            name: name.into(),
            check: Box::new(move || Box::pin(check())),
        });
        self
    }

    pub async fn check(&self) -> HealthReport {
        let mut checks = Vec::with_capacity(self.probes.len());
        let mut ready = true;
        for probe in &self.probes {
            let result = tokio::time::timeout(self.timeout, (probe.check)()).await;
            let probe_result = match result {
                Ok(Ok(())) => ProbeResult {
                    name: probe.name.clone(),
                    status: "ok".to_string(),
                    detail: None,
                },
                Ok(Err(error)) => {
                    ready = false;
                    ProbeResult {
                        name: probe.name.clone(),
                        status: "failed".to_string(),
                        detail: Some(error),
                    }
                }
                Err(_) => {
                    ready = false;
                    ProbeResult {
                        name: probe.name.clone(),
                        status: "timeout".to_string(),
                        detail: Some(format!("timed out after {}ms", self.timeout.as_millis())),
                    }
                }
            };
            checks.push(probe_result);
        }
        HealthReport {
            status: if ready { "ok" } else { "degraded" }.to_string(),
            ready,
            checks,
        }
    }

    pub fn handler(self: Arc<Self>) -> DependencyHealthHandler {
        DependencyHealthHandler { registry: self }
    }
}

impl Default for DependencyHealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DependencyHealthHandler {
    registry: Arc<DependencyHealthRegistry>,
}

#[async_trait]
impl Handler for DependencyHealthHandler {
    async fn handle(
        &self,
        _request: Request,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let report = self.registry.check().await;
        let status = if report.ready {
            hyper::StatusCode::OK
        } else {
            hyper::StatusCode::SERVICE_UNAVAILABLE
        };
        Ok(Response::new()
            .status(status)
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .json(&report)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn aggregates_success_and_failure_probes() {
        let registry = DependencyHealthRegistry::new()
            .add_probe("database", || async { Ok(()) })
            .add_probe("cache", || async { Err("offline".to_string()) });
        let report = registry.check().await;
        assert!(!report.is_ready());
        assert_eq!(report.status, "degraded");
        assert_eq!(report.checks[0].status, "ok");
        assert_eq!(report.checks[1].detail.as_deref(), Some("offline"));
    }

    #[tokio::test]
    async fn reports_timed_out_dependencies_as_unready() {
        let registry = DependencyHealthRegistry::new()
            .timeout(Duration::from_millis(5))
            .add_probe("slow", || async {
                tokio::time::sleep(Duration::from_millis(30)).await;
                Ok(())
            });
        let report = registry.check().await;
        assert!(!report.ready);
        assert_eq!(report.checks[0].status, "timeout");
    }

    #[tokio::test]
    async fn empty_registry_is_ready() {
        let report = DependencyHealthRegistry::new().check().await;
        assert!(report.ready);
        assert_eq!(report.status, "ok");
        assert!(report.checks.is_empty());
    }
}
