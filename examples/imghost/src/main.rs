use yaiko_core::{App, Router, Server, Request, Response, Settings, BoxError, tracing};
use yaiko_core::LoggingMiddleware;
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;

mod models;
mod routes;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    dotenvy::dotenv().ok();
    let settings = Settings::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting ImgHost...");

    // 1. Database
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://imghost.db".into());
    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .connect(&db_url)
        .await?;

    // Enable WAL mode: allows concurrent reads while a write is in progress
    sqlx::query("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA cache_size=10000;")
        .execute(&pool)
        .await?;

    tracing::info!("Connected to database");

    // 2. API Routes
    let p = pool.clone();
    let api = Router::new()
        .post("/upload", {
            let p = p.clone();
            move |req| routes::upload::upload(req, p.clone())
        })
        .get("/images/:id", {
            let p = p.clone();
            move |req| routes::image::get_image(req, p.clone())
        })
        .delete("/images/:id", {
            let p = p.clone();
            move |req| routes::image::delete_image(req, p.clone())
        });


    // 3. Main Router — viewer page must come before static "/" catch-all
    let router = Router::new()
        .mount("/api", api)
        .get("/i/:id", viewer_handler)
        .static_files("/uploads", "./public/uploads")
        .static_files("/", "./public")
        .use_middleware(LoggingMiddleware::new());

    // 4. Build App
    let app = App::new().router(router);

    // 5. Start Server
    let addr: SocketAddr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or_else(|_| settings.server.host),
        std::env::var("PORT").unwrap_or_else(|_| settings.server.port.to_string())
    ).parse()?;

    let server = Server::new(app, addr);
    tracing::info!("🖼️  ImgHost running at http://{}", addr);
    server.run().await?;

    Ok(())
}

/// Serves the viewer HTML page for any /i/:id route.
/// The page's JavaScript fetches the image metadata from /api/images/:id.
async fn viewer_handler(_req: Request) -> Result<Response, BoxError> {
    let html = include_str!("../public/viewer.html");
    Ok(Response::new()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(html.into()))
}
