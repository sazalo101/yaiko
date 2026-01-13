# Tutorial: Building an App with Yaiko

This tutorial walks you through building a complete web application with Yaiko, from installation to deployment.

## Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Git

## Step 1: Install Yaiko CLI

```bash
# Clone the repository
git clone https://github.com/yaiko/yaiko.git
cd yaiko

# Install the CLI
cargo install --path ./yaiko-cli

# Verify
yaiko --version
```

## Step 2: Create a New Project

```bash
yaiko init myapp
cd myapp
```

This creates:
```
myapp/
├── src/
│   ├── main.rs
│   ├── controllers/
│   ├── models/
│   └── middleware/
├── public/css/ & js/
├── templates/
├── Cargo.toml
├── yaiko.toml
└── .env
```

## Step 3: Explore the Structure

### main.rs
```rust
use yaiko_core::{App, Router, Server, Request, Response};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = Router::new()
        .get("/", home_handler)
        .get("/api/items", list_items);

    let app = App::new()
        .router(router)
        .static_files("./public", "/static");

    let server = Server::new(app, "127.0.0.1:3000".parse()?);
    server.run().await?;
    Ok(())
}

async fn home_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().html("<h1>Hello Yaiko!</h1>"))
}

async fn list_items(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().json(&serde_json::json!({
        "items": ["Item 1", "Item 2", "Item 3"]
    }))?)
}
```

## Step 4: Run Development Server

```bash
yaiko dev
```

Open http://localhost:3000. The server watches for changes and auto-reloads.

## Step 5: Add a Controller

```bash
yaiko generate controller tasks
```

Edit `src/controllers/tasks.rs`:
```rust
use yaiko_core::{Request, Response, StatusCode, json};

pub async fn list(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().json(&json!({
        "tasks": [
            {"id": 1, "title": "Learn Yaiko", "done": false},
            {"id": 2, "title": "Build an app", "done": false}
        ]
    }))?)
}

pub async fn create(mut req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let body = req.json().await?;
    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({"message": "Task created", "task": body}))?)
}
```

Add routes in `main.rs`:
```rust
.get("/api/tasks", controllers::tasks::list)
.post("/api/tasks", controllers::tasks::create)
```

## Step 6: Add Frontend

Edit `templates/index.html`:
```html
<!DOCTYPE html>
<html>
<head>
    <title>My App</title>
    <link rel="stylesheet" href="/static/css/main.css">
</head>
<body>
    <div id="app">
        <h1>Tasks</h1>
        <ul id="task-list"></ul>
        <button id="add-task">Add Task</button>
    </div>
    <script src="https://code.jquery.com/jquery-3.7.1.min.js"></script>
    <script src="/static/js/core.js"></script>
    <script>
        $(document).ready(function() {
            // Load tasks
            $.get('/api/tasks', function(data) {
                data.tasks.forEach(function(task) {
                    $('#task-list').append('<li>' + task.title + '</li>');
                });
            });
        });
    </script>
</body>
</html>
```

## Step 7: Add Database (Optional)

```bash
# Create migration
yaiko migrate create tasks

# Edit migrations/YYYYMMDD_tasks.sql
```

```sql
CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    done BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW()
);
```

Update `.env`:
```bash
DATABASE_URL=postgres://user:pass@localhost/myapp
```

## Step 8: Build for Production

```bash
yaiko build --release
```

Binary is at `./target/release/myapp`

## Next Steps

- [Deployment Guide](deployment.md) - Deploy to a VPS with nginx
- [Database Guide](database.md) - Full database integration
- [Security Guide](security.md) - Add authentication
