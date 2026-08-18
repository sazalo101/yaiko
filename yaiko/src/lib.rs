pub mod api_keys;
pub mod app;
pub mod audit;
pub mod auth;
pub mod cache;
pub mod coalescing;
pub mod compression;
pub mod compression_policy;
pub mod cookie_policy;
pub mod cors;
pub mod csp;
pub mod data_transfer;
pub mod database;
pub mod delivery;
pub mod delivery_observability;
pub mod encryption;
pub mod file_upload;
pub mod handler;
pub mod media_delivery;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod metrics_registry;
pub mod middleware;
pub mod migrations;
pub mod notifications;
pub mod rate_limit;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod session;
pub mod static_files;
pub mod storage;
pub mod task_observability;
pub mod task_scheduler;
pub mod template;
pub mod tenant;
pub mod upload_policy;

// Production modules
pub mod config;
pub mod error;
pub mod extract;
pub mod feature_flags;
pub mod fixtures;
pub mod health;
pub mod health_registry;
pub mod http_client;
pub mod i18n;
pub mod idempotency;
pub mod jobs;
pub mod schema_validation;
pub mod search;
pub mod security;
pub mod seo;
pub mod webhook;
pub mod websocket;

// Validation module
pub mod validation;
pub mod versioning;

// Developer experience modules
pub mod lifecycle;
pub mod logging;
pub mod oauth;
pub mod openapi;
pub mod pagination;
pub mod privacy;
pub mod testing;
pub mod tracing_context;

// Optional dev module for development utilities
#[cfg(feature = "dev")]
pub mod dev;

pub use api_keys::{
    sign_request, verify_request_signature, ApiKeyError, ApiKeyStore, IssuedApiKey,
};
pub use app::App;
pub use audit::{AuditEvent, AuditEventBuilder, AuditQuery, MemoryAuditSink};
pub use cache::{Cache, CacheNamespace, CacheResult, CacheStore, MemoryCache};
pub use coalescing::{stale_window, CachedValue, CoalesceError, FlightOwner, RequestCoalescer};
pub use compression_policy::{CompressionDecision, CompressionEncoding, CompressionPolicy};
pub use config::Settings;
pub use cookie_policy::{CookieError, CookiePolicy, SignedCookieCodec};
pub use cors::{CorsDecision, CorsDenial, CorsPolicy, OriginRule};
pub use csp::{ContentSecurityPolicy, CspNonce, SecurityPolicyHeaders};
pub use data_transfer::{
    export_csv, export_json, import_json, import_json_value, safe_filename, DataFormat,
    DataTransferError, ExportPayload,
};
pub use database::Database;
pub use delivery::{unix_seconds, DeliveryError, DeliveryRecord, DeliveryScheduler, DeliveryState};
pub use delivery_observability::{DeliveryObservation, DeliveryObserver, DeliveryOutcome};
pub use encryption::{EncryptedEnvelope, EncryptionError, KeyRing};
pub use error::{AppError, AppResult, ErrorCode, ErrorDetails, ErrorDocument};
pub use extract::{Form, FromRequest, Json, Path, Query};
pub use feature_flags::{FeatureChange, FeatureFlagStore, FeatureSnapshot, FeatureValue};
pub use fixtures::{CleanupGuard, FixtureFactory};
pub use handler::Handler;
pub use media_delivery::{ByteRange, MediaAsset, MediaDelivery, MediaResponse, RangeError};
pub use metrics_registry::{
    MetricError, MetricKind, MetricValue, MetricsRegistry, MetricsSnapshot,
};
pub use middleware::Middleware;
pub use migrations::{AppliedMigration, Migration, MigrationError, MigrationPlan, MigrationRunner};
pub use notifications::{
    DeliveryResult, NotificationEnvelope, NotificationTemplate, RetryPolicy, TemplateError,
};
pub use oauth::{
    OAuthCallback, OAuthProvider, OAuthState, OAuthTokenResponse, OidcUserInfo, PkceChallenge,
};
pub use openapi::{OpenApiDocument, OpenApiHandler, OpenApiOperation, OpenApiResponse};
pub use pagination::{CursorCodec, Page, PageRequest, PaginationError};
pub use privacy::{PrivacyPolicy, SecretString};
pub use rate_limit::{MemoryRateLimiter, QuotaPolicy, RateLimitDecision};
pub use request::Request;
pub use response::Response;
pub use router::{Route, Router};
pub use schema_validation::{FieldError, SchemaRule, SchemaValidator, ValidationReport};
pub use search::{Filter, FilterOperator, QueryBuilder, QueryError, QuerySpec, SortField};
pub use server::Server;
pub use session::{MemorySessionStore, Session, SessionHandle, SessionMiddleware, SessionStore};
pub use static_files::StaticFiles;
pub use storage::{LocalStorage, Storage, StorageResult};
pub use upload_policy::{
    checksum, sanitize_filename, TempUploadGuard, UploadError, UploadMetadata, UploadPolicy,
};

// Security exports
pub use security::{CsrfProtection, RateLimiter, SecurityHeaders};

// Auth exports
pub use auth::{
    login_session, logout_session, require_role, AuthMiddleware, Claims, JwtAuth, SessionAuth,
};

// Logging exports
pub use lifecycle::{LifecycleError, ServiceRegistry, ServiceState, ShutdownReport, ShutdownToken};
pub use logging::{init_tracing, LoggingMiddleware};

// Testing exports
pub use task_observability::{TaskObservation, TaskObserver, TaskOutcome};
pub use task_scheduler::{ScheduleError, ScheduledTask, ScheduledTaskState, TaskScheduler};
pub use tenant::{TenantContext, TenantError, TenantId, TenantQuota, TenantResolver};
pub use testing::{TestClient, TestResponse};
pub use tracing_context::{Sampler, Span, SpanEvent, TraceContext};

// File upload exports
pub use file_upload::{parse_multipart, FileUpload};

// Compression exports
pub use compression::CompressionMiddleware;

// WebSocket exports
pub use versioning::{ApiVersion, VersionDecision, VersionError, VersionNegotiator};
pub use webhook::{PreparedWebhook, WebhookError, WebhookEvent, WebhookVerifier};
pub use websocket::{is_websocket_upgrade, WebSocketConnection, WebSocketManager};

// Background jobs exports
pub use idempotency::{
    fingerprint, ClaimOutcome, IdempotencyError, IdempotencyLease, MemoryIdempotencyStore,
    StoredResponse,
};
pub use jobs::{DeadLetter, Job, JobQueue};

// Health check exports
pub use health::{DetailedHealthCheck, HealthCheck, LivenessCheck, ReadinessCheck};
pub use health_registry::{
    DependencyHealthHandler, DependencyHealthRegistry, HealthReport, ProbeResult,
};
pub use http_client::{
    HttpClientError, HttpRequestSpec, HttpResponse, HttpRetryPolicy, OutboundHttpClient,
};
pub use i18n::{Catalog, I18nError, Locale, Translator};

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
