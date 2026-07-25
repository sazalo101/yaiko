# yaiko-core

The core web framework engine for **Yaiko** — Built on Hyper and Tokio for high-performance Rust web applications.

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

## Usage

Add `yaiko-core` to your `Cargo.toml`:

```toml
[dependencies]
yaiko-core = "0.1.1"
tokio = { version = "1.0", features = ["full"] }
```

### Quick Web Server Example

```rust
use yaiko_core::{App, Router, Server, Request, Response, BoxError};

async fn hello(_req: Request) -> Result<Response, BoxError> {
    Ok(Response::new().text("Hello from Yaiko!"))
}

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let router = Router::new()
        .get("/", hello);

    let app = App::new().router(router);
    Server::new(app, "127.0.0.1:3000".parse()?).run().await?;
    Ok(())
}
```

## Features

- **Blazing Fast**: Hyper-powered non-blocking HTTP engine
- **Batteries Included**: Routing, Middleware, CSRF, Rate Limiting, Security Headers
- **Template & Static File Support**: Built-in Handlebars templates and static asset serving
- **Observability**: Built-in Prometheus metrics and tracing
- **Real-Time**: Native WebSockets support

## License

MIT
