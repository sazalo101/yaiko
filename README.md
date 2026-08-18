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
🌱 **[Yaiko for Beginners](examples/docs/content/beginners-book.md)** — Step-by-step tutorial book for learning web development with Rust & jQuery
📖 **[The Yaiko Book](examples/docs/content/book.md)** — Full framework guide, architecture, benchmarks, and project ideas
📚 **[Documentation](docs/)** — Getting started, routing, database, security, deployment & more
🧩 **[Built-in Modules](examples/docs/content/built-in-modules.md)** — Current catalog, typed facades, and usage patterns

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
- **🧩 Typed Built-ins** - Frontend, data, security, API, realtime, tooling, observability, deployment, and media-editor facades
- **🎬 Media Editing** - Timeline, captions, background music, annotations, reviews, versions, templates, and export presets

## Quick Start

```bash
# Install Yaiko from crates.io
cargo install yaiko

# Check that your local environment is ready
yaiko doctor

# Create a new project
yaiko init my-app -d sqlite

# Start developing
cd my-app
yaiko dev

# Production build
yaiko build --release
```

### Install from Source (alternative)

```bash
git clone https://github.com/sazalo101/yaiko.git
cd yaiko
cargo install --path yaiko-cli --force
```

## Built-in Examples

- **`examples/imghost`**: Free image hosting web app with JigsawStack NSFW content validation deployed at [imghost.se](https://imghost.se)
- **`examples/teampulse`**: Real-time team messaging app with WebSockets & SQLite
- **`examples/auth`**: User registration, login, JWT authentication, and session handling
- **`examples/chat`**: Real-time WebSocket chat room application
- **`examples/docs`**: Documentation site template
- **`examples/link-in-bio`**: Link tree / Bio page generator
- **`examples/file-request-links`**: Secure file request and sharing application
- **`examples/docs`**: Documentation site with the beginner guide, full book, and built-in module catalog
- **`examples/blog`**: Static blog demonstrating routing, SEO endpoints, static assets, logging, security headers, and JSON APIs

## Built-in Module Status

The current branch contains verified built-ins for routing, frontend primitives, data ergonomics, security, API/RPC, background and realtime workflows, developer tooling, observability, deployment planning, utilities, and media editing. These modules are reusable policy/domain facades; persistent repositories, HTTP/WebSocket application handlers, controlled FFmpeg workers, and the browser video-editor UI remain the next integration layer.

Recent verified deployments include:

| Commit | Batch |
|---|---|
| `58cd0aa` | Deployment configuration facade |
| `45a975f` | Deterministic format policy |
| `ed62cde` | Deterministic lint policy |
| `2d3d175` | Deterministic test facade |
| `3a7b466` | HMR policy |
| `33486d7` | File-watch policy |
| `5b9220e` | Type generation |
| `79eb870` | Health aggregation |
| `80c561f` | Structured log facade |

Each batch passed focused tests, strict Clippy, formatting, feature-matrix checks, CLI tests, and example builds before deployment.

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
