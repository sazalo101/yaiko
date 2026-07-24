# Yaiko CLI Documentation

The **Yaiko CLI** (`yaiko`) is the official developer tool for project scaffolding, development, code generation, database migrations, and production builds.

---

## Installation

```bash
# Install CLI from the Yaiko repository
cargo install --path ./yaiko-cli --force

# Verify installation
yaiko --version
```

---

## Command Overview (`yaiko --help`)

| Command | Description |
| ------- | ----------- |
| `yaiko init <name>` | Initialize a new Yaiko project |
| `yaiko dev` | Start development server with hot-reloading |
| `yaiko build` | Build application for production |
| `yaiko doctor` | Check environment readiness and tool dependencies |
| `yaiko migrate` | Manage SQL database migrations |
| `yaiko generate` | Generate controllers, models, and middleware |

---

## Detailed Command Reference

### 1. `yaiko init <NAME>`
Initialize a new project in directory `<NAME>`.

```bash
# Default (PostgreSQL)
yaiko init my-app

# With SQLite
yaiko init my-app --database sqlite
yaiko init my-app -d sqlite
```

**Options**:
- `-d, --database <DATABASE>`: Database engine (`postgres` [default], `sqlite`).

---

### 2. `yaiko dev`
Start the development server with file watching and auto hot-reload.

```bash
# Default (http://127.0.0.1:3000)
yaiko dev

# Custom port and host
yaiko dev --port 8080 --host 0.0.0.0
yaiko dev -p 8080 --host 0.0.0.0
```

**Options**:
- `-p, --port <PORT>`: Port number (default: `3000`).
- `--host <HOST>`: Bind host IP (default: `127.0.0.1`).

---

### 3. `yaiko build`
Compile the application for production.

```bash
# Development build
yaiko build

# Optimized release build
yaiko build --release
yaiko build -r
```

**Options**:
- `-r, --release`: Enable cargo `--release` optimizations.

---

### 4. `yaiko doctor`
Inspect local system environment to verify Rust, Cargo, SQLx, and CLI readiness.

```bash
yaiko doctor
```

---

### 5. `yaiko migrate`
Manage SQL migrations in the `migrations/` directory.

```bash
# Create a new timestamped migration file
yaiko migrate create add_users_table

# Run pending migrations
yaiko migrate run

# Roll back the last migration
yaiko migrate rollback

# Check status of applied migrations
yaiko migrate status
```

---

### 6. `yaiko generate` (or `yaiko g`)
Scaffold boilerplate components into your codebase.

```bash
# Scaffold a controller in src/controllers/posts.rs
yaiko generate controller posts

# Scaffold a model in src/models/post.rs
yaiko generate model post

# Scaffold middleware in src/middleware/auth.rs
yaiko generate middleware auth
```
