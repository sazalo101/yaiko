use std::path::Path;
use std::process::Stdio;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::process::Command;

pub async fn run(release: bool) -> anyhow::Result<()> {
    println!("[*] Building project for production...");
    
    // Check if we're in a Yaiko project
    if !Path::new("Cargo.toml").exists() {
        println!("[!] Not a Yaiko project. Run 'yaiko init' first.");
        return Ok(());
    }
    
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    
    // Step 1: Build Rust
    pb.set_message("Compiling Rust code...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    
    let build_status = Command::new("cargo")
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;
    
    if !build_status.success() {
        pb.finish_with_message("[!] Build failed!".to_string());
        return Ok(());
    }
    
    // Step 2: Optimize assets (future: minify CSS/JS)
    pb.set_message("Optimizing assets...");
    
    // For now, just check that public directory exists
    if Path::new("public").exists() {
        // Future: minify CSS, JS, compress images
        pb.set_message("Assets ready (minification coming soon)");
    }
    
    pb.finish_with_message("[OK] Build complete!".to_string());
    
    let binary_path = if release {
        "./target/release/"
    } else {
        "./target/debug/"
    };
    
    println!("\n  Binary location: {}", binary_path.cyan());
    println!("  To run: ./target/release/<your-app-name>\n");
    
    Ok(())
}
