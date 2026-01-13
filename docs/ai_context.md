# Building with Yaiko: AI Developer Guide

**Yaiko** is a modern, high-performance fullstack web framework for Rust. This guide provides the context needed for an AI to write valid, idiomatic Yaiko code.

## 1. Core Architecture

- **Backend**: Rust (built on `hyper`, `tokio`, `sqlx`).
- **Frontend**: Server-side rendered HTML + jQuery (optional).
- **Architecture**: MVC-like (Controllers, Models, Templates).

## 2. Project Structure

```text
src/
  main.rs           # Entry point (App, Server, Router)
  controllers/      # Request handlers
  models/           # Database structs (sqlx::FromRow)
  middleware/       # Custom middleware
templates/          # Handlebars templates
public/             # Static assets (css, js)
yaiko.toml          # Configuration
```

## 3. Core API Reference

### Imports
Always import from `yaiko_core`:
```rust
use yaiko_core::{App, Router, Server, Request, Response, StatusCode, json};
```

### Handlers
Handlers are async functions that take a `Request` and return a `Result<Response, Box<dyn Error>>`.

```rust
async fn my_handler(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    // Logic...
    Ok(Response::new().text("OK"))
}
```

### Request Object
- **Params**: `req.param("id")` -> `Option<&String>`
- **Query**: `req.query("q")` -> `Option<&String>`
- **JSON Body**: `let data: MyStruct = req.json().await?;`
- **State**: `let pool = req.state::<sqlx::PgPool>()?;`

### Response Object
- **Text**: `Response::new().text("Hello")`
- **JSON**: `Response::new().json(&json!({"foo": "bar"}))?`
- **HTML**: `Response::new().html("<h1>Hi</h1>")`
- **Status**: `Response::new().status(StatusCode::NOT_FOUND)`

### Routing
```rust
let router = Router::new()
    .get("/", home_handler)
    .post("/users", create_user)
    .put("/users/:id", update_user) // :id is a param
    .use_middleware(LoggingMiddleware::new());
```

## 4. Database (sqlx)

Yaiko uses `sqlx` for database interaction.

### Model
```rust
use sqlx::FromRow;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
}
```

### Accessing Pool
The database pool is available in `req.state`.

```rust
async fn get_users(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let pool = req.state::<sqlx::PgPool>()?;
    
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await?;
        
    Ok(Response::new().json(&json!({ "users": users }))?)
}
```

## 5. Complete Example

```rust
use yaiko_core::{App, Router, Server, Request, Response, json, StatusCode};
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateTask {
    title: String,
}

async fn create_task(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body: CreateTask = req.json().await?;
    
    // In a real app, save to DB here
    
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({
            "message": "Task created",
            "title": body.title
        }))?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = Router::new()
        .post("/tasks", create_task);
        
    let app = App::new().router(router);
    
    let server = Server::new(app, "127.0.0.1:3000".parse()?);
    server.run().await?;
    Ok(())
}
```

## 6. Key Features
- **WebSockets**: `yaiko_core::is_websocket_upgrade(&req)`
- **Jobs**: `state.queue.add("job_name", async_task)`
- **Uploads**: `yaiko_core::parse_multipart(req.body, content_type)`
