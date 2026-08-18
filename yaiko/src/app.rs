use crate::{
    static_files::StaticFiles, template::TemplateEngine, Handler, Request, Response, Router,
};
use async_trait::async_trait;
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

pub type ErrorHandlerFn =
    Arc<dyn Fn(Box<dyn std::error::Error + Send + Sync>) -> Response + Send + Sync>;
pub type NotFoundHandlerFn = Arc<dyn Fn(Request) -> Response + Send + Sync>;

pub struct App {
    router: Router,
    static_handler: Option<Arc<StaticFiles>>,
    pub template_engine: Option<Arc<tokio::sync::RwLock<TemplateEngine>>>,
    error_handler: Option<ErrorHandlerFn>,
    not_found_handler: Option<NotFoundHandlerFn>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
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
    async fn handle(
        &self,
        req: Request,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
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
        let result = AssertUnwindSafe(async { self.router.handle_request(req).await })
            .catch_unwind()
            .await;

        match result {
            Ok(Ok(res)) => {
                // If the response is exactly a 404, check for automatic SEO fallbacks or custom 404 handler
                if res.status == hyper::StatusCode::NOT_FOUND {
                    if not_found_req_uri.path() == "/robots.txt" {
                        let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| {
                            if let Some(host) = not_found_req_headers
                                .get("host")
                                .and_then(|h| h.to_str().ok())
                            {
                                format!("https://{}", host)
                            } else {
                                "http://localhost:3000".to_string()
                            }
                        });
                        let content = format!(
                            "User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n",
                            site_url.trim_end_matches('/')
                        );
                        return Ok(Response::new()
                            .header("Content-Type", "text/plain; charset=utf-8")
                            .text(&content));
                    }

                    if not_found_req_uri.path() == "/sitemap.xml" {
                        let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| {
                            if let Some(host) = not_found_req_headers
                                .get("host")
                                .and_then(|h| h.to_str().ok())
                            {
                                format!("https://{}", host)
                            } else {
                                "http://localhost:3000".to_string()
                            }
                        });
                        let base = site_url.trim_end_matches('/');
                        let content = format!(
                            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>{}/</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
"#,
                            base
                        );
                        return Ok(Response::new()
                            .header("Content-Type", "application/xml; charset=utf-8")
                            .body(hyper::Body::from(content)));
                    }

                    if let Some(handler) = not_found_handler {
                        // Build a request with the actual URI/method/headers that 404'd
                        let mut builder = hyper::Request::builder()
                            .method(not_found_req_method)
                            .uri(not_found_req_uri);
                        for (key, value) in &not_found_req_headers {
                            builder = builder.header(key, value);
                        }
                        let hyper_req = builder.body(hyper::Body::empty()).unwrap();
                        let actual_req = Request::from_hyper_with_addr(hyper_req, None)
                            .await
                            .unwrap();
                        return Ok(handler(actual_req));
                    }
                }
                Ok(res)
            }
            Ok(Err(e)) => {
                // Handle natural framework Errors
                if let Some(handler) = &self.error_handler {
                    Ok(handler(e))
                } else {
                    Ok(Response::new()
                        .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                        .text("Internal Server Error"))
                }
            }
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

    #[tokio::test]
    async fn app_auto_seo_serves_robots_and_sitemap() {
        let app = App::new();

        let req_robots = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/robots.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        let res_robots = app.handle(req_robots).await.unwrap();
        assert_eq!(res_robots.status, hyper::StatusCode::OK);

        let req_sitemap = Request::from_hyper(
            hyper::Request::builder()
                .method("GET")
                .uri("/sitemap.xml")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        let res_sitemap = app.handle(req_sitemap).await.unwrap();
        assert_eq!(res_sitemap.status, hyper::StatusCode::OK);
    }
}
