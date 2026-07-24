use std::path::{Path, PathBuf};
use std::process::Stdio;
use anyhow::Context;
use colored::Colorize;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::sync::mpsc::channel;
use std::time::Duration;
use tokio::process::Command;

use crate::commands::common;

pub async fn run(host: &str, port: u16) -> anyhow::Result<()> {
    println!("[*] Starting development server...");
    let project_dir = PathBuf::from(".");
    common::ensure_yaiko_project(&project_dir)?;
    let project_name = common::load_project_name(&project_dir)?;
    
    // Initial build and run
    let mut child = start_server(&project_dir, &project_name, host, port).await?;
    
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

    if Path::new("public").exists() {
        debouncer.watcher().watch(Path::new("public"), RecursiveMode::Recursive)?;
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
                    let _ = child.wait().await;
                    
                    // Restart server
                    child = start_server(&project_dir, &project_name, host, port).await?;
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

async fn start_server(
    project_dir: &Path,
    project_name: &str,
    host: &str,
    port: u16,
) -> anyhow::Result<tokio::process::Child> {
    println!("[*] Building project...");
    
    let manifest = common::cargo_manifest(project_dir);
    let build_status = Command::new("cargo")
        .args(["build", "--manifest-path"])
        .arg(&manifest)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("Failed to run 'cargo build'. Is Cargo installed?")?;
    
    if !build_status.success() {
        anyhow::bail!("Build failed. Fix the Rust errors above and save again to retry.");
    }
    
    println!("[OK] Starting server on {}:{}...", host, port);
    
    let child = Command::new("cargo")
        .args(["run", "--manifest-path"])
        .arg(&manifest)
        .arg("--bin")
        .arg(project_name)
        .env("HOST", host)
        .env("PORT", port.to_string())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("Failed to start '{}'.", project_name))?;
    
    Ok(child)
}
