use crate::{Handler, Request, Response};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Middleware: Send + Sync + 'static {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>>;
}

// Logger middleware
pub struct Logger;

#[async_trait]
impl Middleware for Logger {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();
        let method = req.method.clone();
        let uri = req.uri.clone();
        let request_id = req.request_id.clone();

        let response = next.handle(req).await?;

        let duration = start.elapsed();
        println!(
            "[{}] {} {} {} - {:?}",
            request_id, method, uri, response.status, duration
        );

        Ok(response)
    }
}

// CORS middleware
pub struct Cors {
    allow_origin: String,
    allow_methods: String,
    allow_headers: String,
    max_age: Option<u32>,
}

impl Default for Cors {
    fn default() -> Self {
        Self::new()
    }
}

impl Cors {
    pub fn new() -> Self {
        Cors {
            allow_origin: "*".to_string(),
            allow_methods: "GET, POST, PUT, DELETE, OPTIONS".to_string(),
            allow_headers: "Content-Type, Authorization".to_string(),
            max_age: None,
        }
    }

    pub fn allow_origin(mut self, origin: &str) -> Self {
        self.allow_origin = origin.to_string();
        self
    }

    pub fn allow_methods(mut self, methods: &str) -> Self {
        self.allow_methods = methods.to_string();
        self
    }

    pub fn allow_headers(mut self, headers: &str) -> Self {
        self.allow_headers = headers.to_string();
        self
    }

    pub fn max_age(mut self, age: u32) -> Self {
        self.max_age = Some(age);
        self
    }
}

#[async_trait]
impl Middleware for Cors {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if req.method == hyper::Method::OPTIONS {
            let mut res = Response::new()
                .header("Access-Control-Allow-Origin", &self.allow_origin)
                .header("Access-Control-Allow-Methods", &self.allow_methods)
                .header("Access-Control-Allow-Headers", &self.allow_headers)
                .status(hyper::StatusCode::OK);

            if let Some(age) = self.max_age {
                res = res.header("Access-Control-Max-Age", &age.to_string());
            }
            return Ok(res);
        }

        let mut response = next.handle(req).await?;
        response.headers.insert(
            "Access-Control-Allow-Origin".to_string(),
            self.allow_origin.clone(),
        );
        Ok(response)
    }
}

// Body Size Limit middleware
pub struct BodySizeLimit {
    max_bytes: u64,
}

impl BodySizeLimit {
    pub fn new(max_bytes: u64) -> Self {
        BodySizeLimit { max_bytes }
    }
}

#[async_trait]
impl Middleware for BodySizeLimit {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(length_str) = req.headers.get(hyper::header::CONTENT_LENGTH) {
            if let Ok(length) = length_str.to_str().unwrap_or("0").parse::<u64>() {
                if length > self.max_bytes {
                    return Ok(Response::new()
                        .status(hyper::StatusCode::PAYLOAD_TOO_LARGE)
                        .text("Payload Too Large"));
                }
            }
        }
        next.handle(req).await
    }
}

// Timeout middleware
pub struct Timeout {
    duration: std::time::Duration,
}

impl Timeout {
    pub fn new(duration: std::time::Duration) -> Self {
        Timeout { duration }
    }
}

#[async_trait]
impl Middleware for Timeout {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        match tokio::time::timeout(self.duration, next.handle(req)).await {
            Ok(result) => result,
            Err(_) => Ok(Response::new()
                .status(hyper::StatusCode::GATEWAY_TIMEOUT)
                .text("Request Timeout")),
        }
    }
}

// Request ID middleware
pub struct RequestId;

#[async_trait]
impl Middleware for RequestId {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let req_id = req.request_id.clone();
        let mut response = next.handle(req).await?;
        response.headers.insert("X-Request-ID".to_string(), req_id);
        Ok(response)
    }
}

// ETag cache validation middleware
pub struct ETag;

#[async_trait]
impl Middleware for ETag {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let if_none_match = req
            .headers
            .get(hyper::header::IF_NONE_MATCH)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let mut response = next.handle(req).await?;

        // Only generate ETag for successful GET requests with a text/html or application/json body
        if response.status.is_success() {
            let mut final_res = Response::new().status(response.status);
            for (k, v) in response.headers.drain() {
                final_res.headers.insert(k, v);
            }

            // Read body to compute hash
            if let Ok(body_bytes) = hyper::body::to_bytes(response.body).await {
                if !body_bytes.is_empty() {
                    let hash_str = sha1_smol::Sha1::from(&body_bytes).digest().to_string();
                    let hash = format!("\"{}\"", hash_str);

                    if let Some(client_hash) = if_none_match {
                        if client_hash == hash {
                            // Client has the same version mapped, reply 304 without payload
                            return Ok(final_res
                                .status(hyper::StatusCode::NOT_MODIFIED)
                                .header("ETag", &hash));
                        }
                    }

                    // Inject the buffered bytes as the new body
                    final_res.headers.insert("ETag".to_string(), hash);
                    final_res.body = hyper::Body::from(body_bytes);
                    return Ok(final_res);
                }

                // If it reached here but didn't match ETag, we need to rebuild the original response
                final_res.body = hyper::Body::from(body_bytes);
                return Ok(final_res);
            }
            return Ok(final_res);
        }

        // Return untouched if rules don't match (e.g. wasn't a success)
        Ok(response)
    }
}
