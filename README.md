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

**Repository**: [https://github.com/sazalo101/yaiko](https://github.com/sazalo101/yaiko)

## Features

- **🚀 Fast** - Built on Rust with hyper for blazing fast performance
- **🔒 Secure** - CSRF protection, rate limiting, security headers out of the box
- **📦 Full Stack** - Rust backend + jQuery frontend, batteries included
- **🔌 Real-time** - Built-in WebSocket support
- **⚡ Background Jobs** - Async job queue for background processing
- **📊 Observability** - Built-in Prometheus metrics and tracing
- **📂 File Uploads** - Easy multipart file upload handling with JigsawStack NSFW validation support
- **🛠️ CLI Tools** - Scaffolding, dev server, migrations, code generators
- **📝 SEO Ready** - Automatic robots.txt and sitemap.xml generation

## Quick Start

```bash
# Clone the repository
git clone https://github.com/sazalo101/yaiko.git
cd yaiko

# Install the CLI from this repo
cargo install --path yaiko-cli --force

# Check that your local environment is ready
yaiko doctor

# Create a new project
yaiko init my-app

# Start developing
cd my-app
yaiko dev

# Production build
yaiko build --release
```

## Built-in Examples

- **`examples/imghost`**: Free image hosting web app with JigsawStack NSFW content validation deployed at [imghost.se](https://imghost.se)
- **`examples/teampulse`**: Real-time team messaging app with WebSockets & SQLite
- **`examples/auth`**: User registration, login, JWT authentication, and session handling
- **`examples/chat`**: Real-time WebSocket chat room application
- **`examples/docs`**: Documentation site template
- **`examples/link-in-bio`**: Link tree / Bio page generator
- **`examples/file-request-links`**: Secure file request and sharing application

## CLI Commands

| Command                            | Description                      |
| ---------------------------------- | -------------------------------- |
| `yaiko init <name>`                | Create a new project             |
| `yaiko doctor`                     | Validate local Yaiko setup       |
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
│   ├── routes/           # Route handlers
│   └── models/           # Database models
├── public/
│   ├── css/style.css     # Styles
│   └── js/app.js         # Frontend application logic
├── migrations/           # SQL migrations
├── Cargo.toml
├── yaiko.toml            # Framework config
└── .env                  # Environment variables
```

## License

MIT
