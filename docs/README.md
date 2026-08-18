# Yaiko Documentation

Welcome to **Yaiko** — a modern, production-ready fullstack web framework for Rust + jQuery.

## Quick Links

| Guide                                 | Description                           |
| ------------------------------------- | ------------------------------------- |
| [CLI Reference](cli.md)               | Full CLI command & flag reference     |
| [Tutorial](tutorial.md)               | Build an app from start to finish     |
| [Getting Started](getting-started.md) | Install and create your first project |
| [Routing](routing.md)                 | Define routes and handlers            |
| [Configuration](configuration.md)     | Configure your application            |
| [Database](database.md)               | Work with PostgreSQL/SQLite           |
| [Security](security.md)               | CSRF, rate limiting, auth             |
| [Frontend](frontend.md)               | jQuery + Handlebars templates         |
| [Deployment](deployment.md)           | Deploy to VPS with nginx              |
| [Built-in Modules](../examples/docs/content/built-in-modules.md) | Typed frontend, data, API, realtime, tooling, and media facades |

## Why Yaiko?

- **Fast** — Built on Rust with hyper
- **Secure** — Security middleware included
- **Full Stack** — Backend + frontend in one
- **CLI** — Scaffolding, dev server, migrations
- **Typed built-ins** — Validated policies for frontend primitives, data, security, API/RPC, realtime, tooling, observability, deployment, and media editing

## Install

```bash
cargo install --path ./yaiko-cli
yaiko init my-app
cd my-app && yaiko dev
```

## Examples

- [Static Blog](../examples/blog) — Simple blog with markdown posts
- [Notes App](../examples/notes) — CRUD note-taking application
- [AI Chat](../examples/chat) — Claude-like chat with OpenRouter API
- [Built-in module catalog](../examples/docs/content/built-in-modules.md) — Current built-ins and usage examples
- [ImgHost](../examples/imghost) — Uploads, storage, media delivery, and production deployment patterns
