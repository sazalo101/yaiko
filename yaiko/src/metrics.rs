use crate::{Handler, Middleware, Request, Response};
use async_trait::async_trait;
use prometheus::{Counter, Encoder, Histogram, HistogramOpts, Registry, TextEncoder};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub struct Metrics {
    pub request_counter: Counter,
    pub request_duration: Histogram,
    pub error_counter: Counter,
    registry: Arc<Registry>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Arc::new(Registry::new());

        let request_counter = Counter::new("http_requests_total", "Total HTTP requests")
            .expect("Failed to create request counter");

        let request_duration = Histogram::with_opts(HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration",
        ))
        .expect("Failed to create request duration histogram");

        let error_counter = Counter::new("http_errors_total", "Total HTTP errors")
            .expect("Failed to create error counter");

        registry
            .register(Box::new(request_counter.clone()))
            .unwrap();
        registry
            .register(Box::new(request_duration.clone()))
            .unwrap();
        registry.register(Box::new(error_counter.clone())).unwrap();

        Metrics {
            request_counter,
            request_duration,
            error_counter,
            registry,
        }
    }

    pub fn export(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

pub struct MetricsMiddleware {
    metrics: Arc<Metrics>,
}

impl MetricsMiddleware {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        MetricsMiddleware { metrics }
    }
}

#[async_trait]
impl Middleware for MetricsMiddleware {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let start = Instant::now();
        self.metrics.request_counter.inc();

        let result = next.handle(req).await;

        let duration = start.elapsed();
        self.metrics
            .request_duration
            .observe(duration.as_secs_f64());

        if result.is_err() {
            self.metrics.error_counter.inc();
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn metrics_can_be_created_and_exported() {
        let metrics = Metrics::new();
        let exported = metrics.export().expect("metrics export should succeed");
        assert!(exported.contains("http_requests_total"));
        assert!(exported.contains("http_request_duration_seconds"));
        assert!(exported.contains("http_errors_total"));
    }
}
