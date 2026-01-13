//! Configuration module for Yaiko applications
//!
//! Provides typed configuration loading from files and environment variables.

use serde::Deserialize;
use std::path::Path;

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    3000
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_type")]
    pub db_type: String,
    pub url: Option<String>,
}

fn default_db_type() -> String {
    "postgres".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_type: default_db_type(),
            url: None,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub cors_origins: Vec<String>,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_requests: u32,
    #[serde(default = "default_rate_window")]
    pub rate_limit_window_secs: u64,
    #[serde(default = "default_csrf")]
    pub csrf_enabled: bool,
}

fn default_rate_limit() -> u32 {
    100
}
fn default_rate_window() -> u64 {
    60
}
fn default_csrf() -> bool {
    true
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_origins: vec![],
            rate_limit_requests: default_rate_limit(),
            rate_limit_window_secs: default_rate_window(),
            csrf_enabled: default_csrf(),
        }
    }
}

/// SEO configuration
#[derive(Debug, Clone, Deserialize)]
pub struct SeoConfig {
    #[serde(default = "default_true")]
    pub robots_txt_enabled: bool,
    #[serde(default = "default_true")]
    pub sitemap_enabled: bool,
    #[serde(default = "default_changefreq")]
    pub sitemap_changefreq: String,
}

fn default_true() -> bool {
    true
}
fn default_changefreq() -> String {
    "weekly".to_string()
}

impl Default for SeoConfig {
    fn default() -> Self {
        Self {
            robots_txt_enabled: true,
            sitemap_enabled: true,
            sitemap_changefreq: default_changefreq(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "pretty".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

/// Main application settings
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub seo: SeoConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Settings {
    /// Load settings from yaiko.toml and environment variables
    pub fn load() -> Result<Self, config::ConfigError> {
        Self::load_from("yaiko.toml")
    }

    /// Load settings from a specific config file
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self, config::ConfigError> {
        let builder = config::Config::builder()
            // Start with defaults
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3000)?
            // Load from config file if it exists
            .add_source(config::File::from(path.as_ref()).required(false))
            // Override with environment variables (YAIKO_ prefix)
            .add_source(
                config::Environment::with_prefix("YAIKO")
                    .separator("__")
                    .try_parsing(true),
            );

        builder.build()?.try_deserialize()
    }

    /// Get the server address as a string
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
