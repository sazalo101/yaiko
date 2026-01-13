use yaiko_core::{App, Router, Server, Request, Response, StatusCode, json, Settings, tracing};
use yaiko_core::middleware::{Logger, Cors};
use yaiko_core::security::{SecurityHeaders, RateLimiter};
use std::net::SocketAddr;

mod controllers;
mod models;
mod middleware;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv::dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    
    tracing::info!("Starting Yaiko application...");
    
    let router = Router::new()
        .get("/", home_handler)
        .get("/health", health_handler)
        .get("/api/users", controllers::users::list)
        .get("/api/users/:id", controllers::users::get)
        .post("/api/users", controllers::users::create)
        .get("/robots.txt", robots_handler)
        .get("/sitemap.xml", sitemap_handler)
        .use_middleware(Logger)
        .use_middleware(Cors::new().allow_origin("*"));
    
    let app = App::new()
        .router(router)
        .static_files("./public", "/static");
    
    let addr: SocketAddr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        std::env::var("PORT").unwrap_or_else(|_| "3000".to_string())
    ).parse()?;
    
    let server = Server::new(app, addr);
    tracing::info!("🚀 Server running on http://{}", addr);
    server.run().await?;
    
    Ok(())
}

async fn home_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new()
        .html(include_str!("../templates/index.html"))
        .status(StatusCode::OK))
}

async fn health_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new()
        .json(&json!({
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))?)
}

async fn robots_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new()
        .body("User-agent: *\nAllow: /\nSitemap: /sitemap.xml\n")
        .header("Content-Type", "text/plain")
        .status(StatusCode::OK))
}

async fn sitemap_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let host = std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let content = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
<url><loc>{}/</loc><changefreq>weekly</changefreq><priority>1.0</priority></url>
</urlset>"#, host);
    Ok(Response::new()
        .body(&content)
        .header("Content-Type", "application/xml")
        .status(StatusCode::OK))
}
