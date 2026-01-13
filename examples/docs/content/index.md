# Yaiko

A modern, production-ready fullstack web framework for **Rust + jQuery**.

```
    ██╗   ██╗ █████╗ ██╗██╗  ██╗ ██████╗ 
    ╚██╗ ██╔╝██╔══██╗██║██║ ██╔╝██╔═══██╗
     ╚████╔╝ ███████║██║█████╔╝ ██║   ██║
      ╚██╔╝  ██╔══██║██║██╔═██╗ ██║   ██║
       ██║   ██║  ██║██║██║  ██╗╚██████╔╝
       ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝ ╚═════╝ 
```

## Features

- **🚀 Fast** - Built on Rust with hyper for blazing fast performance
- **🔒 Secure** - CSRF protection, rate limiting, security headers out of the box
- **📦 Full Stack** - Rust backend + jQuery frontend, batteries included
- **🔌 Real-time** - Built-in WebSocket support
- **⚡ Background Jobs** - Async job queue for background processing
- **📂 File Uploads** - Easy multipart file upload handling
- **🛠️ CLI Tools** - Scaffolding, dev server, migrations, code generators
- **📝 SEO Ready** - Automatic robots.txt and sitemap.xml generation

## Quick Start

```bash
# Install the CLI
cargo install --path ./yaiko-cli

# Create a new project
yaiko init my-app

# Start developing
cd my-app
yaiko dev
```

## CLI Commands

| Command                            | Description                      |
| ---------------------------------- | -------------------------------- |
| `yaiko init <name>`                | Create a new project             |
| `yaiko dev`                        | Start dev server with hot-reload |
| `yaiko build`                      | Build for production             |
| `yaiko migrate create <name>`      | Create a migration               |
| `yaiko migrate run`                | Run pending migrations           |
| `yaiko generate controller <name>` | Generate a controller            |
| `yaiko generate model <name>`      | Generate a model                 |

## Project Structure

```
my-app/
├── src/
│   ├── main.rs           # Entry point
│   ├── controllers/      # Route handlers
│   ├── models/           # Database models
│   └── middleware/       # Custom middleware
├── public/
│   ├── css/main.css      # Styles
│   └── js/core.js        # jQuery utilities
├── templates/            # Handlebars templates
├── migrations/           # SQL migrations
├── Cargo.toml
├── yaiko.toml            # Framework config
└── .env                  # Environment variables
```

## Configuration

`yaiko.toml`:
```toml
[server]
host = "127.0.0.1"
port = 3000

[database]
type = "postgres"

[security]
cors_origins = ["http://localhost:3000"]
rate_limit_requests = 100
csrf_enabled = true

[seo]
robots_txt_enabled = true
sitemap_enabled = true
```

## License

MIT
# yaiko
