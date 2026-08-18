//! Idempotency-key primitives for safely deduplicating retried operations.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl StoredResponse {
    pub fn new(status: u16, body: Vec<u8>) -> Self {
        Self {
            status,
            headers: BTreeMap::new(),
            body,
        }
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyLease {
    pub key: String,
    pub fingerprint: String,
    owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimOutcome {
    Acquired(IdempotencyLease),
    Replay(StoredResponse),
    InFlight,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyError {
    InvalidKey,
    InvalidFingerprint,
    NotOwner,
}

#[derive(Debug, Clone)]
enum RecordState {
    InFlight { owner: String },
    Completed { response: StoredResponse },
}

#[derive(Debug, Clone)]
struct Record {
    fingerprint: String,
    state: RecordState,
    expires_at: u64,
}

#[derive(Clone, Default)]
pub struct MemoryIdempotencyStore {
    records: Arc<Mutex<HashMap<String, Record>>>,
}

impl MemoryIdempotencyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn claim(
        &self,
        key: impl Into<String>,
        fingerprint: impl Into<String>,
        now: u64,
        ttl_seconds: u64,
    ) -> Result<ClaimOutcome, IdempotencyError> {
        let key = key.into();
        let fingerprint = fingerprint.into();
        validate_key(&key)?;
        validate_fingerprint(&fingerprint)?;
        let mut records = self.records.lock().await;
        purge_expired(&mut records, now);
        if let Some(record) = records.get(&key) {
            if record.fingerprint != fingerprint {
                return Ok(ClaimOutcome::Conflict);
            }
            return Ok(match &record.state {
                RecordState::InFlight { .. } => ClaimOutcome::InFlight,
                RecordState::Completed { response } => ClaimOutcome::Replay(response.clone()),
            });
        }
        let owner = Uuid::new_v4().to_string();
        records.insert(
            key.clone(),
            Record {
                fingerprint: fingerprint.clone(),
                state: RecordState::InFlight {
                    owner: owner.clone(),
                },
                expires_at: now.saturating_add(ttl_seconds.max(1)),
            },
        );
        Ok(ClaimOutcome::Acquired(IdempotencyLease {
            key,
            fingerprint,
            owner,
        }))
    }

    pub async fn finalize(
        &self,
        lease: &IdempotencyLease,
        response: StoredResponse,
        now: u64,
        ttl_seconds: u64,
    ) -> Result<(), IdempotencyError> {
        let mut records = self.records.lock().await;
        let Some(record) = records.get_mut(&lease.key) else {
            return Err(IdempotencyError::NotOwner);
        };
        if record.fingerprint != lease.fingerprint
            || !matches!(&record.state, RecordState::InFlight { owner } if owner == &lease.owner)
        {
            return Err(IdempotencyError::NotOwner);
        }
        record.state = RecordState::Completed { response };
        record.expires_at = now.saturating_add(ttl_seconds.max(1));
        Ok(())
    }

    pub async fn abandon(&self, lease: &IdempotencyLease) -> Result<(), IdempotencyError> {
        let mut records = self.records.lock().await;
        let Some(record) = records.get(&lease.key) else {
            return Err(IdempotencyError::NotOwner);
        };
        if record.fingerprint != lease.fingerprint
            || !matches!(&record.state, RecordState::InFlight { owner } if owner == &lease.owner)
        {
            return Err(IdempotencyError::NotOwner);
        }
        records.remove(&lease.key);
        Ok(())
    }

    pub async fn cleanup(&self, now: u64) -> usize {
        let mut records = self.records.lock().await;
        let before = records.len();
        purge_expired(&mut records, now);
        before - records.len()
    }

    pub async fn len(&self) -> usize {
        self.records.lock().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.records.lock().await.is_empty()
    }
}

pub fn fingerprint(method: &str, path: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    format!("sha256:{:x}", hasher.finalize())
}

fn validate_key(key: &str) -> Result<(), IdempotencyError> {
    if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
        Err(IdempotencyError::InvalidKey)
    } else {
        Ok(())
    }
}

fn validate_fingerprint(value: &str) -> Result<(), IdempotencyError> {
    if value.is_empty() || value.len() > 256 {
        Err(IdempotencyError::InvalidFingerprint)
    } else {
        Ok(())
    }
}

fn purge_expired(records: &mut HashMap<String, Record>, now: u64) {
    records.retain(|_, record| record.expires_at > now);
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn replays_completed_response_and_rejects_fingerprint_conflicts() {
        let store = MemoryIdempotencyStore::new();
        let fp = fingerprint("POST", "/payments", br#"{"amount":10}"#);
        let lease = match store.claim("payment-1", &fp, 100, 60).await.unwrap() {
            ClaimOutcome::Acquired(lease) => lease,
            other => panic!("unexpected outcome: {other:?}"),
        };
        store
            .finalize(
                &lease,
                StoredResponse::new(201, b"created".to_vec()),
                101,
                60,
            )
            .await
            .unwrap();
        assert_eq!(
            store.claim("payment-1", &fp, 102, 60).await.unwrap(),
            ClaimOutcome::Replay(StoredResponse::new(201, b"created".to_vec()))
        );
        assert_eq!(
            store
                .claim("payment-1", "sha256:different", 102, 60)
                .await
                .unwrap(),
            ClaimOutcome::Conflict
        );
    }

    #[tokio::test]
    async fn concurrent_claims_have_one_owner_and_abandon_allows_retry() {
        let store = MemoryIdempotencyStore::new();
        let first = store.claim("job-1", "sha256:one", 100, 60).await.unwrap();
        assert!(matches!(first, ClaimOutcome::Acquired(_)));
        assert_eq!(
            store.claim("job-1", "sha256:one", 100, 60).await.unwrap(),
            ClaimOutcome::InFlight
        );
        let lease = match first {
            ClaimOutcome::Acquired(lease) => lease,
            _ => unreachable!(),
        };
        store.abandon(&lease).await.unwrap();
        assert!(matches!(
            store.claim("job-1", "sha256:one", 100, 60).await.unwrap(),
            ClaimOutcome::Acquired(_)
        ));
    }

    #[tokio::test]
    async fn expired_records_are_removed_and_invalid_keys_are_rejected() {
        let store = MemoryIdempotencyStore::new();
        let _ = store.claim("expired", "sha256:x", 100, 1).await.unwrap();
        assert_eq!(store.cleanup(101).await, 1);
        assert_eq!(store.len().await, 0);
        assert_eq!(
            store.claim("", "sha256:x", 100, 1).await,
            Err(IdempotencyError::InvalidKey)
        );
    }
}
