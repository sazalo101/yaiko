use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use yaiko_core::{
    hash_password, login_session, logout_session, require_role, verify_password, App, MemorySessionStore,
    Request, Response, Router, Server, SessionAuth, SessionMiddleware, Settings, StatusCode, json, tracing,
};
use yaiko_core::middleware::{Cors, Logger};

#[derive(Debug, Clone, Serialize)]
struct User {
    id: String,
    email: String,
    password_hash: String,
    roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AuthInput {
    email: String,
    password: String,
}

type Users = Arc<RwLock<HashMap<String, User>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let settings = Settings::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let users = Arc::new(RwLock::new(HashMap::new()));
    let sessions = Arc::new(MemorySessionStore::new());
    let users_for_register = users.clone();
    let users_for_login = users.clone();
    let users_for_me = users.clone();

    let router = Router::new()
        .get("/", |_req: Request| async move {
            Ok(Response::new().json(&json!({
                "message": "Auth example",
                "routes": ["/register", "/login", "/me", "/admin", "/logout"]
            }))?)
        })
        .post("/register", move |req: Request| {
            let users = users_for_register.clone();
            async move { register(req, users).await }
        })
        .post("/login", move |req: Request| {
            let users = users_for_login.clone();
            async move { login(req, users).await }
        })
        .get("/me", move |req: Request| {
            let users = users_for_me.clone();
            async move { me(req, users).await }
        })
        .get("/admin", |req: Request| async move { admin(req).await })
        .post("/logout", |req: Request| async move { logout(req).await })
        .use_middleware(Logger)
        .use_middleware(Cors::new().allow_origin("*"))
        .use_middleware(SessionAuth::new().skip_path("/").skip_path("/register").skip_path("/login"))
        .use_middleware(SessionMiddleware::new(sessions).secure(false));

    let app = App::new().router(router);
    let addr: SocketAddr = settings.server_addr().parse()?;
    let server = Server::new(app, addr);

    tracing::info!("Auth example running on http://{}", addr);
    server.run().await?;
    Ok(())
}

async fn register(mut req: Request, users: Users) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let input = parse_auth_input(&mut req).await?;
    let email = normalize_email(&input.email);

    if input.password.len() < 8 || email.is_empty() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({"error": "Email and password (min 8 chars) are required"}))?);
    }

    let mut users = users.write().await;
    if users.contains_key(&email) {
        return Ok(Response::new()
            .status(StatusCode::CONFLICT)
            .json(&json!({"error": "User already exists"}))?);
    }

    let roles = if users.is_empty() {
        vec!["admin".to_string()]
    } else {
        vec!["user".to_string()]
    };

    let user = User {
        id: format!("user-{}", users.len() + 1),
        email: email.clone(),
        password_hash: hash_password(&input.password)?,
        roles: roles.clone(),
    };

    users.insert(email.clone(), user.clone());

    let session = req.session.as_ref().expect("session middleware required");
    login_session(session, &user.id, &roles)?;

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({"id": user.id, "email": email, "roles": roles}))?)
}

async fn login(mut req: Request, users: Users) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let input = parse_auth_input(&mut req).await?;
    let email = normalize_email(&input.email);
    let user = users.read().await.get(&email).cloned();

    match user {
        Some(user) if verify_password(&input.password, &user.password_hash)? => {
            let session = req.session.as_ref().expect("session middleware required");
            login_session(session, &user.id, &user.roles)?;
            Ok(Response::new().json(&json!({
                "id": user.id,
                "email": user.email,
                "roles": user.roles
            }))?)
        }
        _ => Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Invalid credentials"}))?),
    }
}

async fn me(req: Request, users: Users) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let Some(user_id) = req.user_id.clone() else {
        return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Not authenticated"}))?);
    };

    let users = users.read().await;
    let user = users.values().find(|user| user.id == user_id);

    match user {
        Some(user) => Ok(Response::new().json(&json!({
            "id": user.id,
            "email": user.email,
            "roles": user.roles
        }))?),
        None => Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Session user no longer exists"}))?),
    }
}

async fn admin(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    if let Err(response) = require_role(&req, "admin") {
        return Ok(response);
    }

    Ok(Response::new().json(&json!({
        "message": "Welcome, admin",
        "user_id": req.user_id
    }))?)
}

async fn logout(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(session) = &req.session {
        logout_session(session);
    }

    Ok(Response::new().json(&json!({"status": "logged_out"}))?)
}

async fn parse_auth_input(req: &mut Request) -> Result<AuthInput, Box<dyn std::error::Error + Send + Sync>> {
    let body = req.json().await?;
    Ok(serde_json::from_value(body)?)
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
