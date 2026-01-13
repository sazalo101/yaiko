use yaiko_core::{
    App, Router, Server, Request, Response, StatusCode, json,
    LoggingMiddleware, SecurityHeaders, HealthCheck, RateLimiter, init_tracing, tracing,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;

// ============================================================================
// MODELS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
pub struct OpenRouterResponse {
    pub choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
pub struct OpenRouterChoice {
    pub message: Message,
}

// ============================================================================
// STATE
// ============================================================================

#[derive(Clone)]
pub struct AppState {
    db: SqlitePool,
    api_key: String,
}

impl AppState {
    async fn new() -> Self {
        dotenv::dotenv().ok();
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .expect("OPENROUTER_API_KEY must be set");
        
        let db_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:chat.db".to_string());
            
        let db = SqlitePool::connect(&db_url).await
            .expect("Failed to connect to database");
            
        // Run migrations
        let schema = include_str!("../schema.sql");
        sqlx::query(schema).execute(&db).await
            .expect("Failed to run migrations");
        
        AppState {
            db,
            api_key,
        }
    }
    
    async fn get_user_from_token(&self, token: &str) -> Option<User> {
        let session = sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE token = ? AND expires_at > ?")
            .bind(token)
            .bind(Utc::now())
            .fetch_optional(&self.db)
            .await
            .ok()??;
            
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(session.user_id)
            .fetch_optional(&self.db)
            .await
            .ok()?
    }
}

// ============================================================================
// MAIN
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    init_tracing();
    
    tracing::info!("Yaiko Chat - AI Chat Application");
    
    let state = AppState::new().await;
    
    let router = Router::new()
        // Health check
        .get("/health", HealthCheck::new())
        // Pages
        .get("/", {
            let s = state.clone();
            move |req| home_handler(req, s.clone())
        })
        .get("/chat", {
            let s = state.clone();
            move |req| chat_page(req, s.clone())
        })
        // Auth API
        .post("/api/signup", {
            let s = state.clone();
            move |req| signup(req, s.clone())
        })
        .post("/api/login", {
            let s = state.clone();
            move |req| login(req, s.clone())
        })
        .post("/api/logout", {
            let s = state.clone();
            move |req| logout(req, s.clone())
        })
        .get("/api/me", {
            let s = state.clone();
            move |req| get_me(req, s.clone())
        })
        // Chat API
        .post("/api/chat", {
            let s = state.clone();
            move |req| chat(req, s.clone())
        })
        .get("/api/conversations", {
            let s = state.clone();
            move |req| list_conversations(req, s.clone())
        })
        .get("/api/conversations/:id", {
            let s = state.clone();
            move |req| get_conversation(req, s.clone())
        })
        // SEO
        .get("/robots.txt", robots_handler)
        .get("/sitemap.xml", sitemap_handler)
        // Static files
        .static_files("/static", "./public")
        // Middleware
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new())
        .use_middleware(RateLimiter::new(100, 60));
    
    let app = App::new().router(router);
    
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    tracing::info!("Server running at http://{}", addr);
    server.run().await?;
    
    Ok(())
}

// ============================================================================
// PAGE HANDLERS
// ============================================================================

async fn home_handler(_req: Request, _state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().html(include_str!("../templates/index.html")))
}

async fn chat_page(_req: Request, _state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().html(include_str!("../templates/chat.html")))
}

// ============================================================================
// AUTH HANDLERS
// ============================================================================

async fn signup(mut req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let value = req.json().await?;
    let body: SignupRequest = serde_json::from_value(value)?;
    
    // Check if user exists
    let exists = sqlx::query("SELECT 1 FROM users WHERE email = ?")
        .bind(&body.email)
        .fetch_optional(&state.db)
        .await?;
        
    if exists.is_some() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({"error": "Email already registered"}))?);
    }
    
    // Hash password
    let password_hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST)
        .map_err(|e| format!("Hash error: {}", e))?;
    
    // Create user
    let user_id = Uuid::new_v4().to_string();
    let created_at = Utc::now();
    
    sqlx::query("INSERT INTO users (id, email, password_hash, created_at) VALUES (?, ?, ?, ?)")
        .bind(&user_id)
        .bind(&body.email)
        .bind(&password_hash)
        .bind(created_at)
        .execute(&state.db)
        .await?;
        
    let user = User {
        id: user_id.clone(),
        email: body.email,
        password_hash,
        created_at,
    };
    
    // Create session
    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(7);
    
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, ?)")
        .bind(&token)
        .bind(&user_id)
        .bind(expires_at)
        .execute(&state.db)
        .await?;
    
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({
            "user": user,
            "token": token
        }))?)
}

async fn login(mut req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let value = req.json().await?;
    let body: LoginRequest = serde_json::from_value(value)?;
    
    // Find user
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&body.email)
        .fetch_optional(&state.db)
        .await?;
        
    let user = match user {
        Some(u) => u,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Invalid credentials"}))?),
    };
    
    // Verify password
    let valid = bcrypt::verify(&body.password, &user.password_hash)
        .map_err(|e| format!("Verify error: {}", e))?;
    
    if !valid {
        return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Invalid credentials"}))?);
    }
    
    // Create session
    let token = Uuid::new_v4().to_string();
    let expires_at = Utc::now() + chrono::Duration::days(7);
    
    sqlx::query("INSERT INTO sessions (token, user_id, expires_at) VALUES (?, ?, ?)")
        .bind(&token)
        .bind(&user.id)
        .bind(expires_at)
        .execute(&state.db)
        .await?;
    
    Ok(Response::new().json(&json!({
        "user": user,
        "token": token
    }))?)
}

async fn logout(req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(token) = get_auth_token(&req) {
        sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(token)
            .execute(&state.db)
            .await?;
    }
    Ok(Response::new().json(&json!({"message": "Logged out"}))?)
}

async fn get_me(req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    if let Some(token) = get_auth_token(&req) {
        if let Some(user) = state.get_user_from_token(&token).await {
            return Ok(Response::new().json(&json!({"user": user}))?);
        }
    }
    Ok(Response::new()
        .status(StatusCode::UNAUTHORIZED)
        .json(&json!({"error": "Not authenticated"}))?)
}

// ============================================================================
// CHAT HANDLERS
// ============================================================================

async fn chat(mut req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    // Get user from token
    let token = match get_auth_token(&req) {
        Some(t) => t,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Not authenticated"}))?),
    };
    
    let user = match state.get_user_from_token(&token).await {
        Some(u) => u,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Invalid session"}))?),
    };
    
    let value = req.json().await?;
    let body: ChatRequest = serde_json::from_value(value)?;
    
    // Get or create conversation
    let conv_id = match body.conversation_id {
        Some(id) => id,
        None => {
            let id = Uuid::new_v4().to_string();
            let title = body.message.chars().take(30).collect::<String>() + "...";
            let now = Utc::now();
            
            sqlx::query("INSERT INTO conversations (id, user_id, title, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
                .bind(&id)
                .bind(&user.id)
                .bind(&title)
                .bind(now)
                .bind(now)
                .execute(&state.db)
                .await?;
                
            id
        }
    };
    
    // Save user message
    let user_msg_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    sqlx::query("INSERT INTO messages (id, conversation_id, role, content, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(&user_msg_id)
        .bind(&conv_id)
        .bind("user")
        .bind(&body.message)
        .bind(now)
        .execute(&state.db)
        .await?;
        
    // Get conversation history for context
    let messages = sqlx::query_as::<_, (String, String)>("SELECT role, content FROM messages WHERE conversation_id = ? ORDER BY created_at ASC")
        .bind(&conv_id)
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|(role, content)| Message { role, content })
        .collect::<Vec<_>>();
    
    // Call OpenRouter API
    let client = reqwest::Client::new();
    let api_response = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", state.api_key))
        .json(&OpenRouterRequest {
            model: "mistralai/devstral-2512:free".to_string(),
            messages,
        })
        .send()
        .await
        .map_err(|e| format!("API error: {}", e))?;
    
    let api_result: OpenRouterResponse = api_response.json().await
        .map_err(|e| format!("Parse error: {}", e))?;
    
    // Save AI response
    let response_message = if let Some(choice) = api_result.choices.first() {
        let ai_msg_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        
        sqlx::query("INSERT INTO messages (id, conversation_id, role, content, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(&ai_msg_id)
            .bind(&conv_id)
            .bind(&choice.message.role)
            .bind(&choice.message.content)
            .bind(now)
            .execute(&state.db)
            .await?;
            
        // Update conversation timestamp
        sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(&conv_id)
            .execute(&state.db)
            .await?;
            
        Some(choice.message.clone())
    } else {
        None
    };
    
    Ok(Response::new().json(&json!({
        "conversation_id": conv_id,
        "message": response_message
    }))?)
}

async fn list_conversations(req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let token = match get_auth_token(&req) {
        Some(t) => t,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Not authenticated"}))?),
    };
    
    let user = match state.get_user_from_token(&token).await {
        Some(u) => u,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Invalid session"}))?),
    };
    
    let conversations = sqlx::query("SELECT id, title, updated_at FROM conversations WHERE user_id = ? ORDER BY updated_at DESC")
        .bind(&user.id)
        .fetch_all(&state.db)
        .await?;
        
    let user_convs: Vec<_> = conversations.iter().map(|row| {
        json!({
            "id": row.get::<String, _>("id"),
            "title": row.get::<String, _>("title"),
            "updated_at": row.get::<DateTime<Utc>, _>("updated_at")
        })
    }).collect();
    
    Ok(Response::new().json(&json!({"conversations": user_convs}))?)
}

async fn get_conversation(req: Request, state: AppState) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let token = match get_auth_token(&req) {
        Some(t) => t,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Not authenticated"}))?),
    };
    
    let user = match state.get_user_from_token(&token).await {
        Some(u) => u,
        None => return Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({"error": "Invalid session"}))?),
    };
    
    let conv_id = req.param("id").map(|s| s.to_string()).unwrap_or_default();
    
    // Check ownership
    let conv = sqlx::query("SELECT * FROM conversations WHERE id = ? AND user_id = ?")
        .bind(&conv_id)
        .bind(&user.id)
        .fetch_optional(&state.db)
        .await?;
        
    if let Some(conv_row) = conv {
        // Get messages
        let messages = sqlx::query_as::<_, (String, String)>("SELECT role, content FROM messages WHERE conversation_id = ? ORDER BY created_at ASC")
            .bind(&conv_id)
            .fetch_all(&state.db)
            .await?
            .into_iter()
            .map(|(role, content)| Message { role, content })
            .collect::<Vec<_>>();
            
        let conversation = Conversation {
            id: conv_row.get("id"),
            user_id: conv_row.get("user_id"),
            title: conv_row.get("title"),
            messages,
            created_at: conv_row.get("created_at"),
            updated_at: conv_row.get("updated_at"),
        };
        
        return Ok(Response::new().json(&json!({"conversation": conversation}))?);
    }
    
    Ok(Response::new()
        .status(StatusCode::NOT_FOUND)
        .json(&json!({"error": "Conversation not found"}))?)
}

// ============================================================================
// SEO HANDLERS
// ============================================================================

async fn robots_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let content = "User-agent: *\nAllow: /\nDisallow: /api/\nDisallow: /chat\nSitemap: /sitemap.xml\n";
    Ok(Response::new().text(content))
}

async fn sitemap_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let host = "http://localhost:3000";
    let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>{}/</loc>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
</urlset>
"#, host);
    
    Ok(Response::new()
        .header("Content-Type", "application/xml")
        .text(&xml))
}

// ============================================================================
// HELPERS
// ============================================================================

fn get_auth_token(req: &Request) -> Option<String> {
    req.headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}
