//! Signed webhook event primitives and delivery preparation helpers.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WebhookEvent {
    pub id: String,
    pub event_type: String,
    pub created_at: u64,
    pub payload: Value,
}

impl WebhookEvent {
    pub fn new(id: impl Into<String>, event_type: impl Into<String>, payload: Value) -> Self {
        Self {
            id: id.into(),
            event_type: event_type.into(),
            created_at: now_unix(),
            payload,
        }
    }

    pub fn body(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWebhook {
    pub url: String,
    pub body: Vec<u8>,
    pub signature: String,
    pub event_id: String,
}

#[derive(Clone)]
pub struct WebhookVerifier {
    secret: Arc<Vec<u8>>,
    tolerance: Duration,
    seen: Arc<Mutex<HashMap<String, u64>>>,
}

impl WebhookVerifier {
    pub fn new(secret: impl AsRef<[u8]>, tolerance: Duration) -> Self {
        Self {
            secret: Arc::new(secret.as_ref().to_vec()),
            tolerance,
            seen: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn signature(&self, timestamp: u64, body: &[u8]) -> Result<String, &'static str> {
        let mut mac =
            HmacSha256::new_from_slice(self.secret.as_slice()).map_err(|_| "invalid secret")?;
        mac.update(format!("{}.", timestamp).as_bytes());
        mac.update(body);
        Ok(format!(
            "t={},v1={}",
            timestamp,
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        ))
    }

    pub async fn verify(&self, header: &str, body: &[u8], now: u64) -> Result<u64, WebhookError> {
        let (timestamp, provided) = parse_signature(header)?;
        let tolerance = self.tolerance.as_secs();
        if now.abs_diff(timestamp) > tolerance {
            return Err(WebhookError::Expired);
        }
        let expected = self
            .signature(timestamp, body)
            .map_err(|_| WebhookError::InvalidSecret)?;
        let expected = expected
            .split_once("v1=")
            .map(|(_, value)| value)
            .unwrap_or_default();
        if !constant_time_eq(expected.as_bytes(), provided.as_bytes()) {
            return Err(WebhookError::InvalidSignature);
        }
        let replay_key = format!("{}:{}", timestamp, provided);
        let mut seen = self.seen.lock().await;
        seen.retain(|_, seen_at| now.saturating_sub(*seen_at) <= tolerance);
        if seen.insert(replay_key, now).is_some() {
            return Err(WebhookError::Replay);
        }
        Ok(timestamp)
    }

    pub async fn prepare(
        &self,
        url: impl Into<String>,
        event: &WebhookEvent,
        timestamp: u64,
    ) -> Result<PreparedWebhook, &'static str> {
        let body = event.body().map_err(|_| "event serialization failed")?;
        let signature = self.signature(timestamp, &body)?;
        Ok(PreparedWebhook {
            url: url.into(),
            body,
            signature,
            event_id: event.id.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WebhookError {
    #[error("malformed webhook signature")]
    MalformedSignature,
    #[error("webhook signature expired")]
    Expired,
    #[error("invalid webhook secret")]
    InvalidSecret,
    #[error("invalid webhook signature")]
    InvalidSignature,
    #[error("webhook signature replay detected")]
    Replay,
}

fn parse_signature(header: &str) -> Result<(u64, &str), WebhookError> {
    let mut timestamp = None;
    let mut signature = None;
    for part in header.split(',') {
        let Some((key, value)) = part.trim().split_once('=') else {
            continue;
        };
        match key {
            "t" => timestamp = value.parse::<u64>().ok(),
            "v1" if !value.is_empty() => signature = Some(value),
            _ => {}
        }
    }
    match (timestamp, signature) {
        (Some(timestamp), Some(signature)) => Ok((timestamp, signature)),
        _ => Err(WebhookError::MalformedSignature),
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn signs_verifies_and_rejects_replays() {
        let verifier = WebhookVerifier::new("secret", Duration::from_secs(60));
        let event = WebhookEvent::new("evt-1", "video.ready", json!({"clip": 7}));
        let body = event.body().unwrap();
        let header = verifier.signature(100, &body).unwrap();
        assert_eq!(verifier.verify(&header, &body, 100).await, Ok(100));
        assert_eq!(
            verifier.verify(&header, &body, 100).await,
            Err(WebhookError::Replay)
        );
    }

    #[tokio::test]
    async fn rejects_tampering_and_stale_timestamps() {
        let verifier = WebhookVerifier::new("secret", Duration::from_secs(60));
        let body = br#"{"ok":true}"#;
        let header = verifier.signature(100, body).unwrap();
        assert_eq!(
            verifier.verify(&header, br#"{"ok":false}"#, 100).await,
            Err(WebhookError::InvalidSignature)
        );
        assert_eq!(
            verifier.verify(&header, body, 161).await,
            Err(WebhookError::Expired)
        );
    }

    #[tokio::test]
    async fn prepares_signed_event_delivery() {
        let verifier = WebhookVerifier::new("secret", Duration::from_secs(60));
        let event = WebhookEvent::new("evt-2", "caption.created", json!({"id": 9}));
        let prepared = verifier
            .prepare("https://example.test/hooks", &event, 100)
            .await
            .unwrap();
        assert_eq!(prepared.url, "https://example.test/hooks");
        assert_eq!(prepared.event_id, "evt-2");
        assert!(prepared.signature.starts_with("t=100,v1="));
    }
}
