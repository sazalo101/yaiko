# Building APIs with Yaiko
2026-01-10

Learn how to build RESTful APIs with the Yaiko framework.

## Defining Routes

Routes in Yaiko are defined using a fluent API:

```rust
let router = Router::new()
    .get("/api/users", list_users)
    .get("/api/users/:id", get_user)
    .post("/api/users", create_user);
```

## Handling JSON

Parse JSON requests and return JSON responses:

```rust
async fn create_user(mut req: Request) -> Result<Response, Error> {
    let body: CreateUser = req.json().await?;
    Ok(Response::new().json(&json!({ "user": body }))?)
}
```

## Error Handling

Use the AppError type for automatic HTTP status mapping:

```rust
use yaiko_core::error::AppError;

if user.is_none() {
    return Err(AppError::NotFound("User not found".to_string()));
}
```

That's how easy it is to build APIs with Yaiko!
