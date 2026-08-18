use std::net::SocketAddr;

use yaiko_core::{
    json, App, Head, HealthCheck, Metadata, Request, Response, Router, Server,
};

async fn home(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let metadata = Metadata::new()
        .title("Yaiko Built-in Catalog")
        .map_err(|_| "invalid metadata title")?
        .description("A runnable tour of Yaiko's typed framework facades")
        .map_err(|_| "invalid metadata description")?
        .canonical("https://example.com/catalog")
        .map_err(|_| "invalid canonical URL")?
        .json_ld(r#"{"@type":"WebSite","name":"Yaiko Built-in Catalog"}"#)
        .map_err(|_| "invalid JSON-LD")?;
    let head = Head::new()
        .title("Yaiko Built-in Catalog")
        .map_err(|_| "invalid head title")?
        .meta("description", "A runnable tour of Yaiko built-ins")
        .map_err(|_| "invalid head metadata")?
        .link("canonical", "/")
        .map_err(|_| "invalid head link")?
        .script("/catalog.js", true)
        .map_err(|_| "invalid head script")?;
    let body = format!(
        "<!doctype html><html><head>{}{}</head><body><h1>Yaiko Built-in Catalog</h1><p>Try <a href=\"/api/catalog\">/api/catalog</a>.</p></body></html>",
        metadata.render(),
        head.render()
    );
    Ok(Response::new().html(&body))
}

async fn catalog(_request: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().json(&json!({
        "framework": "yaiko",
        "modules": [
            "router", "health", "head", "metadata", "security", "media-editor"
        ],
        "principle": "typed, bounded, deterministic facades"
    }))?)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let router = Router::new()
        .get("/", home)
        .get("/health", HealthCheck::new())
        .get("/api/catalog", catalog);
    let app = App::new().router(router);
    let address: SocketAddr = "127.0.0.1:3010".parse()?;
    println!("Yaiko catalog running at http://{address}");
    Server::new(app, address).run().await?;
    Ok(())
}
