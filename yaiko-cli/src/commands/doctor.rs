use colored::Colorize;
use which::which;

use crate::commands::common;

pub async fn run() -> anyhow::Result<()> {
    println!("[*] Checking Yaiko environment...");

    let mut failed = false;

    failed |= !check_binary("cargo", "Cargo is required to build Yaiko apps.");
    failed |= !check_binary("rustc", "Rust is required to compile Yaiko apps.");

    match common::resolve_yaiko_core_dependency() {
        Ok(source) => println!("[OK] yaiko-core dependency source: {}", source.green()),
        Err(error) => {
            failed = true;
            println!("[!] {}", error);
        }
    }

    if std::path::Path::new("Cargo.toml").exists() || std::path::Path::new("yaiko.toml").exists() {
        match common::ensure_yaiko_project(std::path::Path::new(".")) {
            Ok(()) => println!("[OK] Current directory looks like a Yaiko project."),
            Err(error) => {
                failed = true;
                println!("[!] {}", error);
            }
        }
    } else {
        println!("[*] Current directory is not a Yaiko project. That is fine for 'yaiko init'.");
    }

    if failed {
        anyhow::bail!("Yaiko doctor found issues. Fix the messages above and rerun the command.");
    }

    println!("[OK] Yaiko environment looks ready.");
    Ok(())
}

fn check_binary(binary: &str, hint: &str) -> bool {
    match which(binary) {
        Ok(path) => {
            println!("[OK] Found {} at {}", binary.green(), path.display());
            true
        }
        Err(_) => {
            println!("[!] {} not found. {}", binary.red(), hint);
            false
        }
    }
}
