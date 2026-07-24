use yaiko_core::{
    App, Router, Server, BoxError, 
    LoggingMiddleware, SecurityHeaders, RateLimiter, CsrfProtection,
    SessionMiddleware, MemorySessionStore,
    WebSocketManager, init_tracing, tracing,
};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

mod auth_routes;
mod chat_ws;
mod file_routes;

use chat_ws::AppState;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    init_tracing();
    tracing::info!("Starting TeamPulse...");

    // 1. Database Setup
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://teampulse.db".into());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    // 2. Application State
    let state = Arc::new(AppState {
        ws_manager: Arc::new(WebSocketManager::new()),
        db_pool: pool.clone(),
    });

    // 3. Setup Routes
    let p = pool.clone();
    let s = state.clone();
    
    let api = Router::new()
        .post("/auth/register", {
            let p = p.clone();
            move |req| auth_routes::register(req, p.clone())
        })
        .post("/auth/login", {
            let p = p.clone();
            move |req| auth_routes::login(req, p.clone())
        })
        .get("/auth/me", auth_routes::me)
        .post("/auth/logout", auth_routes::logout)
        .post("/upload", file_routes::upload_file)
        .get("/history/:room", {
            let s = s.clone();
            move |req| chat_ws::get_history(req, s.clone())
        });

    let store = Arc::new(MemorySessionStore::new());

    let router = Router::new()
        .mount("/api", api)
        .get("/ws", {
            let s = s.clone();
            move |req| chat_ws::chat_ws_handler(req, s.clone())
        })
        .static_files("/uploads", "./public/uploads")
        .static_files("/", "./public")
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new())
        .use_middleware(SessionMiddleware::new(store));

    // 4. Build Application
    let app = App::new().router(router);


    // 5. Start Server
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    tracing::info!("TeamPulse running at http://{}", addr);
    server.run().await?;

    Ok(())
}
