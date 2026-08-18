use chrono::Utc;
use colored::Colorize;
use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::fs;
use std::path::Path;

pub async fn create(name: &str) -> anyhow::Result<()> {
    println!("[*] Creating migration: {}", name.green());

    let migrations_dir = Path::new("migrations");
    if !migrations_dir.exists() {
        fs::create_dir_all(migrations_dir)?;
    }

    let timestamp = Utc::now().format("%Y%m%d%H%M%S");
    let filename = format!("{}_{}.sql", timestamp, name);
    let filepath = migrations_dir.join(&filename);

    let content = format!(
        r#"-- Migration: {name}
-- Created: {timestamp}

-- Write your UP migration here

-- Example:
-- CREATE TABLE users (
--     id SERIAL PRIMARY KEY,
--     email VARCHAR(255) NOT NULL UNIQUE,
--     password_hash VARCHAR(255) NOT NULL,
--     created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
-- );

-- DOWN migration (for rollback)
-- To run rollback, create a separate file: {timestamp}_{name}_down.sql
"#
    );

    fs::write(&filepath, content)?;

    println!("[OK] Created: migrations/{}", filename);
    println!("  Edit the file and run 'yaiko migrate run' to apply.\n");

    Ok(())
}

pub async fn run() -> anyhow::Result<()> {
    // Load .env file automatically
    dotenvy::dotenv().ok();

    println!("[*] Running pending migrations...");

    // Check for DATABASE_URL
    let db_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            println!("[!] DATABASE_URL not set. Check your .env file.");
            return Ok(());
        }
    };

    let migrations_dir = Path::new("migrations");
    if !migrations_dir.exists() {
        println!("{} No migrations directory found.", "!".yellow());
        return Ok(());
    }

    // List migration files
    let mut migrations: Vec<_> = fs::read_dir(migrations_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sql"))
        .filter(|e| !e.file_name().to_string_lossy().contains("_down"))
        .collect();

    migrations.sort_by_key(|e| e.file_name());

    if migrations.is_empty() {
        println!("[OK] No migrations found in migrations/ directory.");
        return Ok(());
    }

    if db_url.starts_with("sqlite:") {
        run_sqlite(&db_url, &migrations).await?;
    } else if db_url.starts_with("postgres:") || db_url.starts_with("postgresql:") {
        run_postgres(&db_url, &migrations).await?;
    } else {
        println!(
            "[!] Unsupported DATABASE_URL protocol. Supported: sqlite:, postgres:, postgresql:"
        );
    }

    Ok(())
}

async fn run_sqlite(db_url: &str, migrations: &[fs::DirEntry]) -> anyhow::Result<()> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(db_url)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _yaiko_migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filename TEXT NOT NULL UNIQUE,
            executed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;

    let applied: Vec<String> =
        sqlx::query_as::<_, (String,)>("SELECT filename FROM _yaiko_migrations ORDER BY filename")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect();

    let mut ran = 0;
    for migration in migrations {
        let filename = migration.file_name().to_string_lossy().to_string();
        if applied.contains(&filename) {
            continue;
        }

        println!("  [*] Running: {}", filename.green());
        let sql = fs::read_to_string(migration.path())?;
        let up_sql: String = sql
            .lines()
            .take_while(|line| !line.trim().starts_with("-- DOWN"))
            .collect::<Vec<_>>()
            .join("\n");

        if !up_sql.trim().is_empty() {
            sqlx::query(&up_sql).execute(&pool).await?;
        }

        sqlx::query("INSERT INTO _yaiko_migrations (filename) VALUES (?)")
            .bind(&filename)
            .execute(&pool)
            .await?;

        ran += 1;
    }

    if ran == 0 {
        println!("[OK] All migrations already applied.");
    } else {
        println!("\n[OK] Applied {} migration(s).", ran);
    }

    Ok(())
}

async fn run_postgres(db_url: &str, migrations: &[fs::DirEntry]) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(db_url)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _yaiko_migrations (
            id SERIAL PRIMARY KEY,
            filename VARCHAR(255) NOT NULL UNIQUE,
            executed_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
        )",
    )
    .execute(&pool)
    .await?;

    let applied: Vec<String> =
        sqlx::query_as::<_, (String,)>("SELECT filename FROM _yaiko_migrations ORDER BY filename")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .map(|r| r.0)
            .collect();

    let mut ran = 0;
    for migration in migrations {
        let filename = migration.file_name().to_string_lossy().to_string();
        if applied.contains(&filename) {
            continue;
        }

        println!("  [*] Running: {}", filename.green());
        let sql = fs::read_to_string(migration.path())?;
        let up_sql: String = sql
            .lines()
            .take_while(|line| !line.trim().starts_with("-- DOWN"))
            .collect::<Vec<_>>()
            .join("\n");

        if !up_sql.trim().is_empty() {
            sqlx::query(&up_sql).execute(&pool).await?;
        }

        sqlx::query("INSERT INTO _yaiko_migrations (filename) VALUES ($1)")
            .bind(&filename)
            .execute(&pool)
            .await?;

        ran += 1;
    }

    if ran == 0 {
        println!("[OK] All migrations already applied.");
    } else {
        println!("\n[OK] Applied {} migration(s).", ran);
    }

    Ok(())
}

pub async fn rollback() -> anyhow::Result<()> {
    println!("[*] Rolling back last migration...");
    println!(
        "{} Rollback requires sqlx CLI: sqlx migrate revert\n",
        "!".yellow()
    );
    Ok(())
}

pub async fn status() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    println!("[*] Migration status:");

    let migrations_dir = Path::new("migrations");
    if !migrations_dir.exists() {
        println!("  No migrations directory found.\n");
        return Ok(());
    }

    let mut migrations: Vec<_> = fs::read_dir(migrations_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sql"))
        .filter(|e| !e.file_name().to_string_lossy().contains("_down"))
        .collect();

    migrations.sort_by_key(|e| e.file_name());

    if migrations.is_empty() {
        println!("  No migrations found.\n");
        return Ok(());
    }

    println!();
    for migration in &migrations {
        let filename = migration.file_name();
        println!("  {} {}", "○".cyan(), filename.to_string_lossy());
    }
    println!();

    Ok(())
}
