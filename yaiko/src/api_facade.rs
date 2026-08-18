//! Typed API route and response facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiError {
    InvalidPath,
    InvalidMethod,
    PayloadTooLarge,
    InvalidStatus,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRoute {
    pub method: ApiMethod,
    pub path: String,
    pub name: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}
#[derive(Debug, Clone)]
pub struct ApiFacade {
    max_body: usize,
    routes: Vec<ApiRoute>,
}
impl ApiFacade {
    pub fn new(max_body: usize) -> Self {
        Self {
            max_body,
            routes: Vec::new(),
        }
    }
    pub fn route(
        mut self,
        method: ApiMethod,
        path: impl Into<String>,
        name: impl Into<String>,
    ) -> Result<Self, ApiError> {
        let path = path.into();
        let name = name.into();
        if !path.starts_with('/')
            || path.contains("..")
            || path.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(ApiError::InvalidPath);
        }
        if name.is_empty() || name.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(ApiError::InvalidMethod);
        }
        if self
            .routes
            .iter()
            .any(|r| r.path == path && r.method == method)
        {
            return Err(ApiError::InvalidMethod);
        }
        self.routes.push(ApiRoute { method, path, name });
        self.routes
            .sort_by(|a, b| a.path.cmp(&b.path).then(a.name.cmp(&b.name)));
        Ok(self)
    }
    pub fn response(
        &self,
        status: u16,
        content_type: impl Into<String>,
        body: impl Into<Vec<u8>>,
    ) -> Result<ApiResponse, ApiError> {
        if !(100..=599).contains(&status) {
            return Err(ApiError::InvalidStatus);
        }
        let body = body.into();
        if body.len() > self.max_body {
            return Err(ApiError::PayloadTooLarge);
        }
        Ok(ApiResponse {
            status,
            content_type: content_type.into(),
            body,
        })
    }
    pub fn routes(&self) -> &[ApiRoute] {
        &self.routes
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registers_sorted_routes_and_builds_responses() {
        let a = ApiFacade::new(8)
            .route(ApiMethod::Post, "/media", "create")
            .unwrap()
            .route(ApiMethod::Get, "/media", "list")
            .unwrap();
        assert_eq!(a.routes()[0].name, "create");
        assert_eq!(
            a.response(201, "application/json", b"ok".to_vec())
                .unwrap()
                .status,
            201
        )
    }
    #[test]
    fn validates_paths_status_and_body_bounds() {
        let a = ApiFacade::new(1);
        assert!(a.clone().route(ApiMethod::Get, "../x", "bad").is_err());
        assert!(a.response(700, "text/plain", Vec::new()).is_err());
        assert!(a.response(200, "text/plain", b"xx".to_vec()).is_err())
    }
}
