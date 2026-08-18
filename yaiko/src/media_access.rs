//! Scoped, expiring media delivery access tokens.

use crate::media_processing::MediaPath;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaAccessError {
    UnsafePath,
    InvalidTtl,
    InvalidScope,
    InvalidToken,
    Expired,
    ScopeMismatch,
    PathMismatch,
    RangeForbidden,
    Replay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAccessClaims {
    pub scope: String,
    pub path: MediaPath,
    pub expires_at: u64,
    pub single_use: bool,
    pub allow_range: bool,
}

#[derive(Clone)]
pub struct MediaAccessTokens {
    secret: Arc<Vec<u8>>,
    used: Arc<Mutex<HashSet<String>>>,
}

impl MediaAccessTokens {
    pub fn new(secret: impl AsRef<[u8]>) -> Result<Self, MediaAccessError> {
        if secret.as_ref().len() < 16 {
            return Err(MediaAccessError::InvalidToken);
        }
        Ok(Self {
            secret: Arc::new(secret.as_ref().to_vec()),
            used: Arc::new(Mutex::new(HashSet::new())),
        })
    }
    pub fn issue(
        &self,
        scope: impl Into<String>,
        path: impl Into<String>,
        now: SystemTime,
        ttl_secs: u64,
        single_use: bool,
        allow_range: bool,
    ) -> Result<String, MediaAccessError> {
        let scope = validate_scope(scope.into())?;
        let path = MediaPath::new(path.into()).map_err(|_| MediaAccessError::UnsafePath)?;
        if ttl_secs == 0 || ttl_secs > 86_400 {
            return Err(MediaAccessError::InvalidTtl);
        }
        let exp = now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MediaAccessError::InvalidTtl)?
            .as_secs()
            .checked_add(ttl_secs)
            .ok_or(MediaAccessError::InvalidTtl)?;
        let nonce = format!("{}", self.used.lock().unwrap().len());
        let payload = format!(
            "v1|{}|{}|{}|{}|{}|{}",
            scope,
            path.display(),
            exp,
            u8::from(single_use),
            u8::from(allow_range),
            nonce
        );
        Ok(self.sign(payload))
    }
    pub fn verify(
        &self,
        token: &str,
        expected_scope: &str,
        expected_path: &str,
        now: SystemTime,
        range_requested: bool,
    ) -> Result<MediaAccessClaims, MediaAccessError> {
        let (payload, signature) = token
            .rsplit_once('.')
            .ok_or(MediaAccessError::InvalidToken)?;
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).map_err(|_| MediaAccessError::InvalidToken)?;
        mac.update(payload.as_bytes());
        let provided = URL_SAFE_NO_PAD
            .decode(signature)
            .map_err(|_| MediaAccessError::InvalidToken)?;
        mac.verify_slice(&provided)
            .map_err(|_| MediaAccessError::InvalidToken)?;
        let parts: Vec<&str> = payload.split('|').collect();
        if parts.len() != 7 || parts[0] != "v1" {
            return Err(MediaAccessError::InvalidToken);
        }
        let scope = validate_scope(parts[1].to_string())?;
        let path =
            MediaPath::new(parts[2].to_string()).map_err(|_| MediaAccessError::UnsafePath)?;
        let expires_at = parts[3]
            .parse::<u64>()
            .map_err(|_| MediaAccessError::InvalidToken)?;
        let single_use = parts[4] == "1";
        let allow_range = parts[5] == "1";
        if now
            .duration_since(UNIX_EPOCH)
            .map_err(|_| MediaAccessError::Expired)?
            .as_secs()
            >= expires_at
        {
            return Err(MediaAccessError::Expired);
        }
        if scope != expected_scope {
            return Err(MediaAccessError::ScopeMismatch);
        }
        if path.display() != expected_path {
            return Err(MediaAccessError::PathMismatch);
        }
        if range_requested && !allow_range {
            return Err(MediaAccessError::RangeForbidden);
        }
        if single_use && !self.used.lock().unwrap().insert(parts[6].to_string()) {
            return Err(MediaAccessError::Replay);
        }
        Ok(MediaAccessClaims {
            scope,
            path,
            expires_at,
            single_use,
            allow_range,
        })
    }
    fn sign(&self, payload: String) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret).expect("validated secret length");
        mac.update(payload.as_bytes());
        format!(
            "{}.{}",
            payload,
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        )
    }
}

fn validate_scope(scope: String) -> Result<String, MediaAccessError> {
    if scope.is_empty()
        || scope.len() > 128
        || scope
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | '|'))
    {
        return Err(MediaAccessError::InvalidScope);
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(10_000)
    }
    #[test]
    fn issues_and_verifies_scoped_tokens() {
        let tokens = MediaAccessTokens::new([7; 32]).unwrap();
        let token = tokens
            .issue("tenant-a", "renders/video.mp4", now(), 60, false, true)
            .unwrap();
        let claims = tokens
            .verify(&token, "tenant-a", "renders/video.mp4", now(), true)
            .unwrap();
        assert_eq!(claims.scope, "tenant-a");
        assert!(claims.allow_range);
    }
    #[test]
    fn rejects_tampering_scope_path_expiry_and_ranges() {
        let tokens = MediaAccessTokens::new([7; 32]).unwrap();
        let token = tokens
            .issue("tenant-a", "video.mp4", now(), 60, false, false)
            .unwrap();
        assert_eq!(
            tokens.verify(&token, "tenant-b", "video.mp4", now(), false),
            Err(MediaAccessError::ScopeMismatch)
        );
        assert_eq!(
            tokens.verify(&token, "tenant-a", "other.mp4", now(), false),
            Err(MediaAccessError::PathMismatch)
        );
        assert_eq!(
            tokens.verify(&token, "tenant-a", "video.mp4", now(), true),
            Err(MediaAccessError::RangeForbidden)
        );
        let mut tampered = token.clone();
        tampered.push('x');
        assert_eq!(
            tokens.verify(&tampered, "tenant-a", "video.mp4", now(), false),
            Err(MediaAccessError::InvalidToken)
        );
    }
    #[test]
    fn enforces_expiry_replay_and_input_bounds() {
        let tokens = MediaAccessTokens::new([7; 32]).unwrap();
        assert_eq!(
            tokens.issue("tenant-a", "video.mp4", now(), 0, false, false),
            Err(MediaAccessError::InvalidTtl)
        );
        let token = tokens
            .issue("tenant-a", "video.mp4", now(), 1, true, false)
            .unwrap();
        assert!(tokens
            .verify(&token, "tenant-a", "video.mp4", now(), false)
            .is_ok());
        assert_eq!(
            tokens.verify(&token, "tenant-a", "video.mp4", now(), false),
            Err(MediaAccessError::Replay)
        );
        assert_eq!(
            tokens.verify(
                &tokens
                    .issue("tenant-a", "video.mp4", now(), 1, false, false)
                    .unwrap(),
                "tenant-a",
                "video.mp4",
                UNIX_EPOCH + std::time::Duration::from_secs(10002),
                false
            ),
            Err(MediaAccessError::Expired)
        );
    }
}
