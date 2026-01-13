//! Testing utilities for Yaiko applications
//!
//! Provides helpers for writing integration tests.

use crate::{Request, Response, Router};
use hyper::{Body, Method, Uri};
use std::collections::HashMap;

/// Test client for making HTTP requests to a router
pub struct TestClient {
    router: Router,
}

impl TestClient {
    /// Create a new test client with the given router
    pub fn new(router: Router) -> Self {
        Self { router }
    }

    /// Make a GET request
    pub async fn get(&self, path: &str) -> TestResponse {
        self.request(Method::GET, path, None, HashMap::new()).await
    }

    /// Make a POST request with JSON body
    pub async fn post(&self, path: &str, body: impl Into<String>) -> TestResponse {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        self.request(Method::POST, path, Some(body.into()), headers).await
    }

    /// Make a PUT request with JSON body
    pub async fn put(&self, path: &str, body: impl Into<String>) -> TestResponse {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        self.request(Method::PUT, path, Some(body.into()), headers).await
    }

    /// Make a DELETE request
    pub async fn delete(&self, path: &str) -> TestResponse {
        self.request(Method::DELETE, path, None, HashMap::new()).await
    }

    /// Make a custom request
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<String>,
        headers: HashMap<String, String>,
    ) -> TestResponse {
        let uri: Uri = path.parse().expect("Invalid URI");
        
        let mut builder = hyper::Request::builder()
            .method(method.clone())
            .uri(uri);
        
        for (key, value) in &headers {
            builder = builder.header(key.as_str(), value.as_str());
        }
        
        let hyper_body = match body {
            Some(b) => Body::from(b),
            None => Body::empty(),
        };
        
        let hyper_req = builder.body(hyper_body).expect("Failed to build request");
        let req = match Request::from_hyper(hyper_req).await {
            Ok(r) => r,
            Err(e) => {
                return TestResponse {
                    status: 500,
                    headers: HashMap::new(),
                    body: format!("Failed to create request: {}", e),
                };
            }
        };
        
        match self.router.handle_request(req).await {
            Ok(response) => TestResponse::from_response(response).await,
            Err(e) => TestResponse {
                status: 500,
                headers: HashMap::new(),
                body: format!("Error: {}", e),
            },
        }
    }
}

/// Response from a test request
#[derive(Debug)]
pub struct TestResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl TestResponse {
    async fn from_response(response: Response) -> Self {
        let status = response.status.as_u16();
        let headers: HashMap<String, String> = response
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        let body_bytes = hyper::body::to_bytes(response.body)
            .await
            .unwrap_or_default();
        let body = String::from_utf8_lossy(&body_bytes).to_string();
        
        Self { status, headers, body }
    }

    /// Check if status is successful (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Parse body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.body)
    }

    /// Assert status code
    pub fn assert_status(&self, expected: u16) -> &Self {
        assert_eq!(self.status, expected, "Expected status {}, got {}", expected, self.status);
        self
    }

    /// Assert body contains text
    pub fn assert_body_contains(&self, text: &str) -> &Self {
        assert!(self.body.contains(text), "Body does not contain '{}': {}", text, self.body);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_works() {
        // Basic test to ensure TestClient compiles
        let router = Router::new();
        let _client = TestClient::new(router);
    }
}
