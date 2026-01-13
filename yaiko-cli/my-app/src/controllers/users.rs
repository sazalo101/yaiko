use yaiko_core::{Request, Response, StatusCode, json};

pub async fn list(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let users = vec![
        json!({"id": 1, "name": "Alice", "email": "alice@example.com"}),
        json!({"id": 2, "name": "Bob", "email": "bob@example.com"}),
    ];
    Ok(Response::new().json(&json!({ "users": users }))?)
}

pub async fn get(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let id = req.param("id").map(|s| s.as_str()).unwrap_or("0");
    Ok(Response::new().json(&json!({
        "id": id,
        "name": "John Doe",
        "email": "john@example.com"
    }))?)
}

pub async fn create(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body = req.json().await?;
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({
            "message": "User created successfully",
            "user": body
        }))?)
}
