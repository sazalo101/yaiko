use hyper::{Body, Method, Request as HyperRequest, Uri};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub headers: hyper::HeaderMap,
    pub body: Body,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub json_body: Option<Value>,
    pub form_data_cache: Option<HashMap<String, String>>,
    pub request_id: String,
    // Fields used by middleware
    pub user_id: Option<String>,
    pub user_roles: Vec<String>,
    pub session: Option<crate::session::SessionHandle>,
    pub extensions: hyper::http::Extensions,
    pub remote_addr: Option<SocketAddr>,
}

impl Request {
    pub async fn from_hyper(
        req: HyperRequest<Body>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::from_hyper_with_addr(req, None).await
    }

    pub async fn from_hyper_with_addr(
        req: HyperRequest<Body>,
        remote_addr: Option<SocketAddr>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (parts, body) = req.into_parts();
        let query = Self::parse_query(&parts.uri);

        Ok(Request {
            method: parts.method,
            uri: parts.uri,
            headers: parts.headers,
            body,
            params: HashMap::new(),
            query,
            json_body: None,
            form_data_cache: None,
            request_id: uuid::Uuid::new_v4().to_string(),
            user_id: None,
            user_roles: Vec::new(),
            session: None,
            extensions: parts.extensions,
            remote_addr,
        })
    }

    pub async fn json(&mut self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if self.json_body.is_none() {
            let body_bytes =
                hyper::body::to_bytes(std::mem::replace(&mut self.body, hyper::Body::empty()))
                    .await?;
            if !body_bytes.is_empty() {
                self.json_body = Some(serde_json::from_slice(&body_bytes)?);
            }
            // Repopulate the consumed body bytes cleanly for downstream execution!
            self.body = hyper::Body::from(body_bytes);
        }
        Ok(self.json_body.clone().unwrap_or(Value::Null))
    }

    pub async fn form_data(
        &mut self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync>> {
        if self.form_data_cache.is_none() {
            let body = std::mem::replace(&mut self.body, hyper::Body::empty());
            let body_bytes = hyper::body::to_bytes(body).await?;
            self.body = hyper::Body::from(body_bytes.clone());
            if !body_bytes.is_empty() {
                let raw = String::from_utf8_lossy(&body_bytes);
                let mut map = HashMap::new();
                for pair in raw.split('&') {
                    if pair.is_empty() {
                        continue;
                    }

                    let mut parts = pair.splitn(2, '=');
                    let key = decode_form_component(parts.next().unwrap_or_default());
                    let value = decode_form_component(parts.next().unwrap_or_default());
                    map.insert(key, value);
                }
                self.form_data_cache = Some(map);
            } else {
                self.form_data_cache = Some(HashMap::new());
            }
        }
        Ok(self.form_data_cache.clone().unwrap_or_default())
    }

    pub fn param(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }

    pub fn query_param(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    /// Shorthand to get a header value as a string
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    /// Check if the request Content-Type is JSON
    pub fn is_json(&self) -> bool {
        self.header("content-type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    /// Check if the request is an AJAX/XHR request
    pub fn is_ajax(&self) -> bool {
        self.header("x-requested-with")
            .map(|v| v.eq_ignore_ascii_case("xmlhttprequest"))
            .unwrap_or(false)
    }

    /// Get the raw body bytes without consuming the body stream
    pub async fn body_bytes(
        &mut self,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let body = std::mem::replace(&mut self.body, hyper::Body::empty());
        let body_bytes = hyper::body::to_bytes(body).await?;
        // Repopulate so downstream can still read it
        self.body = hyper::Body::from(body_bytes.clone());
        Ok(body_bytes.to_vec())
    }

    fn parse_query(uri: &Uri) -> HashMap<String, String> {
        let mut query = HashMap::new();
        if let Some(query_str) = uri.query() {
            for pair in query_str.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    query.insert(
                        percent_encoding::percent_decode_str(key)
                            .decode_utf8_lossy()
                            .to_string(),
                        percent_encoding::percent_decode_str(value)
                            .decode_utf8_lossy()
                            .to_string(),
                    );
                }
            }
        }
        query
    }
}

fn decode_form_component(value: &str) -> String {
    percent_encoding::percent_decode_str(&value.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn form_data_preserves_encoded_delimiters_and_body() {
        let req = hyper::Request::builder()
            .method(Method::POST)
            .uri("/submit")
            .body(Body::from("title=fish%26chips&note=a%3Db+c"))
            .unwrap();

        let mut req = Request::from_hyper(req).await.unwrap();
        let form = req.form_data().await.unwrap();

        assert_eq!(form.get("title"), Some(&"fish&chips".to_string()));
        assert_eq!(form.get("note"), Some(&"a=b c".to_string()));
        assert_eq!(
            String::from_utf8(req.body_bytes().await.unwrap()).unwrap(),
            "title=fish%26chips&note=a%3Db+c"
        );
    }
}
