use colored::Colorize;
use dialoguer::Confirm;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;

use crate::commands::common;

pub async fn run(name: &str, database: &str) -> anyhow::Result<()> {
    common::ensure_supported_database(database)?;
    println!("[*] Creating new Yaiko project: {}", name.green());

    let project_path = Path::new(name);
    let package_name = project_path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("project path must end with a valid directory name"))?;

    if project_path.exists() {
        let overwrite = Confirm::new()
            .with_prompt(format!("Directory '{}' already exists. Overwrite?", name))
            .default(false)
            .interact()?;

        if !overwrite {
            println!("[!] Aborted.");
            return Ok(());
        }
        fs::remove_dir_all(project_path)?;
    }

    let pb = ProgressBar::new(6);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    pb.set_message("Creating directories...");
    create_directories(project_path)?;
    pb.inc(1);

    pb.set_message("Generating Cargo.toml...");
    generate_cargo_toml(project_path, package_name, database)?;
    pb.inc(1);

    pb.set_message("Generating yaiko.toml...");
    generate_yaiko_config(project_path, database)?;
    pb.inc(1);

    pb.set_message("Generating .env...");
    generate_env_file(project_path, database)?;
    pb.inc(1);

    pb.set_message("Generating source files...");
    generate_source_files(project_path)?;
    pb.inc(1);

    pb.set_message("Generating frontend assets...");
    generate_frontend_assets(project_path)?;
    pb.inc(1);

    pb.finish_with_message("Done!");

    println!("\n[OK] Project '{}' created successfully!\n", name.green());
    println!("  Next steps:");
    println!("    cd {}", name);
    println!("    yaiko dev");
    println!("\n  Happy coding!\n");

    Ok(())
}

fn create_directories(project_path: &Path) -> anyhow::Result<()> {
    let dirs = [
        "",
        "src",
        "src/controllers",
        "src/models",
        "src/middleware",
        "public",
        "public/css",
        "public/js",
        "public/images",
        "public/fonts",
        "templates",
        "templates/layouts",
        "templates/partials",
        "migrations",
        "config",
    ];
    for dir in dirs {
        fs::create_dir_all(project_path.join(dir))?;
    }
    Ok(())
}

fn generate_cargo_toml(project_path: &Path, name: &str, database: &str) -> anyhow::Result<()> {
    let db_feature = if database == "sqlite" {
        "sqlite"
    } else {
        "postgres"
    };
    let yaiko_core_dependency = common::resolve_yaiko_core_dependency()?;
    let content = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
{yaiko_core_dependency}
tokio = {{ version = "1.0", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
chrono = {{ version = "0.4", features = ["serde"] }}
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = {{ version = "0.3", features = ["env-filter"] }}
sqlx = {{ version = "0.7", features = ["{db_feature}", "runtime-tokio-rustls", "chrono"] }}
"#
    );
    fs::write(project_path.join("Cargo.toml"), content)?;
    Ok(())
}

fn generate_yaiko_config(project_path: &Path, database: &str) -> anyhow::Result<()> {
    let content = format!(
        r#"# Yaiko Configuration
[server]
host = "127.0.0.1"
port = 3000

[database]
db_type = "{database}"
url = ""

[security]
cors_origins = ["http://localhost:3000"]
rate_limit_requests = 100
rate_limit_window_secs = 60
csrf_enabled = true

[seo]
robots_txt_enabled = true
sitemap_enabled = true
sitemap_changefreq = "weekly"

[logging]
level = "info"
format = "pretty"
"#
    );
    fs::write(project_path.join("yaiko.toml"), content)?;
    Ok(())
}

fn generate_env_file(project_path: &Path, database: &str) -> anyhow::Result<()> {
    let db_url = if database == "sqlite" {
        "sqlite:./data.db?mode=rwc"
    } else {
        "postgres://user:password@localhost:5432/myapp"
    };
    let content = format!(
        r#"HOST=127.0.0.1
PORT=3000
DATABASE_URL={db_url}
JWT_SECRET=change-me-in-production
RUST_LOG=info
SITE_URL=http://localhost:3000
"#
    );
    fs::write(project_path.join(".env"), &content)?;
    fs::write(project_path.join(".env.example"), &content)?;
    fs::write(
        project_path.join(".gitignore"),
        "/target\n.env\n.DS_Store\n",
    )?;
    Ok(())
}

fn generate_source_files(project_path: &Path) -> anyhow::Result<()> {
    let main_rs = include_str!("../templates/main.rs.tmpl");
    fs::write(project_path.join("src/main.rs"), main_rs)?;
    fs::write(
        project_path.join("src/controllers/mod.rs"),
        "pub mod users;\n",
    )?;
    fs::write(
        project_path.join("src/controllers/users.rs"),
        include_str!("../templates/users.rs.tmpl"),
    )?;
    fs::write(project_path.join("src/models/mod.rs"), "// Models\n")?;
    fs::write(
        project_path.join("src/middleware/mod.rs"),
        "// Middleware\n",
    )?;
    Ok(())
}

fn generate_frontend_assets(project_path: &Path) -> anyhow::Result<()> {
    fs::write(
        project_path.join("templates/index.html"),
        include_str!("../templates/index.html.tmpl"),
    )?;
    fs::write(
        project_path.join("public/css/main.css"),
        include_str!("../templates/main.css.tmpl"),
    )?;
    fs::write(
        project_path.join("public/js/core.js"),
        include_str!("../templates/core.js.tmpl"),
    )?;
    fs::write(
        project_path.join("public/js/app.js"),
        include_str!("../templates/app.js.tmpl"),
    )?;
    Ok(())
}
