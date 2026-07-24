//! Yaiko Tutorials Test App
//! This file exercises every major framework feature to verify tutorial code works.

use yaiko_core::{
    App, Router, Server, Request, Response, StatusCode, Body,
    json, Value,
    LoggingMiddleware, SecurityHeaders, RateLimiter, CsrfProtection,
    CompressionMiddleware, HealthCheck,
    init_tracing, tracing,
    Session, SessionMiddleware, MemorySessionStore, SessionStore,
    JwtAuth, AuthMiddleware,
    hash_password, verify_password,
    TestClient, TestResponse,
    JobQueue, Job,
    WebSocketManager, is_websocket_upgrade, websocket::handle_websocket_upgrade,
    validation::{Validator, Required, MinLength, MaxLength, Email},
    Middleware, Handler,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 1: Hello World
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn hello_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().text("Hello, Yaiko!"))
}

async fn hello_json_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().json(&json!({
        "message": "Hello, Yaiko!",
        "version": "0.1.0"
    }))?)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 2: REST API with CRUD
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    id: u32,
    title: String,
    completed: bool,
}

async fn list_todos(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let todos = vec![
        Todo { id: 1, title: "Learn Yaiko".into(), completed: false },
        Todo { id: 2, title: "Build an app".into(), completed: false },
    ];
    Ok(Response::new().json(&todos)?)
}

async fn get_todo(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    Ok(Response::new().json(&json!({
        "id": id,
        "title": "Learn Yaiko",
        "completed": false,
    }))?)
}

async fn create_todo(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body = req.json().await?;
    let title = body["title"].as_str().unwrap_or("Untitled");

    // Validate input
    let mut data = HashMap::new();
    data.insert("title".to_string(), title.to_string());

    let validator = Validator::new()
        .add_rule("title", Required)
        .add_rule("title", MinLength(2))
        .add_rule("title", MaxLength(100));

    if let Err(errors) = validator.validate(&data) {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({ "errors": errors }))?);
    }

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({
            "id": 3,
            "title": title,
            "completed": false,
        }))?)
}

async fn update_todo(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    let body = req.json().await?;
    Ok(Response::new().json(&json!({
        "id": id,
        "title": body["title"],
        "completed": body["completed"],
    }))?)
}

async fn patch_todo(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    let body = req.json().await?;
    Ok(Response::new().json(&json!({
        "id": id,
        "patched_fields": body,
    }))?)
}

async fn delete_todo(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let _id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    Ok(Response::no_content())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 3: Custom Middleware
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

struct TimingMiddleware;

#[async_trait]
impl Middleware for TimingMiddleware {
    async fn handle(
        &self,
        req: Request,
        next: Arc<dyn Handler>,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let start = std::time::Instant::now();
        let response = next.handle(req).await?;
        let duration = start.elapsed();
        tracing::info!(duration_ms = %duration.as_millis(), "Request completed");
        Ok(response.header("X-Response-Time", &format!("{}ms", duration.as_millis())))
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 4: Auth — Login + Protected Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn login_handler(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body = req.json().await?;
    let email = body["email"].as_str().unwrap_or("");
    let password = body["password"].as_str().unwrap_or("");

    // In real apps, look up user from DB
    let stored_hash = hash_password("secret123").unwrap();

    if email == "user@example.com" && verify_password(password, &stored_hash).unwrap_or(false) {
        let auth = JwtAuth::new("my-jwt-secret");
        let token = auth.generate_token("user-1", vec![])?;
        Ok(Response::new().json(&json!({
            "token": token,
            "user": { "id": "user-1", "email": email }
        }))?)
    } else {
        Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({ "error": "Invalid credentials" }))?)
    }
}

async fn profile_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    // user_id is set by AuthMiddleware
    let user_id = req.user_id.clone().unwrap_or("unknown".into());
    Ok(Response::new().json(&json!({
        "user_id": user_id,
        "message": "You are authenticated!"
    }))?)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 5: Request Helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn inspect_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let info = json!({
        "is_json": req.is_json(),
        "is_ajax": req.is_ajax(),
        "content_type": req.header("content-type"),
        "user_agent": req.header("user-agent"),
    });
    Ok(Response::new().json(&info)?)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 6: Response Types
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn html_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().html("<h1>Hello from HTML</h1><p>This is server-rendered.</p>"))
}

async fn redirect_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().redirect("/"))
}

async fn permanent_redirect_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().redirect_permanent("/new-location"))
}

async fn no_content_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::no_content())
}

async fn cookie_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new()
        .set_cookie("theme", "dark")
        .set_cookie("lang", "en")
        .json(&json!({ "message": "Cookies set!" }))?)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 7: WebSocket
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn ws_check_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let is_ws = is_websocket_upgrade(&req);
    Ok(Response::new().json(&json!({
        "is_websocket_upgrade": is_ws,
    }))?)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 8: Form Data + Validation
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn register_handler(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let form = req.form_data().await?;

    let validator = Validator::new()
        .add_rule("name", Required)
        .add_rule("name", MinLength(2))
        .add_rule("email", Required)
        .add_rule("email", Email)
        .add_rule("password", Required)
        .add_rule("password", MinLength(8));

    if let Err(errors) = validator.validate(&form) {
        return Ok(Response::new()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .json(&json!({ "errors": errors }))?);
    }

    let password_hash = hash_password(form.get("password").unwrap()).unwrap();

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({
            "user": {
                "name": form.get("name"),
                "email": form.get("email"),
                "password_hashed": true,
            }
        }))?)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 9: SSE (Server-Sent Events)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn sse_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(16);

    // Spawn a producer that sends 3 events then closes  
    tokio::spawn(async move {
        for i in 1..=3 {
            let msg = serde_json::to_string(&json!({ "count": i })).unwrap();
            if tx.send(msg).await.is_err() { break; }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    Ok(Response::new().event_stream(rx))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Main: Wire everything together
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn build_router() -> Router {
    // Public routes
    let api = Router::new()
        .get("/todos", list_todos)
        .post("/todos", create_todo)
        .get("/todos/:id", get_todo)
        .put("/todos/:id", update_todo)
        .patch("/todos/:id", patch_todo)
        .delete("/todos/:id", delete_todo);

    // Protected routes (require JWT)
    let protected = Router::new()
        .get("/profile", profile_handler)
        .use_middleware(AuthMiddleware::new(Arc::new(JwtAuth::new("my-jwt-secret"))));

    Router::new()
        // Tutorial 1: Hello world
        .get("/", hello_handler)
        .get("/hello", hello_json_handler)
        // Tutorial 2: REST API
        .mount("/api", api)
        // Tutorial 3: Custom middleware (applied globally below)
        // Tutorial 4: Auth
        .post("/auth/login", login_handler)
        .mount("/auth", protected)
        // Tutorial 5: Request inspection
        .get("/inspect", inspect_handler)
        // Tutorial 6: Response types
        .get("/html", html_handler)
        .get("/redirect", redirect_handler)
        .get("/redirect-permanent", permanent_redirect_handler)
        .delete("/no-content", no_content_handler)
        .get("/cookies", cookie_handler)
        // Tutorial 7: WebSocket check
        .get("/ws", ws_check_handler)
        // Tutorial 8: Form validation
        .post("/register", register_handler)
        // Tutorial 9: SSE
        .get("/events", sse_handler)
        // Health check
        .get("/health", HealthCheck::new())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing();
    tracing::info!("Yaiko Tutorials Test App starting...");

    let router = build_router()
        .use_middleware(TimingMiddleware)
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new());

    // Background jobs demo
    let queue = Arc::new(JobQueue::new());
    let q = queue.clone();
    tokio::spawn(async move { q.start().await; });

    queue.add("welcome_email", || async {
        tracing::info!("Background job: sending welcome email...");
        Ok(())
    }).await;

    let app = App::new().router(router);

    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);

    tracing::info!("Tutorials server running at http://{}", addr);
    server.run().await?;

    Ok(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Tutorial 10: TestClient Integration Tests
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hello() {
        let client = TestClient::new(build_router());
        let res = client.get("/").await;
        res.assert_status(200);
        res.assert_body_contains("Hello, Yaiko!");
    }

    #[tokio::test]
    async fn test_hello_json() {
        let client = TestClient::new(build_router());
        let res = client.get("/hello").await;
        res.assert_status(200);
        res.assert_body_contains("Hello, Yaiko!");
        let body: Value = res.json().unwrap();
        assert_eq!(body["version"], "0.1.0");
    }

    #[tokio::test]
    async fn test_list_todos() {
        let client = TestClient::new(build_router());
        let res = client.get("/api/todos").await;
        res.assert_status(200);
        let body: Vec<Todo> = res.json().unwrap();
        assert_eq!(body.len(), 2);
        assert_eq!(body[0].title, "Learn Yaiko");
    }

    #[tokio::test]
    async fn test_create_todo() {
        let client = TestClient::new(build_router());
        let res = client.post("/api/todos", r#"{"title":"Write tests"}"#).await;
        res.assert_status(201);
        res.assert_body_contains("Write tests");
    }

    #[tokio::test]
    async fn test_create_todo_validation_fails() {
        let client = TestClient::new(build_router());
        let res = client.post("/api/todos", r#"{"title":"X"}"#).await;
        res.assert_status(400);
        res.assert_body_contains("errors");
    }

    #[tokio::test]
    async fn test_get_todo() {
        let client = TestClient::new(build_router());
        let res = client.get("/api/todos/42").await;
        res.assert_status(200);
        res.assert_body_contains("42");
    }

    #[tokio::test]
    async fn test_patch_todo() {
        let client = TestClient::new(build_router());
        let res = client.patch("/api/todos/1", r#"{"completed":true}"#).await;
        res.assert_status(200);
        res.assert_body_contains("patched_fields");
    }

    #[tokio::test]
    async fn test_delete_todo() {
        let client = TestClient::new(build_router());
        let res = client.delete("/api/todos/1").await;
        res.assert_status(204);
    }

    #[tokio::test]
    async fn test_html_response() {
        let client = TestClient::new(build_router());
        let res = client.get("/html").await;
        res.assert_status(200);
        res.assert_body_contains("<h1>Hello from HTML</h1>");
    }

    #[tokio::test]
    async fn test_no_content() {
        let client = TestClient::new(build_router());
        let res = client.delete("/no-content").await;
        res.assert_status(204);
    }

    #[tokio::test]
    async fn test_cookies() {
        let client = TestClient::new(build_router());
        let res = client.get("/cookies").await;
        res.assert_status(200);
        res.assert_body_contains("Cookies set!");
    }

    #[tokio::test]
    async fn test_health_check() {
        let client = TestClient::new(build_router());
        let res = client.get("/health").await;
        res.assert_status(200);
        res.assert_body_contains("ok");
    }

    #[tokio::test]
    async fn test_with_auth_header() {
        let client = TestClient::new(build_router())
            .with_header("x-custom", "hello");
        let res = client.get("/inspect").await;
        res.assert_status(200);
    }

    #[tokio::test]
    async fn test_form_validation() {
        let client = TestClient::new(build_router());
        let mut data = HashMap::new();
        data.insert("name".to_string(), "Al".to_string());
        data.insert("email".to_string(), "bad-email".to_string());
        data.insert("password".to_string(), "short".to_string());
        let res = client.post_form("/register", &data).await;
        res.assert_status(422);
        res.assert_body_contains("errors");
    }
}
