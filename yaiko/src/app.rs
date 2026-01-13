use crate::{Router, Request, Response, Handler, static_files::StaticFiles, template::TemplateEngine};
use async_trait::async_trait;
use std::sync::Arc;

pub struct App {
    router: Router,
    static_handler: Option<Arc<StaticFiles>>,
    template_engine: Option<Arc<TemplateEngine>>,
}

impl App {
    pub fn new() -> Self {
        App {
            router: Router::new(),
            static_handler: None,
            template_engine: None,
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
        self.template_engine = Some(Arc::new(engine));
        self
    }
}

#[async_trait]
impl Handler for App {
    async fn handle(&self, req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(static_handler) = &self.static_handler {
            if req.uri.path().starts_with("/static") {
                return static_handler.handle(req).await;
            }
        }

        self.router.handle_request(req).await
    }
}