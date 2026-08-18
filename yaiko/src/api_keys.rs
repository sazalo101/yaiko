//! Scoped API keys and signed-request authentication primitives.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedApiKey {
    pub id: String,
    pub secret: String,
}

#[derive(Debug, Clone)]
struct ApiKeyRecord {
    secret_digest: Vec<u8>,
    scopes: BTreeSet<String>,
    active: bool,
    expires_at: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiKeyError {
    Invalid,
    NotFound,
    Inactive,
    Expired,
    MissingScope,
    InvalidSignature,
    StaleRequest,
}

#[derive(Clone, Default)]
pub struct ApiKeyStore {
    keys: Arc<Mutex<std::collections::BTreeMap<String, ApiKeyRecord>>>,
}

impl ApiKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue(&self, scopes: Vec<String>, ttl: Option<Duration>) -> IssuedApiKey {
        let id = Uuid::new_v4().to_string();
        let secret = Uuid::new_v4().to_string() + &Uuid::new_v4().to_string();
        let expires_at = ttl.map(|duration| now().saturating_add(duration.as_secs()));
        let record = ApiKeyRecord {
            secret_digest: digest_secret(&secret),
            scopes: scopes.into_iter().collect(),
            active: true,
            expires_at,
        };
        self.keys
            .lock()
            .expect("api-key store poisoned")
            .insert(id.clone(), record);
        IssuedApiKey { id, secret }
    }

    pub fn revoke(&self, id: &str) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.lock().expect("api-key store poisoned");
        let record = keys.get_mut(id).ok_or(ApiKeyError::NotFound)?;
        record.active = false;
        Ok(())
    }

    pub fn rotate(&self, id: &str, ttl: Option<Duration>) -> Result<IssuedApiKey, ApiKeyError> {
        self.revoke(id)?;
        Ok(self.issue(Vec::new(), ttl))
    }

    pub fn authenticate(
        &self,
        id: &str,
        secret: &str,
        scope: Option<&str>,
    ) -> Result<(), ApiKeyError> {
        let keys = self.keys.lock().expect("api-key store poisoned");
        let record = keys.get(id).ok_or(ApiKeyError::NotFound)?;
        if !record.active {
            return Err(ApiKeyError::Inactive);
        }
        if record
            .expires_at
            .map(|expires| now() >= expires)
            .unwrap_or(false)
        {
            return Err(ApiKeyError::Expired);
        }
        verify_digest(&record.secret_digest, secret)
            .then_some(())
            .ok_or(ApiKeyError::Invalid)?;
        if scope
            .map(|scope| !record.scopes.contains(scope))
            .unwrap_or(false)
        {
            return Err(ApiKeyError::MissingScope);
        }
        Ok(())
    }
}

pub fn sign_request(secret: &str, timestamp: u64, method: &str, path: &str, body: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts arbitrary key sizes");
    mac.update(format!("{}.{}.{}.", timestamp, method.to_ascii_uppercase(), path).as_bytes());
    mac.update(&Sha256::digest(body));
    URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
}

pub fn verify_request_signature(
    secret: &str,
    timestamp: u64,
    signature: &str,
    method: &str,
    path: &str,
    body: &[u8],
    tolerance: Duration,
) -> Result<(), ApiKeyError> {
    if now().abs_diff(timestamp) > tolerance.as_secs() {
        return Err(ApiKeyError::StaleRequest);
    }
    let expected = sign_request(secret, timestamp, method, path, body);
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts arbitrary key sizes");
    mac.update(format!("{}.{}.{}.", timestamp, method.to_ascii_uppercase(), path).as_bytes());
    mac.update(&Sha256::digest(body));
    mac.verify_slice(
        &URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| ApiKeyError::InvalidSignature)?,
    )
    .map_err(|_| {
        let _ = expected;
        ApiKeyError::InvalidSignature
    })
}

fn digest_secret(secret: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(b"yaiko-api-key-digest")
        .expect("HMAC accepts arbitrary key sizes");
    mac.update(secret.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn verify_digest(expected: &[u8], secret: &str) -> bool {
    let mut mac = HmacSha256::new_from_slice(b"yaiko-api-key-digest")
        .expect("HMAC accepts arbitrary key sizes");
    mac.update(secret.as_bytes());
    mac.verify_slice(expected).is_ok()
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
    fn issues_scoped_keys_and_rejects_wrong_scope_or_secret() {
        let store = ApiKeyStore::new();
        let key = store.issue(vec!["video:read".to_string()], None);
        assert!(store
            .authenticate(&key.id, &key.secret, Some("video:read"))
            .is_ok());
        assert_eq!(
            store.authenticate(&key.id, &key.secret, Some("video:write")),
            Err(ApiKeyError::MissingScope)
        );
        assert_eq!(
            store.authenticate(&key.id, "wrong", None),
            Err(ApiKeyError::Invalid)
        );
    }

    #[test]
    fn revocation_and_rotation_invalidate_old_credentials() {
        let store = ApiKeyStore::new();
        let key = store.issue(Vec::new(), None);
        store.revoke(&key.id).unwrap();
        assert_eq!(
            store.authenticate(&key.id, &key.secret, None),
            Err(ApiKeyError::Inactive)
        );
        let rotated = store.rotate(&key.id, None).unwrap();
        assert_ne!(key.secret, rotated.secret);
        assert!(store
            .authenticate(&rotated.id, &rotated.secret, None)
            .is_ok());
    }

    #[test]
    fn signs_and_verifies_requests_with_timestamp_tolerance() {
        let timestamp = now();
        let signature = sign_request("secret", timestamp, "post", "/videos", b"payload");
        assert!(verify_request_signature(
            "secret",
            timestamp,
            &signature,
            "POST",
            "/videos",
            b"payload",
            Duration::from_secs(30)
        )
        .is_ok());
        assert_eq!(
            verify_request_signature(
                "secret",
                timestamp,
                "00",
                "POST",
                "/videos",
                b"payload",
                Duration::from_secs(30)
            ),
            Err(ApiKeyError::InvalidSignature)
        );
        assert_eq!(
            verify_request_signature(
                "secret",
                timestamp.saturating_sub(100),
                &signature,
                "POST",
                "/videos",
                b"payload",
                Duration::from_secs(30)
            ),
            Err(ApiKeyError::StaleRequest)
        );
    }
}
