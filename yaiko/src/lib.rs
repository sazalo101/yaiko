pub mod api_keys;
pub mod app;
pub mod audio_processing;
pub mod audit;
pub mod audit_sinks;
pub mod auth;
pub mod body_stream;
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
pub mod distributed_lease;
pub mod distributed_rate_limit;
pub mod encryption;
pub mod file_upload;
pub mod handler;
pub mod media_access;
pub mod media_annotations;
pub mod media_asset_versioning;
pub mod media_audio_format;
pub mod media_cache_key;
pub mod media_color;
pub mod media_content_type;
pub mod media_cursor;
pub mod media_delivery;
pub mod media_delivery_policy;
pub mod media_editor;
pub mod media_export;
pub mod media_export_presets;
pub mod media_graph;
pub mod media_limits;
pub mod media_loudness;
pub mod media_manifest;
pub mod media_metadata;
pub mod media_notification_preferences;
pub mod media_output;
pub mod media_presence;
pub mod media_preview;
pub mod media_processing;
pub mod media_progress;
pub mod media_project_templates;
pub mod media_quota;
pub mod media_retention;
pub mod media_retry;
pub mod media_review;
pub mod media_revision;
pub mod media_scenes;
pub mod media_selection_lock;
pub mod media_snapshot;
pub mod media_task;
pub mod media_timeline;
pub mod media_timestamps;
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod metrics_registry;
pub mod middleware;
pub mod migrations;
pub mod notifications;
pub mod rate_limit;
pub mod rbac;
pub mod request;
pub mod resilience;
pub mod response;
pub mod resumable_upload;
pub mod robots;
pub mod router;
pub mod server;
pub mod session;
pub mod singleflight_cache;
pub mod static_files;
pub mod storage;
pub mod subtitle_style;
pub mod task_observability;
pub mod task_results;
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
pub mod media_waveform;
pub mod media_webhook;
pub mod schema_validation;
pub mod search;
pub mod security;
pub mod seo;
pub mod waveform_serialization;
pub mod webhook;
pub mod websocket;
pub mod ws_channels;
pub mod ws_protocol;

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
pub use audio_processing::{AudioProcessingError, AudioProcessingSpec, Ducking, LoudnessTarget};
pub use audit::{AuditEvent, AuditEventBuilder, AuditQuery, MemoryAuditSink};
pub use audit_sinks::{
    AuditFilter, AuditSink, AuditSinkError, BufferedAuditSink, FilteredAuditSink, JsonlAuditSink,
};
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
pub use distributed_lease::{DistributedLease, Lease, LeaseError, LeaseStore, MemoryLeaseStore};
pub use distributed_rate_limit::{
    DistributedDecision, DistributedRateLimitError, DistributedRateLimiter, MemoryRateLimitStore,
    RateLimitStore, WindowPolicy,
};
pub use encryption::{EncryptedEnvelope, EncryptionError, KeyRing};
pub use error::{AppError, AppResult, ErrorCode, ErrorDetails, ErrorDocument};
pub use extract::{Form, FromRequest, Json, Path, Query};
pub use feature_flags::{FeatureChange, FeatureFlagStore, FeatureSnapshot, FeatureValue};
pub use fixtures::{CleanupGuard, FixtureFactory};
pub use handler::Handler;
pub use media_access::{MediaAccessClaims, MediaAccessError, MediaAccessTokens};
pub use media_annotations::{
    AnnotationError, AnnotationReply, AnnotationSnapshot, AnnotationState, MediaAnnotation,
    MediaAnnotationStore,
};
pub use media_asset_versioning::{
    AssetVersionError, AssetVersionSnapshot, AssetVersionState, MediaAssetVersion,
    MediaAssetVersionStore,
};
pub use media_audio_format::{
    AudioChannels, AudioFormat, AudioFormatError, AudioFormatPolicy, AudioSampleRate,
};
pub use media_cache_key::{CacheKeyError, ThumbnailCacheKey};
pub use media_color::{
    ColorPolicyError, ColorRange, ColorSpace, MediaColorMetadata, MediaColorPolicy, PixelFormat,
    Transfer,
};
pub use media_content_type::{sniff_media, ContentTypeError, MediaContentType, MediaKind};
pub use media_cursor::{
    CursorError, CursorPosition, CursorSnapshot, CursorState, MediaCursorStore,
};
pub use media_delivery::{ByteRange, MediaAsset, MediaDelivery, MediaResponse, RangeError};
pub use media_delivery_policy::{DeliveryHeaders, DeliveryPolicyError, MediaDeliveryPolicy};
pub use media_editor::{EditorError, EditorSnapshot, MediaEditorStore};
pub use media_export::{AudioCodec, Container, ExportError, ExportProfile, VideoCodec};
pub use media_export_presets::{ExportPresetError, MediaExportPreset, MediaExportPresetStore};
pub use media_graph::{MediaGraph, MediaGraphError, MediaNode, NodeState};
pub use media_limits::{MediaFacts, MediaLimits, MediaLimitsError};
pub use media_loudness::{ChannelLayout, LoudnessError, LoudnessMetadata, LoudnessPolicy};
pub use media_manifest::{ManifestError, MediaManifest, MediaManifestStore};
pub use media_metadata::{
    parse_and_validate, MediaMetadata, MediaMetadataError, MediaMetadataRequirements,
};
pub use media_notification_preferences::{
    DigestMode, MediaNotificationPreference, MediaNotificationPreferenceStore, NotificationChannel,
    NotificationPreferenceError,
};
pub use media_output::{MediaOutputError, MediaOutputValidator, ValidatedMediaOutput};
pub use media_presence::{
    MediaPresenceStore, Participant, PresenceError, PresenceRole, PresenceSnapshot,
};
pub use media_preview::{PreviewError, PreviewFormat, ThumbnailSpec};
pub use media_processing::{CaptionTrack, FfmpegJobSpec, MediaError, MediaPath, MusicTrack};
pub use media_progress::{MediaProgressEvent, MediaProgressStore, ProgressError, ProgressSnapshot};
pub use media_project_templates::{
    MaterializedTemplate, ProjectTemplate, ProjectTemplateStore, TemplateError,
};
pub use media_quota::{MediaQuota, QuotaError, QuotaLimit, QuotaReservation, QuotaUsage};
pub use media_retention::{
    CleanupReport, MediaArtifact, MediaRetention, RetentionError, RetentionPolicy,
};
pub use media_retry::{MediaFailure, MediaRetryPolicy, RetryDecision, RetryError};
pub use media_review::{MediaReviewStore, ReviewError, ReviewRequest, ReviewSnapshot, ReviewState};
pub use media_revision::{MediaRevisionStore, RevisionDiff, RevisionError, RevisionRecord};
pub use media_scenes::{validate_scenes, SceneBoundary, SceneError, SceneMarker};
pub use media_selection_lock::{
    MediaSelectionLockStore, SelectionLock, SelectionLockError, SelectionLockSnapshot,
};
pub use media_snapshot::{
    serialize_project_snapshot, MediaProjectSnapshot, SerializedProjectSnapshot, SnapshotAsset,
    SnapshotError,
};
pub use media_task::{MediaTask, MediaTaskError};
pub use media_timeline::{CaptionOverlay, TimelineClip, TimelineError, TimelineSpec, Transition};
pub use media_timestamps::{
    validate_and_normalize, TimestampError, TimestampPolicy, TimestampReport,
};
pub use metrics_registry::{
    MetricError, MetricKind, MetricValue, MetricsRegistry, MetricsSnapshot,
};
pub use middleware::Middleware;
pub use migrations::{AppliedMigration, Migration, MigrationError, MigrationPlan, MigrationRunner};
pub use notifications::{DeliveryResult, NotificationEnvelope, NotificationTemplate, RetryPolicy};
pub use oauth::{
    OAuthCallback, OAuthProvider, OAuthState, OAuthTokenResponse, OidcUserInfo, PkceChallenge,
};
pub use openapi::{OpenApiDocument, OpenApiHandler, OpenApiOperation, OpenApiResponse};
pub use pagination::{CursorCodec, Page, PageRequest, PaginationError};
pub use privacy::{PrivacyPolicy, SecretString};
pub use rate_limit::{MemoryRateLimiter, QuotaPolicy, RateLimitDecision};
pub use rbac::{RbacError, RbacPolicy, Role};
pub use request::Request;
pub use resilience::{Bulkhead, BulkheadPermit, CircuitBreaker, CircuitState, ResilienceError};
pub use response::Response;
pub use resumable_upload::{CompletedUpload, ResumableUploadStore, UploadSessionError};
pub use robots::{RobotsError, RobotsTxt};
pub use router::{Route, Router};
pub use schema_validation::{FieldError, SchemaRule, SchemaValidator, ValidationReport};
pub use search::{Filter, FilterOperator, QueryBuilder, QueryError, QuerySpec, SortField};
pub use server::Server;
pub use session::{MemorySessionStore, Session, SessionHandle, SessionMiddleware, SessionStore};
pub use singleflight_cache::{CacheError, CacheLookup, SingleFlightCache};
pub use static_files::StaticFiles;
pub use storage::{LocalStorage, Storage, StorageResult};
pub use subtitle_style::{SubtitlePosition, SubtitleStyle, SubtitleStyleError};
pub use upload_policy::{
    checksum, sanitize_filename, TempUploadGuard, UploadError, UploadMetadata, UploadPolicy,
};

// Security exports
pub use security::{CsrfProtection, RateLimiter, SecurityHeaders};

// Auth exports
pub use auth::{
    login_session, logout_session, require_role, AuthMiddleware, Claims, JwtAuth, SessionAuth,
};
pub use body_stream::{read_bounded, BodyCancellation, BodyReadReport, BodyStreamError};

// Logging exports
pub use lifecycle::{LifecycleError, ServiceRegistry, ServiceState, ShutdownReport, ShutdownToken};
pub use logging::{init_tracing, LoggingMiddleware};

// Testing exports
pub use task_observability::{TaskObservation, TaskObserver, TaskOutcome};
pub use task_results::{TaskResult, TaskResultError, TaskResultStore, TaskState};
pub use task_scheduler::{ScheduleError, ScheduledTask, ScheduledTaskState, TaskScheduler};
pub use tenant::{TenantContext, TenantError, TenantId, TenantQuota, TenantResolver};
pub use testing::{TestClient, TestResponse};
pub use tracing_context::{Sampler, Span, SpanEvent, TraceContext};

// File upload exports
pub use file_upload::{parse_multipart, FileUpload};

// Compression exports
pub use compression::CompressionMiddleware;

// WebSocket exports
pub use media_waveform::{Chapter, ChapterTrack, Waveform, WaveformError};
pub use media_webhook::{MediaEventKind, MediaWebhookError, MediaWebhookEvent, MediaWebhookSigner};
pub use versioning::{ApiVersion, VersionDecision, VersionError, VersionNegotiator};
pub use waveform_serialization::{
    serialize_waveform, SerializedWaveform, WaveformChapter, WaveformSerializationError,
};
pub use webhook::{PreparedWebhook, WebhookError, WebhookEvent, WebhookVerifier};
pub use websocket::{is_websocket_upgrade, WebSocketConnection, WebSocketManager};
pub use ws_channels::{ChannelError, ChannelMessage, ChannelRegistry, CloseReason, HeartbeatState};
pub use ws_protocol::{WsEnvelope, WsProtocol, WsProtocolError};

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
