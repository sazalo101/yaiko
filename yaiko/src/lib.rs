pub mod app;
pub mod auth;
pub mod cache;
pub mod compression;
pub mod database;
pub mod file_upload;
pub mod handler;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod middleware;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod session;
pub mod static_files;
pub mod storage;
pub mod template;

// Production modules
pub mod config;
pub mod error;
pub mod extract;
pub mod health;
pub mod jobs;
pub mod security;
pub mod seo;
pub mod websocket;

// Validation module
pub mod validation;

// Developer experience modules
pub mod logging;
pub mod openapi;
pub mod testing;

// Optional dev module for development utilities
#[cfg(feature = "dev")]
pub mod dev;

pub use app::App;
pub use cache::{Cache, CacheNamespace, CacheResult, CacheStore, MemoryCache};
pub use config::Settings;
pub use database::Database;
pub use error::{AppError, AppResult, ErrorCode, ErrorDetails, ErrorDocument};
pub use extract::{Form, FromRequest, Json, Path, Query};
pub use handler::Handler;
pub use middleware::Middleware;
pub use openapi::{OpenApiDocument, OpenApiHandler, OpenApiOperation, OpenApiResponse};
pub use request::Request;
pub use response::Response;
pub use router::{Route, Router};
pub use server::Server;
pub use session::{MemorySessionStore, Session, SessionHandle, SessionMiddleware, SessionStore};
pub use static_files::StaticFiles;
pub use storage::{LocalStorage, Storage, StorageResult};

// Security exports
pub use security::{CsrfProtection, RateLimiter, SecurityHeaders};

// Auth exports
pub use auth::{
    login_session, logout_session, require_role, AuthMiddleware, Claims, JwtAuth, SessionAuth,
};

// Logging exports
pub use logging::{init_tracing, LoggingMiddleware};

// Testing exports
pub use testing::{TestClient, TestResponse};

// File upload exports
pub use file_upload::{parse_multipart, FileUpload};

// Compression exports
pub use compression::CompressionMiddleware;

// WebSocket exports
pub use websocket::{is_websocket_upgrade, WebSocketConnection, WebSocketManager};

// Background jobs exports
pub use jobs::{DeadLetter, Job, JobQueue};

// Health check exports
pub use health::{DetailedHealthCheck, HealthCheck, LivenessCheck, ReadinessCheck};

// Re-export commonly used types
pub use async_trait::async_trait;
pub use hyper;
pub use hyper::{Body, Method, StatusCode};
pub use serde::{Deserialize, Serialize};
pub use serde_json::{json, Value};
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;
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
