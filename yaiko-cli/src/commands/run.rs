use anyhow::{Context, Result};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

use crate::commands::common;

/// Build and run the current Yaiko project once, without the development watcher.
pub async fn run() -> Result<()> {
    let project_dir = Path::new(".");
    common::ensure_yaiko_project(project_dir)?;
    println!("[*] Running Yaiko project...");

    let status = Command::new("cargo")
        .arg("run")
        .current_dir(project_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await
        .context("Failed to run Cargo. Is Cargo installed?")?;

    if !status.success() {
        anyhow::bail!("Project failed. Fix the errors above and rerun 'yaiko run'.");
    }
    Ok(())
}
