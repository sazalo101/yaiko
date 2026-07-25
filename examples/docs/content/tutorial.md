# Step-by-Step Tutorial: Building a Fullstack Task Manager with Yaiko

Welcome to the official **Yaiko Framework Tutorial**! In this guide, you will build a production-ready, fullstack **Task Manager** application from scratch using **Rust** on the backend and **jQuery** on the frontend.

---

## 📋 What You Will Learn

1. **CLI Scaffolding**: Create a new app with `yaiko init`.
2. **Environment & Diagnostics**: Use `yaiko doctor` to verify system setup.
3. **Hot-Reloading Development**: Use `yaiko dev` for real-time development.
4. **Controllers & Models**: Generate code modules with `yaiko generate`.
5. **Database & Migrations**: Work with SQLite/PostgreSQL migrations (`yaiko migrate`).
6. **Frontend UI**: Serve HTML templates and handle interactivity with jQuery.
7. **Automated Testing**: Write integration tests with `yaiko_core::TestClient`.
8. **Production Deployment**: Compile optimized release binaries with `yaiko build --release`.

---

## Prerequisites

- **Rust 1.70+**:
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Git**

---

## Step 1: Install the Yaiko CLI

Clone the repository and install the CLI globally:

```bash
# Clone the repository
git clone https://github.com/sazalo101/yaiko.git
cd yaiko

# Install CLI globally
cargo install --path ./yaiko-cli --force

# Verify CLI installation & environment readiness
yaiko doctor
```

---

## Step 2: Initialize Your Project

Create a new application named `taskmanager` with SQLite database support:

```bash
yaiko init taskmanager --database sqlite
cd taskmanager
```

### Directory Structure Created:

```
taskmanager/
├── src/
│   ├── main.rs              # Application entry point & router
│   ├── controllers/         # Route handlers
│   │   └── users.rs
│   ├── models/              # Data models & DB logic
│   └── middleware/          # Custom request middleware
├── public/
│   ├── css/main.css         # Modern glassmorphism styles
│   └── js/
│       ├── core.js          # Yaiko UI & Toast utilities
│       └── app.js           # Frontend interactivity
├── templates/
│   └── index.html           # Landing page HTML
├── migrations/              # SQL migration files
├── yaiko.toml               # Framework configuration
├── .env                     # Environment variables
└── Cargo.toml               # Rust dependencies
```

---

## Step 3: Start the Hot-Reloading Development Server

Launch the development server:

```bash
yaiko dev
```

Output:
```
⚡ Starting Yaiko dev server at http://127.0.0.1:3000...
[info] Watching src/, templates/, public/ for changes...
```

Open [http://127.0.0.1:3000](http://127.0.0.1:3000) in your browser. You will see the default Yaiko welcome page!

---

## Step 4: Create a Database Migration

Stop `yaiko dev` (press `Ctrl+C`) and generate a migration for the `tasks` table:

```bash
yaiko migrate create tasks
```

This creates a file in `migrations/YYYYMMDD_tasks.sql`. Add the following SQL schema:

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    completed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO tasks (title, completed) VALUES ('Learn Yaiko Framework', 1);
INSERT INTO tasks (title, completed) VALUES ('Build Task Manager App', 0);
```

Run the database migration:

```bash
yaiko migrate run
```

---

## Step 5: Generate Task Controller & Model

Use `yaiko generate` to scaffold the backend modules:

```bash
yaiko generate model task
yaiko generate controller tasks
```

### 1. Update `src/models/task.rs`:

```rust
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub completed: bool,
}

impl Task {
    pub async fn all(pool: &SqlitePool) -> Result<Vec<Task>, sqlx::Error> {
        sqlx::query_as::<_, Task>("SELECT id, title, completed FROM tasks ORDER BY id DESC")
            .fetch_all(pool)
            .await
    }

    pub async fn create(pool: &SqlitePool, title: &str) -> Result<Task, sqlx::Error> {
        let id = sqlx::query("INSERT INTO tasks (title, completed) VALUES (?, FALSE)")
            .bind(title)
            .execute(pool)
            .await?
            .last_insert_rowid();

        Ok(Task {
            id,
            title: title.to_string(),
            completed: false,
        })
    }
}
```

### 2. Update `src/controllers/tasks.rs`:

```rust
use yaiko_core::{Request, Response, StatusCode, json, BoxError};
use sqlx::SqlitePool;
use crate::models::task::Task;

pub async fn list(req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let tasks = Task::all(&pool).await?;
    Ok(Response::new().json(&json!({ "tasks": tasks }))?)
}

pub async fn create(mut req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let body = req.json().await?;
    let title = body["title"].as_str().unwrap_or("").trim();

    if title.is_empty() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({ "error": "Title cannot be empty" }))?);
    }

    let task = Task::create(&pool, title).await?;
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "task": task }))?)
}
```

---

## Step 6: Connect Routes in `src/main.rs`

Open `src/main.rs` and update your router and database pool connection:

```rust
use yaiko_core::{App, Router, Server, Request, Response, Settings, BoxError, LoggingMiddleware};
use sqlx::sqlite::SqlitePoolOptions;
use std::net::SocketAddr;

mod controllers;
mod models;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    dotenvy::dotenv().ok();
    let settings = Settings::load()?;
    yaiko_core::init_tracing();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data.db?mode=rwc".into());
    let pool = SqlitePoolOptions::new().max_connections(5).connect(&db_url).await?;

    let p1 = pool.clone();
    let p2 = pool.clone();

    let api = Router::new()
        .get("/tasks", move |req| controllers::tasks::list(req, p1.clone()))
        .post("/tasks", move |req| controllers::tasks::create(req, p2.clone()));

    let router = Router::new()
        .mount("/api", api)
        .static_files("/static", "./public")
        .get("/", home_handler)
        .use_middleware(LoggingMiddleware::new());

    let app = App::new().router(router);

    let addr: SocketAddr = format!("{}:{}", settings.server.host, settings.server.port).parse()?;
    println!("⚡ TaskManager running at http://{}", addr);
    Server::new(app, addr).run().await?;
    Ok(())
}

async fn home_handler(_req: Request) -> Result<Response, BoxError> {
    let html = include_str!("../templates/index.html");
    Ok(Response::new().html(html))
}
```

---

## Step 7: Build Interactive Frontend (`templates/index.html` & `public/js/app.js`)

### 1. Update `templates/index.html`:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TaskManager — Built with Yaiko</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
    <div class="app">
        <header class="header">
            <h2>⚡ TaskManager</h2>
        </header>
        <main class="main" style="max-width: 600px; margin: 2rem auto;">
            <div class="glass" style="padding: 2rem; border-radius: 12px;">
                <h3>Add New Task</h3>
                <form id="task-form" style="display: flex; gap: 10px; margin-top: 1rem;">
                    <input type="text" id="task-input" placeholder="What needs to be done?" style="flex: 1; padding: 10px; border-radius: 6px; border: 1px solid #ccc;">
                    <button type="submit" class="btn btn--primary">Add Task</button>
                </form>
                
                <h3 style="margin-top: 2rem;">Tasks</h3>
                <ul id="task-list" style="list-style: none; padding: 0; margin-top: 1rem;"></ul>
            </div>
        </main>
    </div>

    <div id="toast-container"></div>
    <script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
    <script src="/static/js/core.js"></script>
    <script src="/static/js/app.js"></script>
</body>
</html>
```

### 2. Update `public/js/app.js`:

```javascript
$(document).ready(function() {
    function loadTasks() {
        $.get('/api/tasks', function(res) {
            let list = $('#task-list');
            list.empty();
            if (res.tasks.length === 0) {
                list.append('<li style="color: #888;">No tasks found. Create one above!</li>');
                return;
            }
            res.tasks.forEach(function(task) {
                list.append(
                    '<li style="padding: 10px; border-bottom: 1px solid #eee; display: flex; justify-content: space-between;">' +
                    '<span>' + task.title + '</span>' +
                    '<span style="color: green;">✓</span>' +
                    '</li>'
                );
            });
        });
    }

    $('#task-form').on('submit', function(e) {
        e.preventDefault();
        let title = $('#task-input').val().trim();
        if (!title) return;

        $.ajax({
            url: '/api/tasks',
            method: 'POST',
            contentType: 'application/json',
            data: JSON.stringify({ title: title }),
            success: function(res) {
                Yaiko.ui.toast('Task added successfully! 🚀', 'success');
                $('#task-input').val('');
                loadTasks();
            },
            error: function(err) {
                Yaiko.ui.toast('Failed to add task.', 'error');
            }
        });
    });

    loadTasks();
});
```

---

## Step 8: Test Your Application

Run your dev server:

```bash
yaiko dev
```

- Open [http://127.0.0.1:3000](http://127.0.0.1:3000).
- Enter a task like `"Deploy Yaiko app to production"` and click **Add Task**.
- Notice the instant task insertion and toast notification!

---

## Step 9: Write Integration Tests

Create `tests/integration_test.rs`:

```rust
use yaiko_core::{TestClient, json};
use crate::build_router; // extract router constructor

#[tokio::test]
async fn test_tasks_api() {
    let client = TestClient::new(build_router());

    // Test GET /api/tasks
    let res = client.get("/api/tasks").await;
    res.assert_status(200);

    // Test POST /api/tasks
    let res = client.post("/api/tasks", r#"{"title":"Test task"}"#).await;
    res.assert_status(201);
    res.assert_body_contains("Test task");
}
```

Run tests:

```bash
cargo test
```

---

## Step 10: Production Release Build

Compile your app for maximum performance:

```bash
yaiko build --release
```

The optimized binary is located at `./target/release/taskmanager`.

```bash
./target/release/taskmanager
```

Your fullstack Yaiko app is ready for production! 🚀
