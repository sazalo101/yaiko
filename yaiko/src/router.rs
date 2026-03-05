
use crate::{Request, Response, Handler, Middleware};
use crate::static_files::StaticFiles;
use async_trait::async_trait;
use hyper::Method;
use regex::Regex;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub struct Route {
    pub path: String,
    pub method: Method,
    pub regex: Regex,
    pub param_names: Vec<String>,
    pub handler: Arc<dyn Handler>,
}

// Implement Debug manually for Route
impl fmt::Debug for Route {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Route")
            .field("path", &self.path)
            .field("method", &self.method)
            .field("param_names", &self.param_names)
            .finish()
    }
}
impl Route {
    pub fn new(method: Method, path: &str, handler: Arc<dyn Handler>) -> Self {
        let mut normalized_path = path.trim_end_matches('/');
        if normalized_path.is_empty() {
            normalized_path = "/";
        }
        let (regex, param_names) = Self::path_to_regex(normalized_path);
        Route {
            path: normalized_path.to_string(),
            method,
            regex,
            param_names,
            handler,
        }
    }

    fn path_to_regex(path: &str) -> (Regex, Vec<String>) {
        let mut regex_str = String::new();
        let mut param_names = Vec::new();
        let mut chars = path.chars().peekable();

        regex_str.push('^');

        while let Some(ch) = chars.next() {
            match ch {
                ':' => {
                    let mut param_name = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch.is_alphanumeric() || next_ch == '_' {
                            param_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    param_names.push(param_name);
                    regex_str.push_str("([^/]+)");
                }
                '*' => {
                    regex_str.push_str("(.*)");
                }
                '.' | '+' | '?' | '^' | '$' | '{' | '}' | '[' | ']' | '|' | '(' | ')' | '\\' => {
                    regex_str.push('\\');
                    regex_str.push(ch);
                }
                _ => regex_str.push(ch),
            }
        }

        regex_str.push('$');
        (Regex::new(&regex_str).unwrap(), param_names)
    }

    pub fn matches_path(&self, path: &str) -> Option<HashMap<String, String>> {

        if let Some(captures) = self.regex.captures(path) {
            let mut params = HashMap::new();
            for (i, param_name) in self.param_names.iter().enumerate() {
                if let Some(capture) = captures.get(i + 1) {
                    params.insert(param_name.clone(), capture.as_str().to_string());
                }
            }
            Some(params)
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct Router {
    routes: Vec<Route>,
    middleware: Vec<Arc<dyn Middleware>>,
    pub static_handler: Option<Arc<StaticFiles>>,
    pub static_prefix: Option<String>,
}

impl Router {
    pub fn new() -> Self {
        Router {
            routes: Vec::new(),
            middleware: Vec::new(),
            static_handler: None,
            static_prefix: None,
        }
    }

    pub fn get<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::GET, path, Arc::new(handler)));
        self
    }

    pub fn post<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::POST, path, Arc::new(handler)));
        self
    }

    pub fn put<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::PUT, path, Arc::new(handler)));
        self
    }

    pub fn delete<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::DELETE, path, Arc::new(handler)));
        self
    }

    pub fn patch<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::PATCH, path, Arc::new(handler)));
        self
    }

    pub fn options<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::OPTIONS, path, Arc::new(handler)));
        self
    }

    pub fn head<H>(mut self, path: &str, handler: H) -> Self
    where
        H: Handler + 'static,
    {
        self.routes.push(Route::new(Method::HEAD, path, Arc::new(handler)));
        self
    }

    /// Mount static files from a directory at a URL prefix
    /// 
    /// # Example
    /// ```rust
    /// let router = Router::new()
    ///     .static_files("/static", "./public");
    /// ```
    pub fn static_files(mut self, prefix: &str, dir: &str) -> Self {
        self.static_handler = Some(Arc::new(StaticFiles::new(dir, prefix)));
        self.static_prefix = Some(prefix.to_string());
        self
    }

    pub fn use_middleware<M>(mut self, middleware: M) -> Self
    where
        M: Middleware + 'static,
    {
        self.middleware.push(Arc::new(middleware));
        self
    }

    /// Mount a sub-router at a given path prefix
    pub fn mount(mut self, path: &str, router: Router) -> Self {
        // We create a route that matches the prefix and anything after it
        let mut prefix = path.trim_end_matches('/').to_string();
        if prefix.is_empty() {
            prefix = "/".to_string();
        }
        
        let path_pattern = format!("{}/*", prefix);
        // We mount it for all common methods
        for method in [Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::PATCH, Method::OPTIONS] {
            self.routes.push(Route::new(method, &path_pattern, Arc::new(router.clone())));
        }
        self
    }

    pub async fn handle_request(&self, mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let mut original_path = req.uri.path().trim_end_matches('/').to_string();
        if original_path.is_empty() {
            original_path = "/".to_string();
        }
        let target_method = req.method.clone();
        
        // Track allowed methods for 405 Method Not Allowed / OPTIONS fallbacks
        let mut allowed_methods = Vec::new();
        let mut found_match = false;
        
        // Find matching route
        for route in &self.routes {
            if let Some(params) = route.matches_path(&original_path) {
                found_match = true;
                allowed_methods.push(route.method.as_str().to_string());
                
                // Allow exact method match OR auto-resolve HEAD requests to GET routes
                if route.method == target_method || (target_method == Method::HEAD && route.method == Method::GET) {
                    req.params = params;
                
                // If it's a mount route (ends with /*), we need to strip the prefix for the sub-router
                if route.path.ends_with("/*") {
                    let prefix = route.path.trim_end_matches("/*");
                    let mut new_path = original_path.strip_prefix(prefix).unwrap_or(&original_path).to_string();
                    if new_path.is_empty() {
                        new_path = "/".to_string();
                    }
                    
                    // Replace the URI path but keep query params
                    let mut parts = req.uri.clone().into_parts();
                    let path_and_query = match parts.path_and_query {
                        Some(pq) => {
                            if let Some(query) = pq.query() {
                                hyper::http::uri::PathAndQuery::try_from(format!("{}?{}", new_path, query).as_str()).ok()
                            } else {
                                hyper::http::uri::PathAndQuery::try_from(new_path.as_str()).ok()
                            }
                        }
                        None => hyper::http::uri::PathAndQuery::try_from(new_path.as_str()).ok(),
                    };
                    parts.path_and_query = path_and_query;
                    if let Ok(new_uri) = hyper::Uri::from_parts(parts) {
                        req.uri = new_uri;
                    }
                }
                
                // Apply middleware chain
                let handler = route.handler.clone();
                let final_handler = self.middleware.iter().rev().fold(handler, |next, middleware| {
                    let middleware = middleware.clone();
                    Arc::new(MiddlewareHandler { middleware, next })
                });
                
                return final_handler.handle(req).await;
                }
            }
        }

        // Return a global OPTIONS preflight success safely if ANY route path matched returning the Allow/CORS options
        if found_match && target_method == Method::OPTIONS {
            allowed_methods.sort();
            allowed_methods.dedup();
            let allow_header = allowed_methods.join(", ");
            return Ok(Response::new()
                .status(hyper::StatusCode::OK)
                .header("Allow", &allow_header)
                .header("Access-Control-Allow-Methods", &allow_header));
        }

        // Return a 405 Method Not Allowed error returning the Allow list safely preventing false 404 logic
        if found_match {
            allowed_methods.sort();
            allowed_methods.dedup();
            return Ok(Response::new()
                .status(hyper::StatusCode::METHOD_NOT_ALLOWED)
                .header("Allow", &allowed_methods.join(", "))
                .text("Method Not Allowed"));
        }

        // Try static files if configured
        if let (Some(ref handler), Some(ref prefix)) = (&self.static_handler, &self.static_prefix) {
            let path = req.uri.path();
            if path.starts_with(prefix) {
                return handler.handle(req).await;
            }
        }

        // No route found
        Ok(Response::new()
            .status(hyper::StatusCode::NOT_FOUND)
            .text("Not Found"))
    }
}

// Helper struct to chain middleware
struct MiddlewareHandler {
    middleware: Arc<dyn Middleware>,
    next: Arc<dyn Handler>,
}

#[async_trait]
impl Handler for MiddlewareHandler {
    async fn handle(&self, req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        self.middleware.handle(req, self.next.clone()).await
    }
}

#[async_trait]
impl Handler for Router {
    async fn handle(&self, req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        self.handle_request(req).await
    }
}