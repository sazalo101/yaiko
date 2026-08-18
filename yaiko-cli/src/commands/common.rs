use std::path::{Path, PathBuf};

use anyhow::{bail, Context};

pub const SUPPORTED_DATABASES: &[&str] = &["postgres", "sqlite"];

pub fn ensure_supported_database(database: &str) -> anyhow::Result<()> {
    if SUPPORTED_DATABASES.contains(&database) {
        return Ok(());
    }

    bail!(
        "Unsupported database '{}'. Supported values: {}.",
        database,
        SUPPORTED_DATABASES.join(", ")
    )
}

pub fn ensure_yaiko_project(project_dir: &Path) -> anyhow::Result<()> {
    let cargo = project_dir.join("Cargo.toml");
    let config = project_dir.join("yaiko.toml");

    if !cargo.exists() || !config.exists() {
        bail!(
            "Not a Yaiko project in '{}'. Expected both Cargo.toml and yaiko.toml.",
            project_dir.display()
        );
    }

    Ok(())
}

pub fn cargo_manifest(project_dir: &Path) -> PathBuf {
    project_dir.join("Cargo.toml")
}

pub fn load_project_name(project_dir: &Path) -> anyhow::Result<String> {
    let manifest = std::fs::read_to_string(cargo_manifest(project_dir)).with_context(|| {
        format!(
            "Failed to read '{}'.",
            cargo_manifest(project_dir).display()
        )
    })?;
    let parsed: toml::Value = toml::from_str(&manifest).with_context(|| {
        format!(
            "Failed to parse '{}'.",
            cargo_manifest(project_dir).display()
        )
    })?;

    parsed
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .map(str::to_string)
        .context("Cargo.toml is missing package.name")
}

pub fn resolve_yaiko_core_dependency() -> anyhow::Result<String> {
    if let Ok(path) = std::env::var("YAIKO_CORE_PATH") {
        let canonical = PathBuf::from(&path)
            .canonicalize()
            .with_context(|| format!("YAIKO_CORE_PATH does not exist: {}", path))?;
        validate_yaiko_core_manifest(&canonical)?;
        return Ok(format!(
            "yaiko-core = {{ path = \"{}\" }}",
            normalize_path(&canonical)
        ));
    }

    let cli_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let framework_dir = cli_manifest_dir
        .parent()
        .map(|dir| dir.join("yaiko"))
        .context("Failed to resolve Yaiko framework path from CLI manifest dir")?;

    if framework_dir.exists() {
        let canonical = framework_dir
            .canonicalize()
            .with_context(|| format!("Failed to canonicalize '{}'.", framework_dir.display()))?;
        validate_yaiko_core_manifest(&canonical)?;
        return Ok(format!(
            "yaiko-core = {{ path = \"{}\" }}",
            normalize_path(&canonical)
        ));
    }

    bail!(
        "Unable to resolve yaiko-core. Set YAIKO_CORE_PATH to a local yaiko-core crate before running 'yaiko init'."
    )
}

fn validate_yaiko_core_manifest(path: &Path) -> anyhow::Result<()> {
    let manifest = path.join("Cargo.toml");
    if !manifest.exists() {
        bail!(
            "Resolved yaiko-core path '{}' does not contain Cargo.toml.",
            path.display()
        );
    }
    Ok(())
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
