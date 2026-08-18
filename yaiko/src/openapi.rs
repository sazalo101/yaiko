//! OpenAPI 3.0 document generation for Yaiko routes.

use crate::{Handler, Request, Response};
use async_trait::async_trait;
use hyper::Method;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiDocument {
    pub openapi: String,
    pub info: OpenApiInfo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<OpenApiServer>>,
    pub paths: BTreeMap<String, OpenApiPathItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiServer {
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct OpenApiPathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<OpenApiOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiOperation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        rename = "operationId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub operation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub responses: BTreeMap<String, OpenApiResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenApiResponse {
    pub description: String,
}

impl OpenApiOperation {
    pub fn new(response_status: u16, response_description: impl Into<String>) -> Self {
        let mut responses = BTreeMap::new();
        responses.insert(
            response_status.to_string(),
            OpenApiResponse {
                description: response_description.into(),
            },
        );
        Self {
            summary: None,
            description: None,
            operation_id: None,
            tags: Vec::new(),
            responses,
        }
    }

    pub fn summary(mut self, value: impl Into<String>) -> Self {
        self.summary = Some(value.into());
        self
    }

    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    pub fn operation_id(mut self, value: impl Into<String>) -> Self {
        self.operation_id = Some(value.into());
        self
    }

    pub fn tag(mut self, value: impl Into<String>) -> Self {
        self.tags.push(value.into());
        self
    }
}

impl OpenApiDocument {
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: OpenApiInfo {
                title: title.into(),
                version: version.into(),
                description: None,
            },
            servers: None,
            paths: BTreeMap::new(),
        }
    }

    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.info.description = Some(value.into());
        self
    }

    pub fn server(mut self, url: impl Into<String>, description: Option<String>) -> Self {
        self.servers
            .get_or_insert_with(Vec::new)
            .push(OpenApiServer {
                url: url.into(),
                description,
            });
        self
    }

    pub fn route(mut self, method: Method, path: &str, operation: OpenApiOperation) -> Self {
        let path = normalize_path(path);
        let item = self.paths.entry(path).or_default();
        match method {
            Method::GET => item.get = Some(operation),
            Method::POST => item.post = Some(operation),
            Method::PUT => item.put = Some(operation),
            Method::DELETE => item.delete = Some(operation),
            Method::PATCH => item.patch = Some(operation),
            Method::OPTIONS => item.options = Some(operation),
            Method::HEAD => item.head = Some(operation),
            _ => {}
        }
        self
    }

    pub fn json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn handler(self: Arc<Self>) -> OpenApiHandler {
        OpenApiHandler { document: self }
    }
}

fn normalize_path(path: &str) -> String {
    let mut result = String::new();
    let mut chars = path.trim_end_matches('/').chars().peekable();
    if !path.starts_with('/') {
        result.push('/');
    }
    while let Some(ch) = chars.next() {
        if ch == ':' {
            let mut name = String::new();
            while let Some(next) = chars.peek().copied() {
                if next.is_alphanumeric() || next == '_' {
                    name.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            result.push('{');
            result.push_str(&name);
            result.push('}');
        } else if ch == '*' {
            result.push_str("{path}");
        } else {
            result.push(ch);
        }
    }
    if result.is_empty() {
        "/".to_string()
    } else {
        result
    }
}

pub struct OpenApiHandler {
    document: Arc<OpenApiDocument>,
}

#[async_trait]
impl Handler for OpenApiHandler {
    async fn handle(
        &self,
        _req: Request,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Response::new().json(self.document.as_ref())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Method;

    #[test]
    fn builder_serializes_framework_paths_as_openapi_paths() {
        let document = OpenApiDocument::new("Demo", "1.0.0")
            .description("Demo API")
            .server("https://example.test", None)
            .route(
                Method::GET,
                "/users/:id",
                OpenApiOperation::new(200, "User returned")
                    .summary("Get a user")
                    .operation_id("getUser")
                    .tag("users"),
            );
        let value: serde_json::Value = serde_json::from_str(&document.json().unwrap()).unwrap();
        assert_eq!(value["openapi"], "3.0.3");
        assert_eq!(
            value["paths"]["/users/{id}"]["get"]["operationId"],
            "getUser"
        );
        assert_eq!(
            value["paths"]["/users/{id}"]["get"]["responses"]["200"]["description"],
            "User returned"
        );
    }

    #[test]
    fn empty_and_relative_paths_are_normalized() {
        let document = OpenApiDocument::new("Demo", "1.0.0")
            .route(Method::GET, "", OpenApiOperation::new(200, "OK"))
            .route(
                Method::POST,
                "admin/*",
                OpenApiOperation::new(201, "Created"),
            );
        assert!(document.paths.contains_key("/"));
        assert!(document.paths.contains_key("/admin/{path}"));
    }
}
