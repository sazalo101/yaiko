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
            if req.uri.path().starts_with(self.router.static_prefix.as_deref().unwrap_or("/static")) {
                return static_handler.handle(req).await;
            }
        }

        let not_found_handler = self.not_found_handler.clone();

        // Wrap the core routing logic inside catch_unwind
        let result = AssertUnwindSafe(async {
            self.router.handle_request(req).await
        }).catch_unwind().await;

        match result {
            Ok(Ok(res)) => {
                // If the response is exactly a 404, we have a chance to override it!
                if res.status == hyper::StatusCode::NOT_FOUND {
                    // Because `request` is consumed natively, ideally we'd pass it in if we had it, but we threw it down. 
                    // To do it correctly, we should just let `not_found_handler` return a Response. We'll mint a dummy request 
                    // or let `handle_request` take a reference if possible. Actually, passing a dummy `Request` to `not_found` works since it only needs URL.
                    if let Some(handler) = not_found_handler {
                        // For a simple 404 intercept, we execute the handler yielding its Response.
                        let mut empty_req = Request::from_hyper(hyper::Request::new(hyper::Body::empty())).await.unwrap();
                        return Ok(handler(empty_req));
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