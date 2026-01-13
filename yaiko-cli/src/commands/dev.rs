use std::path::Path;
use std::process::Stdio;
use colored::Colorize;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::sync::mpsc::channel;
use std::time::Duration;
use tokio::process::Command;

pub async fn run(host: &str, port: u16) -> anyhow::Result<()> {
    println!("[*] Starting development server...");
    
    // Check if we're in a Yaiko project
    if !Path::new("Cargo.toml").exists() || !Path::new("yaiko.toml").exists() {
        println!("[!] Not a Yaiko project. Run 'yaiko init' first.");
        return Ok(());
    }
    
    // Initial build and run
    let mut child = start_server(host, port).await?;
    
    println!("[*] Watching for changes...");
    println!("  Press {} to stop\n", "Ctrl+C".yellow());
    
    // Set up file watcher
    let (tx, rx) = channel();
    
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
    
    // Watch src directory
    debouncer.watcher().watch(Path::new("src"), RecursiveMode::Recursive)?;
    
    // Watch templates directory if it exists
    if Path::new("templates").exists() {
        debouncer.watcher().watch(Path::new("templates"), RecursiveMode::Recursive)?;
    }
    
    // Main event loop
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                let should_rebuild = events.iter().any(|e| {
                    matches!(e.kind, DebouncedEventKind::Any)
                });
                
                if should_rebuild {
                    println!("\n[*] Changes detected, rebuilding...");
                    
                    // Kill existing process
                    let _ = child.kill().await;
                    
                    // Restart server
                    child = start_server(host, port).await?;
                }
            }
            Ok(Err(e)) => {
                println!("{} Watch error: {:?}", "!".yellow(), e);
            }
            Err(e) => {
                println!("[!] Channel error: {:?}", e);
                break;
            }
        }
    }
    
    Ok(())
}

async fn start_server(host: &str, port: u16) -> anyhow::Result<tokio::process::Child> {
    println!("[*] Building project...");
    
    // Run cargo build
    let build_status = Command::new("cargo")
        .args(["build"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;
    
    if !build_status.success() {
        println!("[!] Build failed!");
        // Return a placeholder child that immediately exits
        return Ok(Command::new("echo")
            .arg("Build failed")
            .spawn()?);
    }
    
    println!("[OK] Starting server on {}:{}...", host, port);
    
    // Start the server
    let child = Command::new("cargo")
        .args(["run"])
        .env("HOST", host)
        .env("PORT", port.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    Ok(child)
}
