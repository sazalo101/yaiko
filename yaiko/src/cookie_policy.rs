//! Secure cookie policy and signed-cookie helpers.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use cookie::SameSite;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CookiePolicy {
    pub name: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSite,
    pub max_age: Option<Duration>,
}

impl CookiePolicy {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: "/".into(),
            secure: true,
            http_only: true,
            same_site: SameSite::Lax,
            max_age: None,
        }
    }
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }
    pub fn max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }
    pub fn serialize(&self, value: &str) -> Result<String, CookieError> {
        let mut builder = cookie::Cookie::build(self.name.clone(), value.to_string())
            .path(self.path.clone())
            .secure(self.secure)
            .http_only(self.http_only)
            .same_site(self.same_site);
        if let Some(age) = self.max_age {
            builder = builder.max_age(cookie::time::Duration::seconds(age.as_secs() as i64));
        }
        let cookie = builder.finish();
        Ok(cookie.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CookieError {
    InvalidSignature,
    InvalidValue,
    InvalidSecret,
    Expired,
}

#[derive(Clone)]
pub struct SignedCookieCodec {
    secret: Vec<u8>,
    ttl: Duration,
}

impl SignedCookieCodec {
    pub fn new(secret: impl Into<Vec<u8>>, ttl: Duration) -> Self {
        Self {
            secret: secret.into(),
            ttl,
        }
    }
    pub fn sign(&self, value: &str) -> Result<String, CookieError> {
        if value.contains(';') || value.contains('\n') || value.contains('\r') {
            return Err(CookieError::InvalidValue);
        }
        let expires = now().saturating_add(self.ttl.as_secs());
        let payload = format!("{}.{}", expires, value);
        let signature = sign_bytes(&self.secret, payload.as_bytes())?;
        Ok(format!("{}.{}", URL_SAFE_NO_PAD.encode(value), signature))
    }
    pub fn verify(&self, signed: &str) -> Result<String, CookieError> {
        let (encoded, signature) = signed
            .split_once('.')
            .ok_or(CookieError::InvalidSignature)?;
        let value = String::from_utf8(
            URL_SAFE_NO_PAD
                .decode(encoded)
                .map_err(|_| CookieError::InvalidSignature)?,
        )
        .map_err(|_| CookieError::InvalidValue)?;
        let expires = now().saturating_add(self.ttl.as_secs());
        let payload = format!("{}.{}", expires, value);
        let expected = sign_bytes(&self.secret, payload.as_bytes())?;
        if expected != signature {
            return Err(CookieError::InvalidSignature);
        }
        Ok(value)
    }
}

fn sign_bytes(secret: &[u8], data: &[u8]) -> Result<String, CookieError> {
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| CookieError::InvalidSecret)?;
    mac.update(data);
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
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
    fn serializes_secure_cookie_defaults() {
        let value = CookiePolicy::new("session")
            .max_age(Duration::from_secs(60))
            .serialize("abc")
            .unwrap();
        assert!(value.contains("HttpOnly"));
        assert!(value.contains("Secure"));
        assert!(value.contains("SameSite=Lax"));
        assert!(value.contains("Max-Age=60"));
    }

    #[test]
    fn signs_and_rejects_tampered_cookie_values() {
        let codec = SignedCookieCodec::new("secret", Duration::from_secs(60));
        let signed = codec.sign("session-id").unwrap();
        assert_eq!(codec.verify(&signed).unwrap(), "session-id");
        let tampered = format!("{}x", signed);
        assert_eq!(codec.verify(&tampered), Err(CookieError::InvalidSignature));
    }

    #[test]
    fn rejects_cookie_injection_values() {
        let codec = SignedCookieCodec::new("secret", Duration::from_secs(60));
        assert_eq!(codec.sign("bad\nvalue"), Err(CookieError::InvalidValue));
    }
}
