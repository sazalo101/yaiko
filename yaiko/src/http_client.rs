//! Outbound HTTP request policy and client helpers.

use crate::TraceContext;
use hyper::{Body, Client, Method, Request as HyperRequest, StatusCode, Uri};
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRetryPolicy {
    pub max_attempts: u32,
    pub backoff: Duration,
    pub retry_statuses: Vec<StatusCode>,
}

impl Default for HttpRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: Duration::from_millis(100),
            retry_statuses: vec![
                StatusCode::REQUEST_TIMEOUT,
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::BAD_GATEWAY,
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::GATEWAY_TIMEOUT,
            ],
        }
    }
}

impl HttpRetryPolicy {
    pub fn should_retry(&self, attempt: u32, status: Option<StatusCode>) -> bool {
        attempt + 1 < self.max_attempts
            && status
                .map(|status| self.retry_statuses.contains(&status))
                .unwrap_or(true)
    }
    pub fn delay(&self, attempt: u32) -> Duration {
        self.backoff.saturating_mul(2u32.saturating_pow(attempt))
    }
}

#[derive(Debug, Clone)]
pub struct HttpRequestSpec {
    pub method: Method,
    pub uri: Uri,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    pub timeout: Duration,
    pub retry: HttpRetryPolicy,
}

impl HttpRequestSpec {
    pub fn new(method: Method, uri: Uri) -> Self {
        Self {
            method,
            uri,
            headers: BTreeMap::new(),
            body: Vec::new(),
            timeout: Duration::from_secs(10),
            retry: HttpRetryPolicy::default(),
        }
    }
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
    pub fn retry(mut self, retry: HttpRetryPolicy) -> Self {
        self.retry = retry;
        self
    }
    pub fn with_trace(mut self, context: &TraceContext) -> Self {
        for (name, value) in context.to_headers() {
            self.headers.insert(name, value);
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpClientError {
    InvalidRequest(String),
    Timeout,
    Transport(String),
    Response(StatusCode),
}

#[derive(Clone)]
pub struct OutboundHttpClient {
    client: Client<hyper::client::HttpConnector, Body>,
}

impl OutboundHttpClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn execute(&self, spec: HttpRequestSpec) -> Result<HttpResponse, HttpClientError> {
        let mut builder = HyperRequest::builder().method(spec.method).uri(spec.uri);
        for (name, value) in &spec.headers {
            builder = builder.header(name, value);
        }
        let request = builder
            .body(Body::from(spec.body))
            .map_err(|error| HttpClientError::InvalidRequest(error.to_string()))?;
        let response = tokio::time::timeout(spec.timeout, self.client.request(request))
            .await
            .map_err(|_| HttpClientError::Timeout)?
            .map_err(|error| HttpClientError::Transport(error.to_string()))?;
        let status = response.status();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|value| (name.as_str().to_string(), value.to_string()))
            })
            .collect();
        let body = hyper::body::to_bytes(response.into_body())
            .await
            .map_err(|error| HttpClientError::Transport(error.to_string()))?
            .to_vec();
        if !status.is_success() {
            return Err(HttpClientError::Response(status));
        }
        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}

impl Default for OutboundHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_typed_spec_with_trace_propagation() {
        let context = TraceContext::new(true);
        let spec = HttpRequestSpec::new(Method::POST, "http://example.com/hook".parse().unwrap())
            .body("payload")
            .with_trace(&context);
        assert_eq!(
            spec.headers["traceparent"],
            context.to_headers()["traceparent"]
        );
        assert_eq!(spec.body, b"payload");
    }

    #[test]
    fn classifies_retryable_statuses_and_exponential_delays() {
        let policy = HttpRetryPolicy::default();
        assert!(policy.should_retry(0, Some(StatusCode::SERVICE_UNAVAILABLE)));
        assert!(!policy.should_retry(2, Some(StatusCode::SERVICE_UNAVAILABLE)));
        assert_eq!(policy.delay(2), Duration::from_millis(400));
        assert!(!policy.should_retry(0, Some(StatusCode::BAD_REQUEST)));
    }

    #[tokio::test]
    async fn reports_invalid_requests_before_transport() {
        let client = OutboundHttpClient::new();
        let spec = HttpRequestSpec::new(
            Method::GET,
            "http://[invalid"
                .parse()
                .unwrap_or_else(|_| Uri::from_static("http://localhost")),
        )
        .timeout(Duration::from_millis(1));
        let _ = client.execute(spec).await;
    }
}
