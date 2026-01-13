# Configuration

Configure Yaiko applications via `yaiko.toml` and environment variables.

## Configuration File

`yaiko.toml` in your project root:

```toml
[server]
host = "127.0.0.1"
port = 3000

[database]
type = "postgres"  # or "sqlite"

[security]
cors_origins = ["http://localhost:3000"]
rate_limit_requests = 100
rate_limit_window_secs = 60
csrf_enabled = true

[seo]
robots_txt_enabled = true
sitemap_enabled = true
sitemap_changefreq = "weekly"

[logging]
level = "info"      # debug, info, warn, error
format = "pretty"   # or "json" for production
```

## Environment Variables

`.env` file (loaded automatically):

```bash
# Server
HOST=127.0.0.1
PORT=3000

# Database
DATABASE_URL=postgres://user:password@localhost:5432/myapp

# Security
JWT_SECRET=your-super-secret-key
CSRF_SECRET=another-secret-key

# Logging
RUST_LOG=info
```

## Loading Configuration

Use `Settings` in your application:

```rust
use yaiko_core::Settings;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load from yaiko.toml + env vars
    let settings = Settings::load()?;
    
    println!("Server: {}:{}", settings.server.host, settings.server.port);
    println!("Database: {}", settings.database.db_type);
    
    // Use in server
    let addr = settings.server_addr().parse()?;
    let server = Server::new(app, addr);
    server.run().await?;
    
    Ok(())
}
```

## Environment Overrides

Environment variables override config file values. Use `YAIKO__` prefix:

```bash
# Override server.port
YAIKO__SERVER__PORT=8080

# Override security.rate_limit_requests
YAIKO__SECURITY__RATE_LIMIT_REQUESTS=200
```

## Development vs Production

### Development (`yaiko dev`)
```toml
[logging]
level = "debug"
format = "pretty"
```

### Production (`yaiko build`)
```toml
[logging]
level = "info"
format = "json"
```

```bash
# Production environment
RUST_LOG=info
HOST=0.0.0.0
PORT=8080
```

## Configuration Sections

### `[server]`
| Key    | Type    | Default       | Description  |
| ------ | ------- | ------------- | ------------ |
| `host` | string  | `"127.0.0.1"` | Bind address |
| `port` | integer | `3000`        | Port number  |

### `[database]`
| Key    | Type   | Default      | Description            |
| ------ | ------ | ------------ | ---------------------- |
| `type` | string | `"postgres"` | `postgres` or `sqlite` |

### `[security]`
| Key                      | Type     | Default | Description             |
| ------------------------ | -------- | ------- | ----------------------- |
| `cors_origins`           | [string] | `[]`    | Allowed CORS origins    |
| `rate_limit_requests`    | integer  | `100`   | Max requests per window |
| `rate_limit_window_secs` | integer  | `60`    | Rate limit window       |
| `csrf_enabled`           | boolean  | `true`  | Enable CSRF protection  |

### `[seo]`
| Key                  | Type    | Default    | Description              |
| -------------------- | ------- | ---------- | ------------------------ |
| `robots_txt_enabled` | boolean | `true`     | Serve `/robots.txt`      |
| `sitemap_enabled`    | boolean | `true`     | Serve `/sitemap.xml`     |
| `sitemap_changefreq` | string  | `"weekly"` | Sitemap change frequency |

### `[logging]`
| Key      | Type   | Default    | Description   |
| -------- | ------ | ---------- | ------------- |
| `level`  | string | `"info"`   | Log level     |
| `format` | string | `"pretty"` | Output format |
