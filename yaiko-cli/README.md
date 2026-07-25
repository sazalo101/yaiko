# Yaiko CLI

The official Command Line Interface (CLI) tool for the **Yaiko** web framework.

```
    ██╗   ██╗ █████╗ ██╗██╗  ██╗ ██████╗ 
    ╚██╗ ██╔╝██╔══██╗██║██║ ██╔╝██╔═══██╗
     ╚████╔╝ ███████║██║█████╔╝ ██║   ██║
      ╚██╔╝  ██╔══██║██║██╔═██╗ ██║   ██║
       ██║   ██║  ██║██║██║  ██╗╚██████╔╝
       ╚═╝   ╚═╝  ╚═╝╚═╝╚═╝  ╚═╝ ╚═════╝ 
```

**Repository**: [https://github.com/sazalo101/yaiko](https://github.com/sazalo101/yaiko)  
📖 **[The Yaiko Book](https://github.com/sazalo101/yaiko/blob/main/BOOK.md)** | 🌱 **[Yaiko for Beginners](https://github.com/sazalo101/yaiko/blob/main/BEGINNERS_BOOK.md)**

## Installation

```bash
cargo install yaiko
```

## Quick Start

```bash
# Check system dependencies
yaiko doctor

# Initialize a new Yaiko project (with SQLite or Postgres)
yaiko init my-app -d sqlite

# Start hot-reloading development server
cd my-app
yaiko dev

# Build optimized release binary
yaiko build --release
```

## Commands Reference

| Command | Arguments | Description |
| --- | --- | --- |
| `yaiko init <name>` | `-d, --database <sqlite\|postgres>` | Scaffold a new Yaiko fullstack project |
| `yaiko dev` | `-p, --port <3000>` | Run local dev server with auto hot-reload |
| `yaiko build` | `-r, --release` | Build release binary |
| `yaiko doctor` | — | Verify Rust, Cargo & toolchain environment |
| `yaiko migrate create <name>` | — | Generate a new SQL migration file |
| `yaiko migrate run` | — | Apply pending database migrations |
| `yaiko migrate rollback` | — | Revert last applied migration |
| `yaiko migrate status` | — | Check migration status |

## License

MIT
