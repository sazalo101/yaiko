use crate::{Router, Request, Response, Handler, static_files::StaticFiles, template::TemplateEngine};
use async_trait::async_trait;
use std::sync::Arc;
use std::panic::AssertUnwindSafe;
use futures::FutureExt;

pub type ErrorHandlerFn = Arc<dyn Fn(Box<dyn std::error::Error + Send + Sync>) -> Response + Send + Sync>;
pub type NotFoundHandlerFn = Arc<dyn Fn(Request) -> Response + Send + Sync>;

pub struct App {
    router: Router,
    static_handler: Option<Arc<StaticFiles>>,
    pub template_engine: Option<Arc<tokio::sync::RwLock<TemplateEngine>>>,
    error_handler: Option<ErrorHandlerFn>,
    not_found_handler: Option<NotFoundHandlerFn>,
}

impl App {
    pub fn new() -> Self {
        App {
            router: Router::new(),
            static_handler: None,
            template_engine: None,
            error_handler: None,
            not_found_handler: None,
        }
    }

    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    pub fn static_files(mut self, dir: &str, prefix: &str) -> Self {
        self.static_handler = Some(Arc::new(StaticFiles::new(dir, prefix)));
        self
    }

    pub fn templates(mut self, engine: TemplateEngine) -> Self {
        self.template_engine = Some(Arc::new(tokio::sync::RwLock::new(engine)));
        self
    }

    pub fn error_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(Box<dyn std::error::Error + Send + Sync>) -> Response + Send + Sync + 'static,
    {
        self.error_handler = Some(Arc::new(handler));
        self
    }

    pub fn not_found_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(Request) -> Response + Send + Sync + 'static,
    {
        self.not_found_handler = Some(Arc::new(handler));
        self
    }
}

#[async_trait]
impl Handler for App {
    async fn handle(&self, req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(static_handler) = &self.static_handler {
            if static_handler.matches(req.uri.path()) {
                return static_handler.handle(req).await;
            }
        }

        let not_found_handler = self.not_found_handler.clone();
        
        // Capture request info before routing consumes it (for potential 404 handler)
        let not_found_req_uri = req.uri.clone();
        let not_found_req_method = req.method.clone();
        let not_found_req_headers = req.headers.clone();

        // Wrap the core routing logic inside catch_unwind
        let result = AssertUnwindSafe(async {
            self.router.handle_request(req).await
        }).catch_unwind().await;

        match result {
            Ok(Ok(res)) => {
                // If the response is exactly a 404, we have a chance to override it!
                if res.status == hyper::StatusCode::NOT_FOUND {
                    if let Some(handler) = not_found_handler {
                        // Build a request with the actual URI/method/headers that 404'd
                        let mut builder = hyper::Request::builder()
                            .method(not_found_req_method)
                            .uri(not_found_req_uri);
                        for (key, value) in &not_found_req_headers {
                            builder = builder.header(key, value);
                        }
                        let hyper_req = builder.body(hyper::Body::empty()).unwrap();
                        let actual_req = Request::from_hyper_with_addr(hyper_req, None).await.unwrap();
                        return Ok(handler(actual_req));
                    }
                }
                Ok(res)
            },
            Ok(Err(e)) => {
                // Handle natural framework Errors
                if let Some(handler) = &self.error_handler {
                    Ok(handler(e))
                } else {
                    Ok(Response::new()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .text("Internal Server Error"))
                }
            },
            Err(_panic_err) => {
                // Handle unwound thread Panics efficiently
                tracing::error!("A handler thread panicked during execution!");
                if let Some(handler) = &self.error_handler {
                    Ok(handler("Internal thread panic".into()))
                } else {
                    Ok(Response::new()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .text("500 Internal Server Error (Panic Recovery)"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Body;
    use tempfile::tempdir;

    #[tokio::test]
    async fn app_static_files_honors_custom_prefix() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), "hi").unwrap();

        let app = App::new().static_files(dir.path().to_str().unwrap(), "/assets");
        let req = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/assets/hello.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        let response = app.handle(req).await.unwrap();
        assert_eq!(response.status, hyper::StatusCode::OK);
    }
}
