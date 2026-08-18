use crate::{App, Server};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub struct DevServer {
    app: App,
    addr: SocketAddr,
    watch_dir: String,
}

impl DevServer {
    pub fn new(app: App, addr: SocketAddr, watch_dir: &str) -> Self {
        DevServer {
            app,
            addr,
            watch_dir: watch_dir.to_string(),
        }
    }

    pub async fn run_with_reload(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (tx, rx) = channel();

        // Create watcher with proper configuration
        let mut watcher: RecommendedWatcher = Watcher::new(
            tx,
            notify::Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;

        // Convert String to Path and watch
        watcher.watch(Path::new(&self.watch_dir), RecursiveMode::Recursive)?;

        println!(
            "Development server starting with file watching on {}",
            self.watch_dir
        );

        let server = Server::new(self.app, self.addr);

        tokio::spawn(async move {
            while let Ok(event) = rx.recv() {
                println!("File changed: {:?}", event);
            }
        });

        server.run().await
    }
}
