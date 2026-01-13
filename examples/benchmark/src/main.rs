use yaiko_core::{App, Router, Server, Request, Response, json};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // No logging middleware for raw performance testing
    
    let router = Router::new()
        .get("/plaintext", |_| async {
            Ok(Response::new().text("Hello, World!"))
        })
        .get("/json", |_| async {
            Ok(Response::new().json(&json!({"message": "Hello, World!"}))?)
        });
    
    let app = App::new().router(router);
    
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    println!("Benchmark server running at http://{}", addr);
    server.run().await?;
    
    Ok(())
}
