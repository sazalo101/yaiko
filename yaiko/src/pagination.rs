//! Opaque signed cursor pagination utilities.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    pub limit: usize,
    pub cursor: Option<String>,
}

impl PageRequest {
    pub fn new(limit: usize) -> Result<Self, PaginationError> {
        Self::with_bounds(limit, 1, 100)
    }

    pub fn with_bounds(
        limit: usize,
        minimum: usize,
        maximum: usize,
    ) -> Result<Self, PaginationError> {
        if minimum == 0 || minimum > maximum || limit < minimum || limit > maximum {
            return Err(PaginationError::InvalidLimit);
        }
        Ok(Self {
            limit,
            cursor: None,
        })
    }

    pub fn cursor(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CursorPayload<T> {
    value: T,
    expires_at: u64,
}

#[derive(Debug, Clone)]
pub struct CursorCodec {
    secret: Vec<u8>,
    ttl_secs: u64,
}

impl CursorCodec {
    pub fn new(secret: impl Into<Vec<u8>>, ttl_secs: u64) -> Self {
        Self {
            secret: secret.into(),
            ttl_secs,
        }
    }

    pub fn encode<T: Serialize>(&self, value: T) -> Result<String, PaginationError> {
        let payload = serde_json::to_vec(&CursorPayload {
            value,
            expires_at: now().saturating_add(self.ttl_secs),
        })
        .map_err(|_| PaginationError::Serialization)?;
        let encoded = URL_SAFE_NO_PAD.encode(&payload);
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| PaginationError::InvalidSecret)?;
        mac.update(encoded.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{}.{}", encoded, signature))
    }

    pub fn decode<T: DeserializeOwned>(&self, cursor: &str) -> Result<T, PaginationError> {
        let (encoded, signature) = cursor
            .split_once('.')
            .ok_or(PaginationError::InvalidCursor)?;
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| PaginationError::InvalidSecret)?;
        mac.update(encoded.as_bytes());
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| PaginationError::InvalidCursor)?;
        mac.verify_slice(&signature)
            .map_err(|_| PaginationError::InvalidCursor)?;
        let payload = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| PaginationError::InvalidCursor)?;
        let payload: CursorPayload<T> =
            serde_json::from_slice(&payload).map_err(|_| PaginationError::InvalidCursor)?;
        if now() >= payload.expires_at {
            return Err(PaginationError::ExpiredCursor);
        }
        Ok(payload.value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub limit: usize,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, limit: usize, next_cursor: Option<String>) -> Self {
        Self {
            has_more: next_cursor.is_some(),
            items,
            next_cursor,
            limit,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaginationError {
    InvalidLimit,
    InvalidCursor,
    ExpiredCursor,
    InvalidSecret,
    Serialization,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_and_decodes_opaque_cursors() {
        let codec = CursorCodec::new("secret", 60);
        let cursor = codec.encode("last-id").unwrap();
        assert!(!cursor.contains("last-id"));
        assert_eq!(codec.decode::<String>(&cursor).unwrap(), "last-id");
    }

    #[test]
    fn rejects_tampering_and_expiry() {
        let codec = CursorCodec::new("secret", 60);
        let cursor = codec.encode(42u64).unwrap();
        let mut tampered = cursor.clone();
        tampered.push('x');
        assert_eq!(
            codec.decode::<u64>(&tampered),
            Err(PaginationError::InvalidCursor)
        );
        let expired = CursorCodec::new("secret", 0).encode(42u64).unwrap();
        assert_eq!(
            codec.decode::<u64>(&expired),
            Err(PaginationError::ExpiredCursor)
        );
    }

    #[test]
    fn enforces_limits_and_exposes_page_metadata() {
        assert_eq!(PageRequest::new(0), Err(PaginationError::InvalidLimit));
        assert_eq!(
            PageRequest::with_bounds(101, 1, 100),
            Err(PaginationError::InvalidLimit)
        );
        let page = Page::new(vec![1, 2], 2, Some("next".into()));
        assert!(page.has_more);
        assert_eq!(page.next_cursor.as_deref(), Some("next"));
    }
}
