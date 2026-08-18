use anyhow::Context;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use crate::commands::common;

pub async fn run(release: bool) -> anyhow::Result<()> {
    println!("[*] Building project for production...");
    let project_dir = Path::new(".");
    common::ensure_yaiko_project(project_dir)?;
    let project_name = common::load_project_name(project_dir)?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    // Step 1: Build Rust
    pb.set_message("Compiling Rust code...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }

    let build_status = Command::new("cargo")
        .args(&args)
        .arg("--manifest-path")
        .arg(common::cargo_manifest(project_dir))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("Failed to run 'cargo build'. Is Cargo installed?")?;

    if !build_status.success() {
        anyhow::bail!("Build failed. Fix the Rust errors above and rerun 'yaiko build'.");
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

    println!(
        "\n  Binary location: {}{}",
        binary_path.cyan(),
        project_name.cyan()
    );
    println!("  To run: {}{}\n", binary_path, project_name);

    Ok(())
}
