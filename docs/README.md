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

## Why Yaiko?

- **Fast** — Built on Rust with hyper
- **Secure** — Security middleware included
- **Full Stack** — Backend + frontend in one
- **CLI** — Scaffolding, dev server, migrations

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
