use hyper::{Body, Request as HyperRequest, Method, Uri};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub uri: Uri,
    pub headers: hyper::HeaderMap,
    pub body: Body,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub json_body: Option<Value>,
    // Fields used by middleware
    pub user_id: Option<String>,
    pub user_roles: Vec<String>,
    pub session: Option<crate::session::Session>,
}

impl Request {
    pub async fn from_hyper(req: HyperRequest<Body>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
            user_id: None,
            user_roles: Vec::new(),
            session: None,
        })
    }

    pub async fn json(&mut self) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        if self.json_body.is_none() {
            let body_bytes = hyper::body::to_bytes(&mut self.body).await?;
            if !body_bytes.is_empty() {
                self.json_body = Some(serde_json::from_slice(&body_bytes)?);
            }
        }
        Ok(self.json_body.clone().unwrap_or(Value::Null))
    }

    pub fn param(&self, key: &str) -> Option<&String> {
        self.params.get(key)
    }

    pub fn query_param(&self, key: &str) -> Option<&String> {
        self.query.get(key)
    }

    fn parse_query(uri: &Uri) -> HashMap<String, String> {
        let mut query = HashMap::new();
        if let Some(query_str) = uri.query() {
            for pair in query_str.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    query.insert(
                        percent_encoding::percent_decode_str(key).decode_utf8_lossy().to_string(),
                        percent_encoding::percent_decode_str(value).decode_utf8_lossy().to_string(),
                    );
                }
            }
        }
        query
    }
}