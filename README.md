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

## CLI Reference (`yaiko --help`)

| Command / Subcommand | Options & Flags | Description |
| -------------------- | --------------- | ----------- |
| `yaiko init <name>` | `-d, --database <type>` *(default: postgres)* | Initialize a new Yaiko project (supports `postgres`, `sqlite`) |
| `yaiko dev` | `-p, --port <port>` *(default: 3000)*<br>`--host <host>` *(default: 127.0.0.1)* | Start dev server with auto hot-reload |
| `yaiko build` | `-r, --release` | Build binary with release optimizations |
| `yaiko doctor` | — | Check if Rust, Cargo, SQLx & CLI environment are ready |
| `yaiko migrate create <name>` | — | Create a new SQL migration file in `migrations/` |
| `yaiko migrate run` | — | Execute pending database migrations |
| `yaiko migrate rollback` | — | Roll back the last applied migration |
| `yaiko migrate status` | — | View migration execution history and pending status |
| `yaiko generate controller <name>` | — | Scaffold a new route controller in `src/controllers/` |
| `yaiko generate model <name>` | — | Scaffold a new database model in `src/models/` |
| `yaiko generate middleware <name>` | — | Scaffold a new request middleware in `src/middleware/` |


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
