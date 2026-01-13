pub mod app;
pub mod router;
pub mod handler;
pub mod middleware;
pub mod request;
pub mod response;
pub mod server;
pub mod static_files;
pub mod template;
pub mod auth;
pub mod cache;
pub mod compression;
pub mod database;
pub mod file_upload;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod session;

// Production modules
pub mod config;
pub mod error;
pub mod seo;
pub mod security;
pub mod websocket;
pub mod jobs;
pub mod health;

// Validation module
pub mod validation;

// Developer experience modules
pub mod logging;
pub mod testing;

// Optional dev module for development utilities
#[cfg(feature = "dev")]
pub mod dev;

pub use app::App;
pub use router::{Router, Route};
pub use handler::Handler;
pub use middleware::Middleware;
pub use request::Request;
pub use response::Response;
pub use server::Server;
pub use config::Settings;
pub use error::{AppError, AppResult};
pub use static_files::StaticFiles;
pub use database::Database;
pub use session::{Session, SessionMiddleware, MemorySessionStore, SessionStore};

// Security exports
pub use security::{SecurityHeaders, RateLimiter, CsrfProtection};

// Auth exports
pub use auth::{JwtAuth, AuthMiddleware, Claims};

// Logging exports
pub use logging::{LoggingMiddleware, init_tracing};

// Testing exports
pub use testing::{TestClient, TestResponse};

// File upload exports
pub use file_upload::{FileUpload, parse_multipart};

// Compression exports
pub use compression::CompressionMiddleware;

// WebSocket exports
pub use websocket::{WebSocketManager, WebSocketConnection, is_websocket_upgrade};

// Background jobs exports
pub use jobs::{JobQueue, Job};

// Health check exports
pub use health::{HealthCheck, DetailedHealthCheck};

// Re-export commonly used types
pub use hyper::{Body, Method, StatusCode};
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};
pub use async_trait::async_trait;
pub use tracing;

// Password hashing helpers
pub use bcrypt;

/// Hash a password using bcrypt with default cost
pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

/// Verify a password against a bcrypt hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}