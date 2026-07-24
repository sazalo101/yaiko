# Getting Started

This guide will help you create your first Yaiko application.

## Prerequisites

- Rust 1.70+ (`rustup update`)
- PostgreSQL or SQLite (optional)

## Installation

```bash
# Clone Yaiko repository
git clone https://github.com/sazalo101/yaiko.git

cd yaiko

# Install the CLI globally
cargo install --path ./yaiko-cli

# Verify installation
yaiko --version
```

## Create a Project

```bash
# Create new project
yaiko init my-app

# With SQLite instead of PostgreSQL
yaiko init my-app --database sqlite
```

This creates:
```
my-app/
├── src/
│   ├── main.rs           # Entry point
│   ├── controllers/      # Route handlers
│   │   └── users.rs      # Example controller
│   ├── models/           # Database models
│   └── middleware/       # Custom middleware
├── public/
│   ├── css/main.css      # Stylesheet
│   └── js/core.js        # jQuery utilities
├── templates/
│   └── index.html        # Homepage
├── migrations/           # SQL migrations
├── Cargo.toml            # Dependencies
├── yaiko.toml            # Framework config
└── .env                  # Environment variables
```

## Run Development Server

```bash
cd my-app
yaiko dev
```

Open [http://localhost:3000](http://localhost:3000) to see your app.

The dev server:
- Watches `src/` for changes
- Auto-rebuilds on save
- Hot-reloads the browser

## Project Structure

### `src/main.rs`
Entry point. Defines routes and starts the server:

```rust
use yaiko_core::{App, Router, Server, Request, Response};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = Router::new()
        .get("/", home_handler)
        .get("/api/users", controllers::users::list);

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
```

### `yaiko.toml`
Framework configuration:

```toml
[server]
host = "127.0.0.1"
port = 3000

[security]
cors_origins = ["http://localhost:3000"]
rate_limit_requests = 100
csrf_enabled = true

[seo]
robots_txt_enabled = true
sitemap_enabled = true
```

### `.env`
Environment variables:

```bash
DATABASE_URL=postgres://user:pass@localhost/myapp
JWT_SECRET=your-secret-key
RUST_LOG=info
```

## Next Steps

- [Routing Guide](routing.md) — Learn about routes and handlers
- [Database Guide](database.md) — Connect to PostgreSQL/SQLite
- [Frontend Guide](frontend.md) — Templates and jQuery
