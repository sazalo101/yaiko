//! Signed webhook notifications for media-processing lifecycle events.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaEventKind {
    Queued,
    Progress,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaWebhookError {
    InvalidSecret,
    InvalidEvent,
    PayloadTooLarge,
    InvalidTimestamp,
    InvalidSignature,
    Replay,
    Filtered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaWebhookEvent {
    pub id: String,
    pub task_id: String,
    pub kind: MediaEventKind,
    pub progress: Option<u8>,
    pub attempt: u32,
    pub timestamp: u64,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MediaWebhookSigner {
    secret: Arc<Vec<u8>>,
    seen: Arc<Mutex<HashSet<String>>>,
    max_payload_bytes: usize,
}

impl MediaWebhookSigner {
    pub fn new(
        secret: impl AsRef<[u8]>,
        max_payload_bytes: usize,
    ) -> Result<Self, MediaWebhookError> {
        if secret.as_ref().len() < 16 || max_payload_bytes == 0 || max_payload_bytes > 1_048_576 {
            return Err(MediaWebhookError::InvalidSecret);
        }
        Ok(Self {
            secret: Arc::new(secret.as_ref().to_vec()),
            seen: Arc::new(Mutex::new(HashSet::new())),
            max_payload_bytes,
        })
    }
    pub fn encode(&self, event: &MediaWebhookEvent) -> Result<String, MediaWebhookError> {
        validate_event(event)?;
        let payload = format!(
            "{}|{}|{}|{}|{}|{}|{}",
            event.id,
            event.task_id,
            kind_code(event.kind),
            event
                .progress
                .map_or_else(|| "-".into(), |value| value.to_string()),
            event.attempt,
            event.timestamp,
            event.detail.clone().unwrap_or_default()
        );
        if payload.len() > self.max_payload_bytes {
            return Err(MediaWebhookError::PayloadTooLarge);
        }
        Ok(self.sign(&payload))
    }
    pub fn verify(
        &self,
        token: &str,
        now: SystemTime,
        max_age_secs: u64,
    ) -> Result<MediaWebhookEvent, MediaWebhookError> {
        let (payload, signature) = token
            .rsplit_once('.')
            .ok_or(MediaWebhookError::InvalidSignature)?;
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|_| MediaWebhookError::InvalidSecret)?;
        mac.update(payload.as_bytes());
        let provided = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| MediaWebhookError::InvalidSignature)?;
        mac.verify_slice(&provided)
            .map_err(|_| MediaWebhookError::InvalidSignature)?;
        if payload.len() > self.max_payload_bytes {
            return Err(MediaWebhookError::PayloadTooLarge);
        }
        let parts: Vec<&str> = payload.split('|').collect();
        if parts.len() != 7 {
            return Err(MediaWebhookError::InvalidEvent);
        }
        let kind = parse_kind(parts[2])?;
        let progress = if parts[3] == "-" {
            None
        } else {
            Some(
                parts[3]
                    .parse::<u8>()
                    .map_err(|_| MediaWebhookError::InvalidEvent)?,
            )
        };
        let timestamp = parts[5]
            .parse::<u64>()
            .map_err(|_| MediaWebhookError::InvalidTimestamp)?;
        let current = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MediaWebhookError::InvalidTimestamp)?
            .as_secs();
        if timestamp > current || current - timestamp > max_age_secs {
            return Err(MediaWebhookError::InvalidTimestamp);
        }
        let event = MediaWebhookEvent {
            id: parts[0].into(),
            task_id: parts[1].into(),
            kind,
            progress,
            attempt: parts[4]
                .parse()
                .map_err(|_| MediaWebhookError::InvalidEvent)?,
            timestamp,
            detail: (!parts[6].is_empty()).then(|| parts[6].into()),
        };
        validate_event(&event)?;
        if !self.seen.lock().unwrap().insert(event.id.clone()) {
            return Err(MediaWebhookError::Replay);
        }
        Ok(event)
    }
    fn sign(&self, payload: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("validated secret");
        mac.update(payload.as_bytes());
        format!(
            "{}.{}",
            payload,
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        )
    }
}

fn kind_code(kind: MediaEventKind) -> &'static str {
    match kind {
        MediaEventKind::Queued => "queued",
        MediaEventKind::Progress => "progress",
        MediaEventKind::Succeeded => "succeeded",
        MediaEventKind::Failed => "failed",
        MediaEventKind::Cancelled => "cancelled",
    }
}
fn parse_kind(kind: &str) -> Result<MediaEventKind, MediaWebhookError> {
    match kind {
        "queued" => Ok(MediaEventKind::Queued),
        "progress" => Ok(MediaEventKind::Progress),
        "succeeded" => Ok(MediaEventKind::Succeeded),
        "failed" => Ok(MediaEventKind::Failed),
        "cancelled" => Ok(MediaEventKind::Cancelled),
        _ => Err(MediaWebhookError::InvalidEvent),
    }
}
fn validate_event(event: &MediaWebhookEvent) -> Result<(), MediaWebhookError> {
    if event.id.is_empty()
        || event.id.len() > 128
        || event.id.contains('|')
        || event.task_id.is_empty()
        || event.task_id.len() > 128
        || event.task_id.contains('|')
        || event.attempt == 0
        || event.progress.is_some_and(|value| value > 100)
        || event
            .detail
            .as_ref()
            .is_some_and(|value| value.len() > 1024 || value.contains('|'))
    {
        return Err(MediaWebhookError::InvalidEvent);
    }
    if matches!(event.kind, MediaEventKind::Progress) && event.progress.is_none() {
        return Err(MediaWebhookError::InvalidEvent);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(10_000)
    }
    fn event(kind: MediaEventKind) -> MediaWebhookEvent {
        MediaWebhookEvent {
            id: "event-1".into(),
            task_id: "task-1".into(),
            kind,
            progress: Some(50),
            attempt: 1,
            timestamp: 10_000,
            detail: Some("encoding".into()),
        }
    }
    #[test]
    fn signs_and_verifies_lifecycle_events() {
        let signer = MediaWebhookSigner::new([7; 32], 1024).unwrap();
        let token = signer.encode(&event(MediaEventKind::Progress)).unwrap();
        let verified = signer.verify(&token, now(), 60).unwrap();
        assert_eq!(verified, event(MediaEventKind::Progress));
    }
    #[test]
    fn rejects_tampering_expiry_replay_and_invalid_events() {
        let signer = MediaWebhookSigner::new([7; 32], 1024).unwrap();
        let token = signer.encode(&event(MediaEventKind::Succeeded)).unwrap();
        let mut tampered = token.clone();
        tampered.push('x');
        assert_eq!(
            signer.verify(&tampered, now(), 60),
            Err(MediaWebhookError::InvalidSignature)
        );
        assert_eq!(
            signer.verify(&token, now() + std::time::Duration::from_secs(61), 60),
            Err(MediaWebhookError::InvalidTimestamp)
        );
        assert!(signer.verify(&token, now(), 60).is_ok());
        assert_eq!(
            signer.verify(&token, now(), 60),
            Err(MediaWebhookError::Replay)
        );
        let mut invalid = event(MediaEventKind::Progress);
        invalid.progress = None;
        assert_eq!(
            signer.encode(&invalid),
            Err(MediaWebhookError::InvalidEvent)
        );
    }
    #[test]
    fn enforces_payload_and_secret_bounds() {
        assert!(matches!(
            MediaWebhookSigner::new([1; 8], 100),
            Err(MediaWebhookError::InvalidSecret)
        ));
        assert!(matches!(
            MediaWebhookSigner::new([1; 32], 0),
            Err(MediaWebhookError::InvalidSecret)
        ));
        let signer = MediaWebhookSigner::new([1; 32], 32).unwrap();
        let mut large = event(MediaEventKind::Failed);
        large.detail = Some("x".repeat(100));
        assert_eq!(
            signer.encode(&large),
            Err(MediaWebhookError::PayloadTooLarge)
        );
    }
}
