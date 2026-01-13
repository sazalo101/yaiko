# Routing

Define HTTP routes and handlers in Yaiko.

## Basic Routes

```rust
use yaiko_core::{Router, Request, Response, StatusCode, HealthCheck};

let router = Router::new()
    .get("/", home_handler)
    .get("/health", HealthCheck::new()) // Built-in health check
    .get("/about", about_handler)
    .post("/contact", contact_handler)
    .put("/users/:id", update_user)
    .delete("/users/:id", delete_user);
```

## Route Parameters

Access dynamic URL segments with `req.param()`:

```rust
// Route: /users/:id
async fn get_user(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let user_id = req.param("id").unwrap();
    
    Ok(Response::new().json(&json!({
        "id": user_id,
        "name": "John Doe"
    }))?)
}
```

## Query Parameters

Access query strings with `req.query()`:

```rust
// URL: /search?q=hello&page=1
async fn search(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let query = req.query("q").unwrap_or("".to_string());
    let page = req.query("page").unwrap_or("1".to_string());
    
    Ok(Response::new().json(&json!({
        "query": query,
        "page": page
    }))?)
}
```

## Request Body

Parse JSON body with `req.json()`:

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

async fn create_user(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body: CreateUser = req.json().await?;
    
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({
            "message": "User created",
            "user": { "name": body.name, "email": body.email }
        }))?)
}
```

## Response Types

### JSON
```rust
Ok(Response::new().json(&json!({ "status": "ok" }))?)
```

### HTML
```rust
Ok(Response::new().html("<h1>Hello</h1>"))
```

### Plain Text
```rust
Ok(Response::new().text("Hello, World!"))
```

### Redirect
```rust
Ok(Response::new().redirect("/login"))
```

### Custom Status
```rust
Ok(Response::new()
    .status(StatusCode::NOT_FOUND)
    .json(&json!({ "error": "Not found" }))?)
```

## Middleware

Apply middleware to routes:

```rust
use yaiko_core::middleware::{Logger, Cors};
use yaiko_core::security::{SecurityHeaders, RateLimiter};

let router = Router::new()
    .get("/", home_handler)
    .use_middleware(Logger)
    .use_middleware(Cors::new().allow_origin("*"))
    .use_middleware(SecurityHeaders::new())
    .use_middleware(RateLimiter::new(100, 60)); // 100 req/min
```

## Grouping Routes

Organize routes by prefix:

```rust
// All routes under /api
let api_routes = Router::new()
    .get("/users", list_users)
    .get("/users/:id", get_user)
    .post("/users", create_user);

let router = Router::new()
    .get("/", home_handler)
    .mount("/api", api_routes);
```

## Static Files

Serve static assets:

```rust
let app = App::new()
    .router(router)
    .static_files("./public", "/static");

// Files in ./public are served at /static/*
// Example: ./public/css/main.css -> /static/css/main.css
```

## Code Generation

Generate controllers with the CLI:

```bash
yaiko generate controller posts

# Creates src/controllers/posts.rs with:
# - list()
# - get()
# - create()
# - update()
# - delete()
```
