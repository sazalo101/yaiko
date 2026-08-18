//! Structured error handling for Yaiko applications.
//!
//! The public error contract is stable across handlers and transports:
//! machine-readable `code`, HTTP `status`, safe `message`, and an optional
//! request correlation ID.

use crate::{Response, StatusCode};
use serde::Serialize;
use thiserror::Error;

/// Stable machine-readable error identifiers exposed by Yaiko responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    NotFound,
    BadRequest,
    Unauthorized,
    Forbidden,
    Conflict,
    #[serde(rename = "VALIDATION_ERROR")]
    Validation,
    #[serde(rename = "RATE_LIMIT_EXCEEDED")]
    RateLimitExceeded,
    #[serde(rename = "DATABASE_ERROR")]
    Database,
    #[serde(rename = "INTERNAL_ERROR")]
    Internal,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "NOT_FOUND",
            Self::BadRequest => "BAD_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::Conflict => "CONFLICT",
            Self::Validation => "VALIDATION_ERROR",
            Self::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            Self::Database => "DATABASE_ERROR",
            Self::Internal => "INTERNAL_ERROR",
        }
    }
}

/// Structured JSON error document returned by Yaiko.
#[derive(Debug, Serialize)]
pub struct ErrorDocument {
    pub error: ErrorDetails,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub code: ErrorCode,
    pub message: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Application error type that automatically maps to a safe HTTP response.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Database error: {0}")]
    Database(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl AppError {
    /// Return the stable machine-readable code for this error.
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::NotFound(_) => ErrorCode::NotFound,
            Self::BadRequest(_) => ErrorCode::BadRequest,
            Self::Unauthorized(_) => ErrorCode::Unauthorized,
            Self::Forbidden(_) => ErrorCode::Forbidden,
            Self::Conflict(_) => ErrorCode::Conflict,
            Self::Validation(_) => ErrorCode::Validation,
            Self::RateLimitExceeded => ErrorCode::RateLimitExceeded,
            Self::Database(_) => ErrorCode::Database,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    /// Get the HTTP status code for this error.
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::BadRequest(_) | Self::Validation(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::RateLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Return a client-safe message. Internal implementation details are never exposed.
    pub fn public_message(&self) -> String {
        match self {
            Self::NotFound(message)
            | Self::BadRequest(message)
            | Self::Unauthorized(message)
            | Self::Forbidden(message)
            | Self::Conflict(message)
            | Self::Validation(message) => message.clone(),
            Self::RateLimitExceeded => "Too many requests".to_string(),
            Self::Database(_) => "A database error occurred".to_string(),
            Self::Internal(_) => "An internal server error occurred".to_string(),
        }
    }

    /// Convert this error into the standard JSON response without a request ID.
    pub fn into_response(self) -> Response {
        self.into_response_with_request_id(None)
    }

    /// Convert this error into the standard JSON response with correlation metadata.
    pub fn into_response_with_request_id(self, request_id: Option<&str>) -> Response {
        self.response_with_request_id(request_id)
    }

    /// Build a response by borrowing the error, which is useful for boxed errors.
    pub fn response_with_request_id(&self, request_id: Option<&str>) -> Response {
        let status = self.status_code();
        let request_id = request_id.map(str::to_owned);
        let document = ErrorDocument {
            error: ErrorDetails {
                code: self.code(),
                message: self.public_message(),
                status: status.as_u16(),
                request_id: request_id.clone(),
            },
        };

        let mut response = Response::new()
            .status(status)
            .header("Content-Type", "application/json")
            .json(&document)
            .expect("structured error serialization should not fail");
        if let Some(request_id) = request_id {
            response = response.header("X-Request-ID", &request_id);
        }
        response
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound("Resource not found".to_string()),
            _ => Self::Database(err.to_string()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::BadRequest(format!("JSON error: {}", err))
    }
}

/// Result type alias for Yaiko applications.
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::to_bytes;

    #[tokio::test]
    async fn structured_error_contains_stable_metadata_and_request_id() {
        let response = AppError::Validation("email is invalid".to_string())
            .into_response_with_request_id(Some("req-123"));
        let body = to_bytes(response.body).await.unwrap();
        let document: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers.get("X-Request-ID"),
            Some(&"req-123".to_string())
        );
        assert_eq!(document["error"]["code"], "VALIDATION_ERROR");
        assert_eq!(document["error"]["status"], 400);
        assert_eq!(document["error"]["message"], "email is invalid");
        assert_eq!(document["error"]["request_id"], "req-123");
    }

    #[tokio::test]
    async fn internal_details_are_not_exposed() {
        let response = AppError::Internal("database password leaked".to_string()).into_response();
        let body = to_bytes(response.body).await.unwrap();
        let document: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(document["error"]["code"], "INTERNAL_ERROR");
        assert_eq!(
            document["error"]["message"],
            "An internal server error occurred"
        );
        assert!(!document.to_string().contains("database password leaked"));
    }
}
