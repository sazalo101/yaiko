use yaiko_core::{
    App, Router, Server, Request, Response, 
    middleware::{Logger, Cors}, 
    auth::JwtAuth,
    session::{SessionMiddleware, MemorySessionStore},
    compression::CompressionMiddleware,
    template::TemplateEngine,
    json, StatusCode
};
use std::net::SocketAddr;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize components
    let jwt_auth = Arc::new(JwtAuth::new("your-secret-key"));
    let session_store = Arc::new(MemorySessionStore::new());
    let mut template_engine = TemplateEngine::new();
    
    // Register templates
    template_engine.register_template_file("index", "templates/index.hbs")?;
    
    // Create router with routes
    let router = Router::new()
        .get("/", home_handler)
        .get("/users/:id", get_user_handler)
        .post("/users", create_user_handler)
        .get("/health", health_handler)
        // Add middleware to router
        .use_middleware(Logger)
        .use_middleware(Cors::new().allow_origin("*"))
        .use_middleware(CompressionMiddleware::new())
        .use_middleware(SessionMiddleware::new(session_store));

    // Create app with static files and templates
    let app = App::new()
        .router(router)
        .static_files("./public", "/static")
        .templates(template_engine);

    // Create and run server
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    println!("Server starting on http://{}", addr);
    server.run().await?;
    
    Ok(())
}

// Handler functions
async fn home_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new()
        .html("<h1>Welcome to RustNext!</h1>")
        .status(StatusCode::OK))
}

async fn get_user_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let user_id = req.param("id").map(|s| s.as_str()).unwrap_or("unknown");
    
    let user_data = json!({
        "id": user_id,
        "name": "John Doe",
        "email": "john@example.com"
    });
    
    Ok(Response::new().json(&user_data)?)
}

async fn create_user_handler(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body = req.json().await?;
    
    // Process user creation logic here
    println!("Creating user: {:?}", body);
    
    let response_data = json!({
        "message": "User created successfully",
        "user": body
    });
    
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&response_data)?)
}

async fn health_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let health_data = json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    
    Ok(Response::new().json(&health_data)?)
}