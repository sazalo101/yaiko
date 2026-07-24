use yaiko_core::{Request, Response, BoxError, StatusCode, json};
use yaiko_core::{hash_password, verify_password, Session};
use serde::{Deserialize, Serialize};
use sqlx::{SqlitePool, Row};
use uuid::Uuid;

#[derive(Deserialize)]
struct AuthInput {
    username: String,
    password: String,
}

pub async fn register(mut req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let body = req.json().await?;
    let input: AuthInput = serde_json::from_value(body.clone())?;

    if input.username.trim().is_empty() || input.password.len() < 6 {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({"error": "Invalid username or password (min 6 chars)"}))?);
    }

    let hashed = hash_password(&input.password)?;
    let id = Uuid::new_v4().to_string();

    let result = sqlx::query("INSERT INTO users (id, username, password_hash) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&input.username)
        .bind(&hashed)
        .execute(&pool)
        .await;

    match result {
        Ok(_) => {
            // Set session automatically
            if let Some(mut session) = req.session.take() {
                session.set("user_id", &id).unwrap();
                session.set("username", &input.username).unwrap();
                req.session = Some(session);
            }
            
            Ok(Response::new().status(StatusCode::CREATED).json(&json!({"id": id, "username": input.username}))?)
        }
        Err(_) => {
            Ok(Response::new().status(StatusCode::CONFLICT).json(&json!({"error": "Username taken"}))?)
        }
    }
}

pub async fn login(mut req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let body = req.json().await?;
    let input: AuthInput = serde_json::from_value(body.clone())?;

    let row = sqlx::query("SELECT id, password_hash FROM users WHERE username = ?")
        .bind(&input.username)
        .fetch_optional(&pool)
        .await?;

    if let Some(r) = row {
        let stored_hash: String = r.get("password_hash");
        if verify_password(&input.password, &stored_hash)? {
            let id: String = r.get("id");
            
            // Set session
            if let Some(mut session) = req.session.take() {
                session.set("user_id", &id).unwrap();
                session.set("username", &input.username).unwrap();
                req.session = Some(session);
            }
            
            return Ok(Response::new().json(&json!({"id": id, "username": input.username}))?);
        }
    }

    Ok(Response::new().status(StatusCode::UNAUTHORIZED).json(&json!({"error": "Invalid credentials"}))?)
}

pub async fn me(req: Request) -> Result<Response, BoxError> {
    if let Some(session) = &req.session {
        if let Some(user_id) = session.get::<String>("user_id") {
            let username = session.get::<String>("username").unwrap_or_default();
            Ok(Response::new().json(&json!({"id": user_id, "username": username}))?)
        } else {
            Ok(Response::new().status(StatusCode::UNAUTHORIZED).json(&json!({"error": "Not logged in"}))?)
        }
    } else {
        Ok(Response::new().status(StatusCode::UNAUTHORIZED).json(&json!({"error": "No session"}))?)
    }
}

pub async fn logout(mut req: Request) -> Result<Response, BoxError> {
    if let Some(session) = req.session.take() {
        session.clear();
        req.session = Some(session);
    }
    Ok(Response::new().json(&json!({"status": "logged_out"}))?)
}
