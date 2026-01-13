# Getting Started with Yaiko
2026-01-12

Yaiko is a modern, production-ready fullstack web framework for Rust and jQuery.

It combines the speed and safety of Rust with the simplicity of jQuery for building real-world web applications.

## Features

- Fast - Built on Rust with hyper
- Secure - CSRF, rate limiting, security headers
- Full Stack - Backend and frontend in one project
- CLI Tools - Scaffolding, dev server, migrations

## Installation

Install the Yaiko CLI and create your first project:

```bash
cargo install --path ./yaiko-cli
yaiko init my-app
cd my-app && yaiko dev
```

Happy coding!
