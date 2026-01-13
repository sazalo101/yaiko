# Metrics

Yaiko provides built-in support for Prometheus metrics to help you monitor your application's performance and health.

## enabling Metrics

To use metrics, you must enable the `metrics` feature in your `Cargo.toml`:

```toml
[dependencies]
yaiko-core = { version = "0.1.0", features = ["metrics"] }
```

## Usage

### 1. Initialize Metrics

Create a `Metrics` instance and wrap it in an `Arc`:

```rust
use yaiko_core::metrics::Metrics;
use std::sync::Arc;

let metrics = Arc::new(Metrics::new());
```

### 2. Add Middleware

Add the `MetricsMiddleware` to your router to automatically track request duration, counts, and errors:

```rust
use yaiko_core::metrics::MetricsMiddleware;

let router = Router::new()
    // ... routes ...
    .use_middleware(MetricsMiddleware::new(metrics.clone()));
```

### 3. Expose Metrics Endpoint

Add a route to expose the collected metrics in Prometheus format:

```rust
let metrics_clone = metrics.clone();
let router = router.get("/metrics", move |_| {
    let m = metrics_clone.clone();
    async move {
        match m.export() {
            Ok(data) => Ok(Response::new().text(data)),
            Err(e) => Ok(Response::new().status(StatusCode::INTERNAL_SERVER_ERROR).text(e.to_string())),
        }
    }
});
```

## Collected Metrics

The following metrics are collected by default:

- `http_requests_total`: Total number of HTTP requests (Counter).
- `http_request_duration_seconds`: Histogram of request durations.
- `http_errors_total`: Total number of failed requests (Counter).

## Integration with Prometheus

Configure your Prometheus server to scrape the `/metrics` endpoint of your Yaiko application.
