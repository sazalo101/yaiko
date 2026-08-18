# Yaiko Documentation

Welcome to **Yaiko** — a modern, production-ready fullstack web framework for Rust + jQuery.

## Quick Links

| Guide                                 | Description                           |
| ------------------------------------- | ------------------------------------- |
| [Tutorial](tutorial.md)               | Build an app from start to finish     |
| [Getting Started](getting-started.md) | Install and create your first project |
| [Routing](routing.md)                 | Define routes and handlers            |
| [Configuration](configuration.md)     | Configure your application            |
| [Database](database.md)               | Work with PostgreSQL/SQLite           |
| [Security](security.md)               | CSRF, rate limiting, auth             |
| [Frontend](frontend.md)               | jQuery + Handlebars templates         |
| [Deployment](deployment.md)           | Deploy to VPS with nginx              |
| [Built-in Modules](built-in-modules.md) | Typed frontend, data, API, realtime, tooling, and media facades |
| [Beginner Book](beginners-book.md)     | Step-by-step learning path             |
| [Full Book](book.md)                   | Architecture, tutorials, and project ideas |

## Why Yaiko?

- **Fast** — Built on Rust with hyper
- **Secure** — Security middleware included
- **Full Stack** — Backend + frontend in one
- **CLI** — Scaffolding, dev server, migrations
- **Typed built-ins** — Validated policies for frontend, data, security, APIs, realtime, deployment, observability, and media editing

## Install

```bash
cargo install --path ./yaiko-cli
yaiko init my-app
cd my-app && yaiko dev
```

## Examples

- [Static Blog](../../blog) — Simple blog with markdown posts
- [Notes App](../../notes) — CRUD note-taking application
- [AI Chat](../../chat) — Claude-like chat with OpenRouter API
- [Built-in module catalog](built-in-modules.md) — Current catalog and code examples
- [ImgHost](../../imghost) — Upload, storage, and media-delivery example
- [Built-in Catalog](../../catalog) — Router, health, metadata, and structured JSON
- [Media Studio](../../media-studio) — Persistent SQLite media-editor repository
- [Webhook Inbox](../../webhook-inbox) — Signed event verification and replay protection
