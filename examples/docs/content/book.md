# The Yaiko Book

> A modern, production-ready fullstack web framework for Rust + jQuery  
> 🌐 **Repository**: [github.com/sazalo101/yaiko](https://github.com/sazalo101/yaiko)

---

## Chapter 1 — What Yaiko Is

Yaiko is a batteries-included web framework built on **hyper** and **tokio**. It gives you the speed of raw Rust with the developer experience of Rails or Laravel — routing, sessions, auth, jobs, WebSockets, and templating all ship in one crate.

### Core Philosophy

| Principle | How |
|---|---|
| **Zero-cost abstractions** | Built on hyper — no runtime overhead |
| **Convention over config** | `yaiko init` scaffolds everything |
| **Fullstack by default** | Rust backend + jQuery frontend in one project |
| **Production-first** | CSRF, rate limiting, HSTS, CSP, graceful shutdown out of the box |

---

## Chapter 2 — Architecture

```mermaid
graph TB
    Client-->|HTTP/WS|Server
    Server-->|hyper|App
    App-->|middleware chain|Router
    Router-->|matched route|Handler
    Handler-->Response
    App-->SessionMiddleware
    App-->SecurityHeaders
    App-->AuthMiddleware
    App-->RateLimiter
    App-->CsrfProtection
    App-->CompressionMiddleware
    App-->LoggingMiddleware
    Router-->StaticFiles
    Router-->WebSocketManager
    App-->JobQueue
    App-->Database
    App-->Cache
```

### Module Map

| Module | Purpose |
|---|---|
| `app.rs` | Request lifecycle, middleware pipeline, panic recovery, auto-SEO fallback, custom 404/500 |
| `router.rs` | Route registration (GET/POST/PUT/DELETE/PATCH/OPTIONS/HEAD), params, mount, 405 handling, trailing slash normalization |
| `request.rs` | Body parsing (JSON, form, multipart), headers, `is_json()`, `is_ajax()`, `body_bytes()` |
| `response.rs` | JSON/HTML/text builders, redirects, `no_content()`, `stream()`, `event_stream()`, `set_cookie()` |
| `server.rs` | Hyper server boot with graceful shutdown on SIGTERM/Ctrl+C |
| `middleware.rs` | Middleware trait and chain execution |
| `handler.rs` | Handler trait for route callbacks |
| `auth.rs` | JWT creation/validation, `AuthMiddleware` with role-based guards |
| `session.rs` | Cookie-based sessions, `MemorySessionStore`, auto-cleanup background task |
| `security.rs` | `SecurityHeaders` (CSP, HSTS, X-Frame, etc.), `RateLimiter`, `CsrfProtection` |
| `database.rs` | PostgreSQL/SQLite connection pool via sqlx |
| `cache.rs` | In-memory and Redis caching |
| `jobs.rs` | Background job queue with exponential backoff retries |
| `websocket.rs` | Connection manager, rooms, send/broadcast, keepalive, rate limiting, upgrade handler |
| `template.rs` | Handlebars template rendering |
| `static_files.rs` | Static file serving with MIME detection and Cache-Control |
| `file_upload.rs` | Multipart form parsing (files + text fields) |
| `compression.rs` | Gzip response compression |
| `validation.rs` | Declarative field validation (Required, Email, MinLength, MaxLength) |
| `seo.rs` | Robots.txt and sitemap generation |
| `health.rs` | Health check endpoints |
| `logging.rs` | Structured tracing with `tracing` crate |
| `testing.rs` | `TestClient` with auth/session/header injection, PATCH, form POST |
| `config.rs` | TOML-based settings (yaiko.toml) |
| `error.rs` | Typed error handling |
| `metrics.rs` | Request metrics (optional feature) |
| `dev.rs` | Hot-reload dev utilities (optional feature) |

---

## Chapter 3 — Benchmarks

### Test Environment

| Spec | Value |
|---|---|
| Tool | Apache Benchmark (`ab`) |
| Requests | 50,000 |
| Concurrency | 100 |
| Keep-Alive | Enabled |
| Build | `--release` (optimized) |

### Results

#### Plaintext — `GET /plaintext` → `"Hello, World!"`

| Metric | Value |
|---|---|
| **Requests/sec** | **74,878** |
| Mean latency | 1.336 ms |
| P50 | 1 ms |
| P95 | 3 ms |
| P99 | 5 ms |
| Max | 13 ms |
| Transfer rate | 10,164 KB/s |

#### JSON — `GET /json` → `{"message":"Hello, World!"}`

| Metric | Value |
|---|---|
| **Requests/sec** | **81,336** |
| Mean latency | 1.229 ms |
| P50 | 1 ms |
| P95 | 2 ms |
| P99 | 4 ms |
| Max | 12 ms |
| Transfer rate | 12,629 KB/s |

### Live Production Benchmark (ImgHost — `wrk 4.1.0`)

| Environment | Throughput | Latency (p50) | Latency (p99) |
|---|---|---|---|
| **Raw App Engine** | **11,234 req/s** | 8.48 ms | 19.07 ms |
| **Nginx TLS (SSL)** | **3,053 req/s** | 24.12 ms | 52.30 ms |
| **1,000 Conn Stress** | **12,042 req/s** | 41.20 ms | 88.60 ms |

> Zero request failures across 362,000+ requests under 1,000 concurrent connections.

### Context

| Framework | Plaintext RPS (approx) |
|---|---|
| **Yaiko (Rust/hyper)** | **~75,000–81,000** |
| Actix Web (Rust) | ~100,000–150,000 |
| Express.js (Node) | ~15,000–25,000 |
| Django (Python) | ~1,500–3,000 |
| Rails (Ruby) | ~2,000–4,000 |
| Laravel (PHP) | ~1,000–2,500 |
| Spring Boot (Java) | ~25,000–50,000 |

> Yaiko sits comfortably in the high-performance tier — 3–5× faster than Node.js and 30–50× faster than Python/Ruby frameworks — while shipping batteries-included features those frameworks are known for.

---

## Chapter 4 — What You Can Build

### Feature Capabilities

| Category | What Yaiko Provides |
|---|---|
| **Routing** | RESTful routes, path params, query params, sub-router mounting, wildcard matching, 405 with Allow headers |
| **Auth** | JWT tokens, bcrypt password hashing, role-based middleware guards |
| **Sessions** | Cookie-based sessions, pluggable stores (memory, can extend to Redis/DB), auto-expiry cleanup |
| **Realtime** | WebSocket upgrade, connection rooms, broadcast, per-user messaging, keepalive, rate limiting |
| **Background Jobs** | Async job queue with retry + exponential backoff |
| **File Uploads** | Multipart parsing with file + text field extraction |
| **Templates** | Handlebars server-side rendering |
| **Security** | CSRF double-submit, rate limiting, CSP, HSTS, XSS protection, CORS |
| **Database** | PostgreSQL + SQLite via sqlx, connection pooling, migration runner |
| **Caching** | In-memory + Redis, key-value with TTL |
| **SEO** | Auto robots.txt, sitemap, meta tag helpers |
| **SSE** | `Response::event_stream()` for server-sent events |
| **Streaming** | `Response::stream()` for chunked file downloads |
| **Testing** | Full test client with auth/session injection |

---

## Chapter 5 — 15 Side Project Ideas

### 1. 🔗 LinkShelf — Bookmark Manager

A personal bookmark manager with tagging, full-text search, and link previews.

| Feature | Yaiko Module |
|---|---|
| User accounts | `auth.rs` (JWT + bcrypt) |
| Bookmark CRUD | `router.rs` (REST routes) |
| Tag filtering | `request.rs` (query params) |
| Link preview scraping | `jobs.rs` (background fetch) |
| Import/export | `file_upload.rs` (HTML bookmark file) |
| Public sharing | `static_files.rs` + templates |

**Niche appeal:** Developers who want a self-hosted, privacy-first alternative to Pocket/Raindrop.

---

### 2. 📊 PulseBoard — Uptime Monitor

A lightweight uptime monitoring dashboard that pings endpoints and alerts on downtime.

| Feature | Yaiko Module |
|---|---|
| Endpoint configuration | `config.rs` + database |
| Periodic health checks | `jobs.rs` (scheduled pings) |
| Status dashboard | `template.rs` (Handlebars) |
| Real-time status updates | `websocket.rs` (broadcast status changes) |
| Incident history | `database.rs` (PostgreSQL logs) |
| Public status page | `static_files.rs` + `seo.rs` |
| Webhook alerts | `jobs.rs` (fire HTTP calls on failure) |

**Niche appeal:** Small teams who need a self-hosted StatusPage without paying $29/mo.

---

### 3. 📝 DropNote — Ephemeral Notes

Create encrypted, self-destructing notes with a shareable link.

| Feature | Yaiko Module |
|---|---|
| Note creation | `router.rs` POST with `request.rs` JSON parsing |
| Encryption | Server-side AES before DB storage |
| View-once / TTL expiry | `session.rs` counter + `jobs.rs` cleanup |
| Password protection | `auth.rs` bcrypt |
| Rate limiting | `security.rs` RateLimiter |
| Burn-after-read | `database.rs` DELETE after first GET |

**Niche appeal:** Security-conscious teams sharing secrets, API keys, credentials.

---

### 4. 🎯 FeedbackOwl — User Feedback Widget

An embeddable feedback widget (like Canny/UserVoice) that collects feature requests and bugs.

| Feature | Yaiko Module |
|---|---|
| Widget JS embed | `static_files.rs` (serve widget script) |
| CORS for cross-origin | `security.rs` + `router.rs` OPTIONS |
| Feedback submission | `validation.rs` (Required, Email, MaxLength) |
| Upvoting | `database.rs` + `auth.rs` |
| Admin dashboard | `template.rs` + `auth.rs` (role guards) |
| Email notifications | `jobs.rs` webhook |

**Niche appeal:** Indie hackers who want user feedback without paying $49/mo for Canny.

---

### 5. 🗓️ MeetSlot — Scheduling Tool

A Calendly-like scheduling tool for booking meetings.

| Feature | Yaiko Module |
|---|---|
| Availability config | `database.rs` + JSON API |
| Booking page | `template.rs` (Handlebars) + `seo.rs` |
| Slot reservation | `database.rs` (transactions) |
| Confirmation emails | `jobs.rs` (background email) |
| Calendar integration | JSON/iCal export endpoints |
| Timezone handling | `chrono` (already a dependency) |

**Niche appeal:** Freelancers and consultants who want a self-hosted, no-subscription scheduler.

---

### 6. 📡 WebhookRelay — Webhook Testing Tool

Capture, inspect, and replay webhooks from third-party services.

| Feature | Yaiko Module |
|---|---|
| Unique endpoint generation | `router.rs` (dynamic path params) |
| Request capture | `request.rs` (`body_bytes()`, `headers`) |
| Real-time feed | `websocket.rs` (push captured hooks live) |
| Replay button | `jobs.rs` (re-fire stored request) |
| Search/filter | `database.rs` + `request.rs` query params |
| TTL auto-cleanup | `jobs.rs` + `session.rs` pattern |

**Niche appeal:** Developers integrating Stripe, GitHub, Twilio webhooks.

---

### 7. 📸 SnapVault — Image Hosting

A private image hosting service with direct links and gallery view.

| Feature | Yaiko Module |
|---|---|
| Drag-and-drop upload | `file_upload.rs` (multipart) |
| Image serving | `static_files.rs` with Cache-Control |
| Gallery rendering | `template.rs` (Handlebars grid) |
| Auth-gated uploads | `auth.rs` + `session.rs` |
| Batch delete | `router.rs` DELETE + `request.rs` JSON array |
| Streaming download | `response.rs` `stream()` |
| CDN-friendly headers | `response.rs` Cache-Control, ETag |

**Niche appeal:** Developers who want a self-hosted Imgur for screenshots and docs.

---

### 8. 💬 TeamPulse — Team Chat

A Slack-lite internal team chat with channels and direct messages.

| Feature | Yaiko Module |
|---|---|
| Real-time messaging | `websocket.rs` (rooms = channels) |
| User presence | `websocket.rs` keepalive + `broadcast` |
| Message persistence | `database.rs` |
| File sharing | `file_upload.rs` |
| Auth + sessions | `auth.rs` + `session.rs` |
| Message rate limiting | `websocket.rs` `check_rate_limit()` |
| Typing indicators | `websocket.rs` `send_to_room()` |

**Niche appeal:** Small teams (<20) who want chat without Slack's pricing.

---

### 9. 📋 FormForge — Form Builder

| Feature | Yaiko Module |
|---|---|
| Form designer (JSON schema) | `database.rs` + REST API |
| Submission handling | `validation.rs` (dynamic rules) |
| File uploads in forms | `file_upload.rs` |
| Webhook on submission | `jobs.rs` (fire POST to user URL) |
| CSRF protection | `security.rs` CsrfProtection |

---

### 10. 🔑 VaultAPI — API Key Management

| Feature | Yaiko Module |
|---|---|
| Key generation | `auth.rs` (JWT or UUID-based) |
| Rate limiting per key | `security.rs` RateLimiter |
| Audit log | `database.rs` + `logging.rs` |
| Key rotation | `jobs.rs` (scheduled rotation) |

---

### 11. 📰 RSS-Forge — RSS Feed Aggregator

| Feature | Yaiko Module |
|---|---|
| Background polling | `jobs.rs` (periodic fetch + parse) |
| Full-text search | `database.rs` (PostgreSQL `tsvector`) |
| OPML import | `file_upload.rs` (XML file) |
| Real-time new article push | `websocket.rs` broadcast |

---

### 12. ⏱️ TimeTrail — Time Tracking

| Feature | Yaiko Module |
|---|---|
| Timer start/stop | REST API + `database.rs` |
| CSV export | `response.rs` `text()` with CSV Content-Type |
| Invoice PDF generation | `jobs.rs` (background render) |

---

### 13. 🏥 HealthDash — Personal Health Logger

| Feature | Yaiko Module |
|---|---|
| Daily log entries | REST API + `validation.rs` |
| Data export | `response.rs` `stream()` (CSV/JSON download) |
| Shareable reports | `auth.rs` (time-limited JWT links) |

---

### 14. 🎮 TriviaLive — Real-Time Quiz Game

| Feature | Yaiko Module |
|---|---|
| Game lobby | `websocket.rs` rooms |
| Live questions | `websocket.rs` `send_to_room()` |
| Anti-cheat rate limiting | `websocket.rs` `check_rate_limit()` |

---

### 15. 🛡️ AuditTrail — Compliance Logging Service

| Feature | Yaiko Module |
|---|---|
| Append-only storage | `database.rs` (INSERT-only, no UPDATE/DELETE) |
| Real-time stream | `response.rs` `event_stream()` (SSE) |
| Export | `response.rs` `stream()` for large CSV dumps |
| Integrity hashing | Chain-hash each entry (tamper detection) |

---

## Chapter 6 — Getting Started

```bash
# Install the CLI
git clone https://github.com/sazalo101/yaiko.git
cd yaiko
cargo install --path yaiko-cli --force

# Check your environment
yaiko doctor

# Create a new project
yaiko init my-app --database sqlite

# Start developing
cd my-app
yaiko dev
```

A new project ships with:
- `src/main.rs` — app entry point with middleware stack
- `src/controllers/` — route handlers
- `public/js/core.js` — Yaiko jQuery helpers (`Yaiko.api.*`, `Yaiko.ui.confirm()`)
- `templates/` — Handlebars templates
- `yaiko.toml` — framework configuration
- `migrations/` — SQL migration files

---

## Chapter 7 — CLI Reference

| Command | Description |
|---|---|
| `yaiko init <name> -d sqlite` | Create a new project (postgres or sqlite) |
| `yaiko dev` | Start dev server with hot-reloading |
| `yaiko dev -p 8080` | Dev server on custom port |
| `yaiko build --release` | Compile optimized production binary |
| `yaiko run` | Build and run the current project once without the development watcher |
| `yaiko doctor` | Check Rust, Cargo, and environment readiness |
| `yaiko migrate create <name>` | Create a new SQL migration file |
| `yaiko migrate run` | Run all pending migrations |
| `yaiko migrate rollback` | Roll back the last migration |
| `yaiko migrate status` | Show migration history |
| `yaiko generate controller <name>` | Scaffold a controller |
| `yaiko generate model <name>` | Scaffold a data model |
| `yaiko generate middleware <name>` | Scaffold custom middleware |

---

## Chapter 8 — Tutorial: Hello World

The simplest Yaiko app. Responds with plain text and JSON.

### `src/main.rs`
```rust
use yaiko_core::{App, Router, Server, Request, Response, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = Router::new()
        .get("/", hello_text)
        .get("/json", hello_json);

    let app = App::new().router(router);
    Server::new(app, "127.0.0.1:3000".parse()?).run().await?;
    Ok(())
}

async fn hello_text(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().text("Hello, Yaiko!"))
}

async fn hello_json(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().json(&json!({
        "message": "Hello, Yaiko!",
        "version": "0.1.0"
    }))?)
}
```

Run it:
```bash
yaiko dev
# Visit http://127.0.0.1:3000
# Visit http://127.0.0.1:3000/json
```

---

## Chapter 9 — Tutorial: REST API (CRUD)

Build a full CRUD API for a Todo resource.

```rust
use yaiko_core::{Router, Request, Response, StatusCode, json, BoxError};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Todo {
    id: u32,
    title: String,
    completed: bool,
}

// GET /api/todos — List all
async fn list_todos(_req: Request) -> Result<Response, BoxError> {
    let todos = vec![
        Todo { id: 1, title: "Learn Yaiko".into(), completed: false },
        Todo { id: 2, title: "Build an app".into(), completed: false },
    ];
    Ok(Response::new().json(&todos)?)
}

// GET /api/todos/:id — Get one
async fn get_todo(req: Request) -> Result<Response, BoxError> {
    let id = req.param("id").unwrap_or("0");
    Ok(Response::new().json(&json!({
        "id": id,
        "title": "Learn Yaiko",
        "completed": false,
    }))?)
}

// POST /api/todos — Create
async fn create_todo(mut req: Request) -> Result<Response, BoxError> {
    let body = req.json().await?;
    let title = body["title"].as_str().unwrap_or("Untitled");
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "id": 3, "title": title, "completed": false }))?)
}

// PUT /api/todos/:id — Full update
async fn update_todo(mut req: Request) -> Result<Response, BoxError> {
    let id = req.param("id").unwrap_or("0").to_string();
    let body = req.json().await?;
    Ok(Response::new().json(&json!({
        "id": id,
        "title": body["title"],
        "completed": body["completed"],
    }))?)
}

// PATCH /api/todos/:id — Partial update
async fn patch_todo(mut req: Request) -> Result<Response, BoxError> {
    let id = req.param("id").unwrap_or("0").to_string();
    let body = req.json().await?;
    Ok(Response::new().json(&json!({ "id": id, "patched": body }))?)
}

// DELETE /api/todos/:id — Delete
async fn delete_todo(_req: Request) -> Result<Response, BoxError> {
    Ok(Response::no_content())
}
```

### Wiring the routes:
```rust
let api = Router::new()
    .get("/todos", list_todos)
    .post("/todos", create_todo)
    .get("/todos/:id", get_todo)
    .put("/todos/:id", update_todo)
    .patch("/todos/:id", patch_todo)
    .delete("/todos/:id", delete_todo);

let router = Router::new().mount("/api", api);
```

### Testing with curl:
```bash
# List todos
curl http://localhost:3000/api/todos

# Create a todo
curl -X POST http://localhost:3000/api/todos \
  -H "Content-Type: application/json" \
  -d '{"title":"Ship the product"}'

# Update a todo
curl -X PUT http://localhost:3000/api/todos/1 \
  -H "Content-Type: application/json" \
  -d '{"title":"Ship it","completed":true}'

# Delete a todo
curl -X DELETE http://localhost:3000/api/todos/1
```

---

## Chapter 10 — Tutorial: Middleware

Yaiko ships built-in middleware and lets you write your own.

### Using Built-in Middleware
```rust
use yaiko_core::{LoggingMiddleware, SecurityHeaders, CompressionMiddleware};
use yaiko_core::security::RateLimiter;

let router = Router::new()
    .get("/", home_handler)
    .use_middleware(LoggingMiddleware::new())
    .use_middleware(SecurityHeaders::new())
    .use_middleware(CompressionMiddleware::new());
```

### Writing Custom Middleware
```rust
use yaiko_core::{Middleware, Handler, Request, Response};
use async_trait::async_trait;
use std::sync::Arc;

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

// Apply it:
let router = Router::new()
    .get("/", home_handler)
    .use_middleware(TimingMiddleware);
```

### Security Headers Added Automatically

| Header | Value |
|---|---|
| `X-Content-Type-Options` | `nosniff` |
| `X-Frame-Options` | `DENY` |
| `X-XSS-Protection` | `1; mode=block` |
| `Referrer-Policy` | `strict-origin-when-cross-origin` |
| `Permissions-Policy` | `geolocation=(), microphone=(), camera=()` |

---

## Chapter 11 — Tutorial: Authentication (JWT)

### Password Hashing
```rust
use yaiko_core::{hash_password, verify_password};

// Register — hash the password before storing
let password_hash = hash_password("user_password")?;

// Login — verify the submitted password
let valid = verify_password("user_password", &password_hash)?;
```

### JWT Token Generation & Verification
```rust
use yaiko_core::auth::JwtAuth;

let jwt = JwtAuth::new("my-secret-key");

// Generate a token with user ID and roles
let token = jwt.generate_token("user-123", vec!["admin".to_string()])?;

// Verify a token
let claims = jwt.verify_token(&token)?;
println!("User: {}, Roles: {:?}", claims.sub, claims.roles);
```

### Login Handler
```rust
async fn login(mut req: Request) -> Result<Response, BoxError> {
    let body = req.json().await?;
    let email = body["email"].as_str().unwrap_or("");
    let password = body["password"].as_str().unwrap_or("");

    // Look up user from DB, verify password
    let stored_hash = hash_password("secret123").unwrap();

    if email == "user@example.com" && verify_password(password, &stored_hash).unwrap_or(false) {
        let auth = JwtAuth::new("my-jwt-secret");
        let token = auth.generate_token("user-1", vec![])?;
        Ok(Response::new().json(&json!({ "token": token }))?)
    } else {
        Ok(Response::new()
            .status(StatusCode::UNAUTHORIZED)
            .json(&json!({ "error": "Invalid credentials" }))?)
    }
}
```

### Protecting Routes with AuthMiddleware
```rust
use yaiko_core::auth::{JwtAuth, AuthMiddleware};
use std::sync::Arc;

let protected = Router::new()
    .get("/profile", profile_handler)
    .get("/settings", settings_handler)
    .use_middleware(AuthMiddleware::new(Arc::new(JwtAuth::new("my-jwt-secret"))));

let router = Router::new()
    .post("/login", login)
    .mount("/auth", protected);  // /auth/profile and /auth/settings are now protected
```

### Accessing the Authenticated User
```rust
async fn profile_handler(req: Request) -> Result<Response, BoxError> {
    let user_id = req.user_id.clone().unwrap_or("unknown".into());
    Ok(Response::new().json(&json!({
        "user_id": user_id,
        "message": "You are authenticated!"
    }))?)
}
```

### Frontend — Sending the Token
```javascript
$.ajax({
    url: '/auth/profile',
    headers: { 'Authorization': 'Bearer ' + localStorage.getItem('token') },
    success: function(data) { console.log(data); }
});
```

---

## Chapter 12 — Tutorial: Sessions & Cookies

### Session Middleware Setup
```rust
use yaiko_core::{MemorySessionStore, SessionMiddleware};
use std::sync::Arc;

let session_store = Arc::new(MemorySessionStore::new());

let router = Router::new()
    .get("/", home_handler)
    .use_middleware(SessionMiddleware::new(session_store).secure(false));
```

### Reading & Writing Session Data
```rust
use yaiko_core::{login_session, logout_session};

// Login — write user ID into the session
async fn login_handler(mut req: Request) -> Result<Response, BoxError> {
    let session = req.session.as_ref().expect("session middleware required");
    login_session(session, "user-123", &vec!["admin".to_string()])?;
    Ok(Response::new().json(&json!({ "message": "Logged in" }))?)
}

// Logout — destroy the session
async fn logout_handler(req: Request) -> Result<Response, BoxError> {
    if let Some(session) = &req.session {
        logout_session(session);
    }
    Ok(Response::new().redirect("/"))
}
```

### Setting Cookies on Responses
```rust
async fn preferences(req: Request) -> Result<Response, BoxError> {
    Ok(Response::new()
        .set_cookie("theme", "dark")
        .set_cookie("lang", "en")
        .json(&json!({ "message": "Preferences saved" }))?)
}
```

---

## Chapter 13 — Tutorial: Database & Models

### Database Setup
```bash
# .env for SQLite
DATABASE_URL=sqlite:./data.db?mode=rwc

# .env for PostgreSQL
DATABASE_URL=postgres://user:password@localhost:5432/myapp
```

### Creating Migrations
```bash
yaiko migrate create users
```

Edit `migrations/YYYYMMDD_users.sql`:
```sql
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_users_email ON users(email);
```

```bash
yaiko migrate run
```

### Defining a Model
```bash
yaiko generate model user
```

`src/models/user.rs`:
```rust
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub password_hash: String,
    pub name: Option<String>,
}

impl User {
    pub async fn all(pool: &sqlx::PgPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users ORDER BY id DESC")
            .fetch_all(pool)
            .await
    }

    pub async fn find(pool: &sqlx::PgPool, id: i32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_email(pool: &sqlx::PgPool, email: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(pool: &sqlx::PgPool, email: &str, password_hash: &str, name: Option<&str>) -> Result<Self, sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO users (email, password_hash, name) VALUES ($1, $2, $3) RETURNING *"
        )
        .bind(email)
        .bind(password_hash)
        .bind(name)
        .fetch_one(pool)
        .await
    }

    pub async fn delete(pool: &sqlx::PgPool, id: i32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(pool).await?;
        Ok(())
    }
}
```

### Using the Model in Controllers
```rust
pub async fn list_users(req: Request, pool: sqlx::PgPool) -> Result<Response, BoxError> {
    let users = User::all(&pool).await?;
    Ok(Response::new().json(&json!({ "users": users }))?)
}
```

---

## Chapter 14 — Tutorial: Form Validation

Yaiko includes a declarative validation system.

```rust
use yaiko_core::validation::{Validator, Required, MinLength, MaxLength, Email};

async fn register(mut req: Request) -> Result<Response, BoxError> {
    let form = req.form_data().await?;

    let validator = Validator::new()
        .add_rule("name", Required)
        .add_rule("name", MinLength(2))
        .add_rule("name", MaxLength(100))
        .add_rule("email", Required)
        .add_rule("email", Email)
        .add_rule("password", Required)
        .add_rule("password", MinLength(8));

    if let Err(errors) = validator.validate(&form) {
        return Ok(Response::new()
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .json(&json!({ "errors": errors }))?);
    }

    let password_hash = hash_password(form.get("password").unwrap())?;
    // Insert into database...

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "message": "Account created" }))?)
}
```

### Available Rules

| Rule | Description |
|---|---|
| `Required` | Field must be present and non-empty |
| `Email` | Must be a valid email format |
| `MinLength(n)` | Minimum character count |
| `MaxLength(n)` | Maximum character count |

---

## Chapter 15 — Tutorial: File Uploads

### Multipart Upload Handling
```rust
use yaiko_core::file_upload::parse_multipart;

async fn upload_avatar(mut req: Request) -> Result<Response, BoxError> {
    let parts = parse_multipart(&mut req).await?;

    // Access text fields
    if let Some(description) = parts.fields.get("description") {
        println!("Description: {}", description);
    }

    // Access uploaded files
    if let Some(file) = parts.files.get("avatar") {
        let save_path = format!("./public/uploads/{}", file.filename);
        std::fs::write(&save_path, &file.content)?;
        return Ok(Response::new().json(&json!({
            "filename": file.filename,
            "size": file.content.len(),
            "url": format!("/uploads/{}", file.filename),
        }))?);
    }

    Ok(Response::new()
        .status(StatusCode::BAD_REQUEST)
        .json(&json!({ "error": "No file uploaded" }))?)
}
```

### Frontend Upload Form
```html
<form id="upload-form">
    <input type="file" id="avatar" accept="image/*">
    <input type="text" id="description" placeholder="Description">
    <button type="submit">Upload</button>
</form>

<script>
$('#upload-form').on('submit', function(e) {
    e.preventDefault();
    var formData = new FormData();
    formData.append('avatar', $('#avatar')[0].files[0]);
    formData.append('description', $('#description').val());

    $.ajax({
        url: '/api/upload',
        method: 'POST',
        data: formData,
        processData: false,
        contentType: false,
        success: function(res) { alert('Uploaded: ' + res.url); }
    });
});
</script>
```

---

## Chapter 16 — Tutorial: WebSockets

Real-time messaging via the `WebSocketManager`.

### Server-Side Setup
```rust
use yaiko_core::{
    WebSocketManager, is_websocket_upgrade,
    websocket::{handle_websocket_upgrade, WsMessage},
};
use std::sync::Arc;

let ws_manager = Arc::new(WebSocketManager::new());

async fn ws_handler(req: Request) -> Result<Response, BoxError> {
    let manager = req.app_data::<Arc<WebSocketManager>>().unwrap().clone();

    // Upgrade the HTTP connection to WebSocket
    let (response, conn_id, rx) = handle_websocket_upgrade(&req, manager.clone(), None).await?;

    // Put the user in a room
    manager.join_room(&conn_id, "general").await;

    // Announce arrival
    manager.send_to_room("general",
        json!({ "system": "A new user joined!" }).to_string()
    ).await;

    Ok(response)
}
```

### Client-Side (jQuery)
```javascript
const ws = new WebSocket("ws://127.0.0.1:3000/ws");

ws.onopen = function() {
    console.log("Connected!");
};

ws.onmessage = function(event) {
    const data = JSON.parse(event.data);
    if (data.system) {
        $('#messages').append('<div class="system">' + data.system + '</div>');
    } else if (data.text) {
        $('#messages').append('<div class="msg">' + data.text + '</div>');
    }
};

$('#chat-form').submit(function(e) {
    e.preventDefault();
    const text = $('#msg-input').val().trim();
    if (text) {
        ws.send(JSON.stringify({ text: text }));
        $('#msg-input').val('');
    }
});
```

### WebSocket Manager API

| Method | Description |
|---|---|
| `manager.send(&conn_id, msg)` | Send to a specific connection |
| `manager.send_to_room("room", msg)` | Broadcast to all connections in a room |
| `manager.broadcast(msg)` | Send to every connected client |
| `manager.broadcast_json(&value)` | Broadcast a JSON value |
| `manager.join_room(&conn_id, "room")` | Add a connection to a named room |
| `manager.leave_room(&conn_id, "room")` | Remove a connection from a room |
| `manager.check_rate_limit(&conn_id)` | Returns `true` if under the rate limit |

---

## Chapter 17 — Tutorial: Background Jobs

Offload long-running work to the async `JobQueue`.

```rust
use yaiko_core::JobQueue;
use std::sync::Arc;

let queue = Arc::new(JobQueue::new());

// Start the job queue processor in the background
let q = queue.clone();
tokio::spawn(async move { let _ = q.start().await; });

// Enqueue a job — it retries with exponential backoff on failure
queue.add("send_welcome_email", || async {
    // Simulate sending an email
    tracing::info!("Sending welcome email...");
    // reqwest::Client::new().post("...").send().await?;
    Ok(())
}).await;

queue.add("generate_thumbnail", || async {
    tracing::info!("Generating thumbnail...");
    Ok(())
}).await;
```

**Use cases:**
- Sending emails after user registration
- Resizing uploaded images
- Processing webhook payloads
- Generating PDF reports
- Scraping link preview metadata

---

## Chapter 18 — Tutorial: Server-Sent Events (SSE)

Stream real-time data to clients over HTTP using `Response::event_stream()`.

```rust
async fn sse_handler(_req: Request) -> Result<Response, BoxError> {
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(16);

    tokio::spawn(async move {
        for i in 1..=10 {
            let msg = serde_json::to_string(&json!({ "count": i })).unwrap();
            if tx.send(msg).await.is_err() { break; }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });

    Ok(Response::new().event_stream(rx))
}
```

### Client-Side
```javascript
const source = new EventSource('/events');
source.onmessage = function(event) {
    const data = JSON.parse(event.data);
    console.log('Count:', data.count);
};
```

---

## Chapter 19 — Tutorial: Response Types

Yaiko's `Response` builder supports many output formats.

### JSON
```rust
Ok(Response::new().json(&json!({ "status": "ok" }))?)
```

### HTML
```rust
Ok(Response::new().html("<h1>Hello</h1><p>Server-rendered page.</p>"))
```

### Plain Text
```rust
Ok(Response::new().text("Hello, World!"))
```

### Redirect (302)
```rust
Ok(Response::new().redirect("/login"))
```

### Permanent Redirect (301)
```rust
Ok(Response::new().redirect_permanent("/new-url"))
```

### No Content (204)
```rust
Ok(Response::no_content())
```

### Custom Status + Headers
```rust
Ok(Response::new()
    .status(StatusCode::NOT_FOUND)
    .header("X-Custom", "value")
    .json(&json!({ "error": "Not found" }))?)
```

### Cookies
```rust
Ok(Response::new()
    .set_cookie("theme", "dark")
    .set_cookie("lang", "en")
    .text("Cookies set!"))
```

### Streaming (Chunked Transfer)
```rust
Ok(Response::new().stream(byte_stream))
```

---

## Chapter 20 — Tutorial: Request Helpers

### Inspect Request Properties
```rust
async fn inspect(req: Request) -> Result<Response, BoxError> {
    Ok(Response::new().json(&json!({
        "is_json": req.is_json(),       // Content-Type: application/json?
        "is_ajax": req.is_ajax(),       // X-Requested-With: XMLHttpRequest?
        "content_type": req.header("content-type"),
        "user_agent": req.header("user-agent"),
    }))?)
}
```

### Path Parameters
```rust
// Route: /users/:id/posts/:post_id
let user_id = req.param("id").unwrap();
let post_id = req.param("post_id").unwrap();
```

### Query Parameters
```rust
// URL: /search?q=rust&page=2
let query = req.query("q").unwrap_or("".into());
let page = req.query("page").unwrap_or("1".into());
```

### JSON Body
```rust
let body = req.json().await?;
let name = body["name"].as_str().unwrap_or("");
```

### Form Data (URL-encoded)
```rust
let form = req.form_data().await?;
let email = form.get("email").cloned().unwrap_or_default();
```

### Raw Body Bytes
```rust
let bytes = req.body_bytes().await?;
```

---

## Chapter 21 — Tutorial: Testing (TestClient)

Yaiko includes a powerful `TestClient` that routes HTTP calls in-memory — no server port needed.

```rust
use yaiko_core::{TestClient, json};

#[tokio::test]
async fn test_hello() {
    let client = TestClient::new(build_router());
    let res = client.get("/").await;
    res.assert_status(200);
    res.assert_body_contains("Hello, Yaiko!");
}

#[tokio::test]
async fn test_create_todo() {
    let client = TestClient::new(build_router());
    let res = client.post("/api/todos", r#"{"title":"Write tests"}"#).await;
    res.assert_status(201);
    res.assert_body_contains("Write tests");
}

#[tokio::test]
async fn test_delete_returns_204() {
    let client = TestClient::new(build_router());
    let res = client.delete("/api/todos/1").await;
    res.assert_status(204);
}

#[tokio::test]
async fn test_with_auth() {
    let client = TestClient::new(build_router())
        .with_auth("my-jwt-token")
        .with_header("x-custom", "value");
    let res = client.get("/auth/profile").await;
    res.assert_status(200);
}

#[tokio::test]
async fn test_form_submission() {
    let client = TestClient::new(build_router());
    let mut form = std::collections::HashMap::new();
    form.insert("name".into(), "Alice".into());
    form.insert("email".into(), "alice@example.com".into());
    form.insert("password".into(), "secure_password".into());
    let res = client.post_form("/register", &form).await;
    res.assert_status(201);
}
```

### TestClient API

| Method | Description |
|---|---|
| `client.get("/path")` | Send GET request |
| `client.post("/path", body)` | Send POST with JSON body |
| `client.put("/path", body)` | Send PUT with JSON body |
| `client.patch("/path", body)` | Send PATCH with JSON body |
| `client.delete("/path")` | Send DELETE request |
| `client.post_form("/path", &map)` | Send URL-encoded form POST |
| `.with_auth("token")` | Attach `Authorization: Bearer` header |
| `.with_header("key", "val")` | Attach custom header |

| Assertion | Description |
|---|---|
| `res.assert_status(200)` | Verify HTTP status code |
| `res.assert_body_contains("text")` | Check response body substring |
| `res.json::<T>()` | Deserialize body as JSON |

### Running all tutorial tests:
```bash
cd examples/tutorials
cargo test
```

---

## Chapter 22 — Tutorial: Configuration

### `yaiko.toml`
```toml
[server]
host = "127.0.0.1"
port = 3000

[database]
db_type = "sqlite"
url = ""

[security]
cors_origins = ["http://localhost:3000"]
rate_limit_requests = 100
rate_limit_window_secs = 60
csrf_enabled = true

[seo]
robots_txt_enabled = true
sitemap_enabled = true
sitemap_changefreq = "weekly"

[logging]
level = "info"
format = "pretty"
```

### `.env` (takes precedence)
```bash
HOST=127.0.0.1
PORT=3000
DATABASE_URL=sqlite:./data.db?mode=rwc
JWT_SECRET=change-me-in-production
RUST_LOG=info
SITE_URL=http://localhost:3000
```

### Loading Settings in Code
```rust
let settings = yaiko_core::Settings::load()?;
let addr = format!("{}:{}", settings.server.host, settings.server.port);
```

---

## Chapter 23 — Tutorial: SEO (robots.txt & sitemap.xml)

Yaiko automatically generates `/robots.txt` and `/sitemap.xml` for every app. If your routes don't define them, the framework creates default versions using your `SITE_URL`.

### Custom Robots & Sitemap
```rust
use yaiko_core::seo::{RobotsTxt, Sitemap, SitemapUrl};

let robots = RobotsTxt::new()
    .disallow("/admin")
    .disallow("/api")
    .sitemap("https://myapp.com/sitemap.xml");

let sitemap = Sitemap::new("https://myapp.com")
    .add(SitemapUrl::new("/").priority(1.0).changefreq("daily"))
    .add(SitemapUrl::new("/about").priority(0.8))
    .add(SitemapUrl::new("/blog").priority(0.9).changefreq("weekly"));

let router = Router::new()
    .get("/robots.txt", robots.handler())
    .get("/sitemap.xml", sitemap.handler());
```

---

## Chapter 24 — Deployment to Production

### Build for Release
```bash
yaiko build --release
```

### Deploy to VPS

```bash
# Copy binary + assets to your server
scp target/release/myapp root@your-server:/opt/yaiko/
scp -r public root@your-server:/opt/yaiko/
scp .env.production root@your-server:/opt/yaiko/.env
```

### Systemd Service (`/etc/systemd/system/yaiko.service`)
```ini
[Unit]
Description=Yaiko Application
After=network.target

[Service]
Type=simple
User=yaiko
WorkingDirectory=/opt/yaiko
EnvironmentFile=/opt/yaiko/.env
ExecStart=/opt/yaiko/myapp
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
systemctl daemon-reload
systemctl enable yaiko
systemctl start yaiko
```

### Nginx Reverse Proxy
```nginx
server {
    listen 80;
    server_name myapp.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }

    location /static/ {
        alias /opt/yaiko/public/;
        expires 30d;
    }
}
```

### SSL with Certbot
```bash
certbot --nginx -d myapp.com
```

### Production Checklist

- [ ] Build with `--release`
- [ ] Set `RUST_LOG=warn`
- [ ] Strong `JWT_SECRET` (not the default)
- [ ] HTTPS via Certbot
- [ ] Firewall (`ufw allow 22,80,443`)
- [ ] Set up log rotation
- [ ] Database backups

---

## Chapter 25 — End-to-End Tutorial: Building TeamPulse

In this chapter, we build **TeamPulse** — a real-time team chat application with WebSockets.

### Step 1: Scaffold the Project
```bash
yaiko init teampulse --database sqlite
cd teampulse
```

### Step 2: Implement the WebSocket Chat Server

`src/main.rs`:
```rust
use yaiko_core::{
    App, Router, Server, Request, Response, BoxError, json,
    WebSocketManager, is_websocket_upgrade, websocket::{handle_websocket_upgrade, WsMessage},
};
use std::sync::Arc;
use tokio::sync::RwLock;

struct AppState {
    ws_manager: Arc<WebSocketManager>,
    user_count: Arc<RwLock<usize>>,
}

async fn chat_ws_handler(req: Request) -> Result<Response, BoxError> {
    let state = req.app_data::<AppState>().unwrap();
    let manager = state.ws_manager.clone();

    let (response, conn_id, mut rx) = handle_websocket_upgrade(&req, manager.clone(), None).await?;
    manager.join_room(&conn_id, "general").await;

    let count = {
        let mut c = state.user_count.write().await;
        *c += 1;
        *c
    };
    manager.send_to_room("general",
        json!({ "system": format!("User joined. {} total.", count) }).to_string()
    ).await;

    Ok(response)
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    yaiko_core::init_tracing();

    let state = Arc::new(AppState {
        ws_manager: Arc::new(WebSocketManager::new()),
        user_count: Arc::new(RwLock::new(0)),
    });

    let router = Router::new()
        .static_files("/", "./public")
        .get("/ws", chat_ws_handler);

    let mut app = App::new().router(router);
    app.data(state);

    Server::new(app, "127.0.0.1:3000".parse()?).run().await?;
    Ok(())
}
```

### Step 3: Create the Frontend

`public/index.html`:
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>TeamPulse Chat</title>
    <style>
        body { font-family: -apple-system, sans-serif; background: #f0f2f5; margin: 0; padding: 20px; }
        .chat-box { max-width: 600px; margin: 0 auto; background: white; border-radius: 8px;
                     box-shadow: 0 4px 12px rgba(0,0,0,0.1); display: flex; flex-direction: column; height: 80vh; }
        .messages { flex: 1; padding: 20px; overflow-y: auto; border-bottom: 1px solid #ddd; }
        .msg { padding: 8px 12px; margin-bottom: 10px; border-radius: 6px; background: #e9ecef;
               width: fit-content; max-width: 80%; }
        .msg.system { background: #fff3cd; color: #856404; margin: 10px auto; text-align: center; }
        .compose { display: flex; padding: 15px; }
        input { flex: 1; padding: 10px; border: 1px solid #ced4da; border-radius: 4px; }
        button { padding: 10px 20px; margin-left: 10px; background: #0d6efd; color: white;
                 border: none; border-radius: 4px; cursor: pointer; }
    </style>
</head>
<body>
<div class="chat-box">
    <div class="messages" id="messages"></div>
    <form class="compose" id="chat-form">
        <input type="text" id="msg-input" placeholder="Type a message..." autocomplete="off" required>
        <button type="submit">Send</button>
    </form>
</div>
<script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
<script>
$(document).ready(function() {
    const ws = new WebSocket("ws://127.0.0.1:3000/ws");

    ws.onopen = function() { appendMessage({ system: "Connected to TeamPulse!" }); };
    ws.onmessage = function(event) {
        try { appendMessage(JSON.parse(event.data)); } catch(e) {}
    };
    ws.onclose = function() { appendMessage({ system: "Disconnected." }); };

    $('#chat-form').submit(function(e) {
        e.preventDefault();
        const text = $('#msg-input').val().trim();
        if (text) { ws.send(JSON.stringify({ text: text })); $('#msg-input').val(''); }
    });

    function appendMessage(data) {
        const box = $('#messages');
        if (data.system) box.append('<div class="msg system">' + data.system + '</div>');
        else if (data.text) box.append('<div class="msg">' + data.text + '</div>');
        box.scrollTop(box[0].scrollHeight);
    }
});
</script>
</body>
</html>
```

### Step 4: Run It
```bash
yaiko dev
```

Open two browser tabs at `http://127.0.0.1:3000` and chat between them!

---

## Chapter 26 — Live Production Example: ImgHost

A complete production image hosting platform deployed at [imghost.se](https://imghost.se).

**Source code:** [`examples/imghost`](https://github.com/sazalo101/yaiko/tree/main/examples/imghost)

### Features Demonstrated
- Multipart image upload with drag-and-drop UI
- JigsawStack AI NSFW content moderation
- SQLite with WAL mode for high-concurrency reads
- View counter with `tokio::spawn` fire-and-forget writes
- Delete tokens for secure image removal
- Automatic `/robots.txt` and `/sitemap.xml`

### Benchmark Results
- **11,234 req/s** raw engine speed
- **3,053 req/s** through Nginx + TLS
- **12,042 req/s** under 1,000 concurrent connections stress test
- **Zero crashes, zero timeouts** across 362,000+ requests

---

## Appendix — Quick Reference Card

```rust
// === ROUTING ===
Router::new()
    .get("/path", handler)
    .post("/path", handler)
    .put("/path/:id", handler)
    .patch("/path/:id", handler)
    .delete("/path/:id", handler)
    .mount("/prefix", sub_router)
    .static_files("/static", "./public")
    .use_middleware(MyMiddleware)

// === REQUEST ===
req.param("id")              // Path parameter
req.query("page")            // Query parameter
req.json().await?            // Parse JSON body
req.form_data().await?       // Parse form body
req.body_bytes().await?      // Raw bytes
req.header("content-type")   // Header value
req.is_json()                // Content-Type check
req.is_ajax()                // XHR check

// === RESPONSE ===
Response::new().json(&value)?     // JSON
Response::new().html("<h1>Hi</h1>") // HTML
Response::new().text("hello")     // Plain text
Response::new().redirect("/url")  // 302 redirect
Response::no_content()            // 204
Response::new().set_cookie("k","v") // Set cookie
Response::new().event_stream(rx)  // SSE stream
Response::new().stream(bytes)     // Chunked stream
Response::new().status(StatusCode::CREATED)
Response::new().header("Key", "Value")

// === AUTH ===
hash_password("pass")?
verify_password("pass", &hash)?
JwtAuth::new("secret").generate_token("user-id", roles)?
JwtAuth::new("secret").verify_token(&token)?
AuthMiddleware::new(jwt)

// === VALIDATION ===
Validator::new()
    .add_rule("field", Required)
    .add_rule("field", Email)
    .add_rule("field", MinLength(8))
    .add_rule("field", MaxLength(255))
    .validate(&data)?

// === TESTING ===
TestClient::new(router)
    .with_auth("token")
    .get("/path").await
    .assert_status(200)
    .assert_body_contains("text")
```


---

## Chapter 27 — The Built-in Module Catalog

Yaiko’s current architecture is organized around small, typed policy and domain facades. They are designed to be composed with the router, handlers, templates, SQLx database layer, WebSockets, and background workers rather than replacing application-specific architecture.

### Catalog by Area

| Area | Current built-ins |
|---|---|
| Runtime and routing | `network_endpoint`, `fs_router`, `router`, `worker` |
| Frontend primitives | `image`, `font`, `form`, `link`, `head`, `metadata`, `script`, `style`, `icon`, `static_asset` |
| Data | `query_client`, `serialize`, `transaction`, `pool`, `data_transfer` |
| Security | `rbac`, `auth`, `cors`, `csp`, `rate_limit`, `media_access` |
| API and realtime | `api_facade`, `rpc`, `proxy`, `event_bus`, `pubsub`, `webhook`, `openapi` |
| Developer tooling | `watch`, `hmr`, `typegen`, `test_facade`, `lint_policy`, `format_policy` |
| Observability | `log_facade`, `health_facade`, `metrics`, `tracing_context`, `audit`, `error` |
| Deployment and utilities | `deploy`, `compression_policy`, `url`, `robots`, `feed`, `i18n`, `sitemap` |
| Media editor | timeline, processing, annotations, reviews, asset versions, project templates, export presets, delivery policies, `MediaEditorRepository` | Captions, background music, bounded editing policies, signed delivery, and optional SQLx SQLite project persistence. |

### Design Rules

The built-ins share several production-oriented rules. Inputs are bounded, paths and URLs are validated, sensitive fields are redacted where appropriate, deterministic ordering is used for snapshots and generated output, and invalid state transitions return structured errors. The facades are deliberately explicit about policy decisions such as cache capacity, retry limits, TTLs, deployment environment requirements, and reload behavior.

### Media Editor Boundary

The media modules now cover substantial editor-domain behavior: captions, background music, timeline composition, thumbnails and metadata, immutable asset versions, project templates, annotations, review decisions, presence, cursors, selection locks, export presets, signed access, quotas, retention, resumable uploads, and progress events.

These modules are not presented as a complete hosted editor by themselves. The `persistent-media` feature now provides a SQLx-backed SQLite `MediaEditorRepository` for project scope, optimistic revisions, ordered assets, and timelines. The remaining application boundary consists of HTTP and WebSocket handlers, controlled FFmpeg workers, signed artifact delivery, browser timeline components, upload UX, and end-to-end integration tests. Keeping this boundary explicit makes the framework reusable for both ordinary web applications and specialized video workflows.

### CLI-First Example Workflow

Yaiko examples are Yaiko projects: each example contains a `Cargo.toml` and a `yaiko.toml`. Run the CLI from the example directory rather than invoking Cargo with a manifest path.

```bash
# Install the local CLI once from the repository root.
cargo install --path yaiko-cli --force

# Run the HTTP catalog example.
cd examples/catalog
yaiko doctor
yaiko dev

# In another terminal, build it for production.
cd examples/catalog
yaiko build --release
```

The same workflow applies to the other new examples:

| Example | Command sequence | Purpose |
|---|---|---|
| `catalog` | `cd examples/catalog && yaiko doctor && yaiko dev` | Router, health, head, metadata, and JSON |
| `media-studio` | `cd examples/media-studio && yaiko doctor && yaiko build && yaiko run` | SQLite-backed media project persistence |
| `webhook-inbox` | `cd examples/webhook-inbox && yaiko doctor && yaiko build && yaiko run` | Signed webhook verification and replay protection |

Use `yaiko build --release` in an example directory for the optimized production binary. Use `yaiko dev` for long-running HTTP applications with the development watcher, and `yaiko run` for command-line examples that should build and execute once.

### Verification and Release Discipline

New built-in batches follow a consistent release process: focused module tests; formatting; strict Clippy; diff checks; SQLite, PostgreSQL, metrics, and development feature combinations; CLI tests; and builds of the blog, chat, and auth examples. The verified history is recorded in the repository README, while the complete catalog is maintained in [the built-in module guide](built-in-modules.md).

### Example Composition

A typical application composes these layers rather than using every module at once:

```rust
use yaiko_core::{
    App, Head, Link, LogFacade, QueryClient, Router, SecurityHeaders,
};

// Configure the application boundary first.
let mut logs = LogFacade::new(1_000);
let queries = QueryClient::new(256);
let head = Head::new().title("Example application");
let home = Link::new("/", "Home")?;
let router = Router::new().get("/", home_handler);
let _app = App::new()
    .router(router)
    .middleware(SecurityHeaders::new());
```

The exact composition depends on whether the project is a static site, API, realtime application, or media editor. The examples directory demonstrates complete applications, while the built-in module guide demonstrates isolated policies.
