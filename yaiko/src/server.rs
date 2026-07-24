use crate::{App, Request};
use crate::handler::Handler;
use hyper::server::conn::AddrStream;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server as HyperServer;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct Server {
    app: Arc<App>,
    addr: SocketAddr,
}

impl Server {
    pub fn new(app: App, addr: SocketAddr) -> Self {
        Server {
            app: Arc::new(app),
            addr,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let app = self.app.clone();
        
        let make_svc = make_service_fn(move |conn: &AddrStream| {
            let app = app.clone();
            let remote_addr = conn.remote_addr();
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let app = app.clone();
                    let remote_addr = remote_addr;
                    async move {
                        let request = Request::from_hyper_with_addr(req, Some(remote_addr)).await?;
                        let response = app.handle(request).await?;
                        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(response.into_hyper())
                    }
                }))
            }
        });

        let server = HyperServer::bind(&self.addr)
            .serve(make_svc)
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
                println!("\nReceived shutdown signal, draining connections...");
            });
        
        println!("Server running on http://{}", self.addr);
        
        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }

        println!("Server stopped gracefully.");

        Ok(())
    }
}
