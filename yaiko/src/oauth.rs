//! Provider-neutral OAuth 2.0 and OIDC authorization primitives.
//!
//! This module intentionally handles authorization URL construction and response models;
//! applications remain responsible for making token/userinfo HTTP requests and validating
//! provider-specific ID-token claims.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
    pub method: String,
}

impl PkceChallenge {
    pub fn generate() -> Self {
        let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
            method: "S256".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthState {
    pub value: String,
    pub created_at: u64,
}

impl OAuthState {
    pub fn generate() -> Self {
        Self {
            value: format!("{}", Uuid::new_v4().simple()),
            created_at: now_unix(),
        }
    }

    pub fn is_fresh(&self, now: u64, max_age_seconds: u64) -> bool {
        now >= self.created_at && now - self.created_at <= max_age_seconds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthProvider {
    pub authorization_endpoint: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

impl OAuthProvider {
    pub fn authorization_url(&self, state: &OAuthState, pkce: &PkceChallenge) -> String {
        let mut params = vec![
            ("response_type", "code".to_string()),
            ("client_id", self.client_id.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
            ("scope", self.scopes.join(" ")),
            ("state", state.value.clone()),
            ("code_challenge", pkce.challenge.clone()),
            ("code_challenge_method", pkce.method.clone()),
        ];
        let separator = if self.authorization_endpoint.contains('?') {
            '&'
        } else {
            '?'
        };
        let query = params
            .drain(..)
            .map(|(key, value)| format!("{}={}", key, percent_encode(value)))
            .collect::<Vec<_>>()
            .join("&");
        format!("{}{}{}", self.authorization_endpoint, separator, query)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthCallback {
    pub code: String,
    pub state: String,
}

impl OAuthCallback {
    pub fn validate_state(&self, expected: &OAuthState) -> bool {
        self.state == expected.value
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OidcUserInfo {
    pub sub: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
}

fn percent_encode(value: impl AsRef<str>) -> String {
    percent_encoding::utf8_percent_encode(value.as_ref(), percent_encoding::NON_ALPHANUMERIC)
        .to_string()
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

    #[test]
    fn pkce_uses_s256_and_matches_rfc7636_derivation() {
        let pkce = PkceChallenge::generate();
        assert!(pkce.verifier.len() >= 43);
        assert_eq!(pkce.method, "S256");
        assert_eq!(
            pkce.challenge,
            URL_SAFE_NO_PAD.encode(Sha256::digest(pkce.verifier.as_bytes()))
        );
    }

    #[test]
    fn authorization_url_contains_encoded_security_parameters() {
        let provider = OAuthProvider {
            authorization_endpoint: "https://id.example/authorize".into(),
            client_id: "client id".into(),
            redirect_uri: "https://app.example/callback".into(),
            scopes: vec!["openid".into(), "profile".into()],
        };
        let state = OAuthState {
            value: "state-123".into(),
            created_at: now_unix(),
        };
        let pkce = PkceChallenge::generate();
        let url = provider.authorization_url(&state, &pkce);
        assert!(url.starts_with("https://id.example/authorize?"));
        assert!(url.contains("client_id=client%20id"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("state=state%2D123"));
    }

    #[test]
    fn callback_requires_exact_state_match_and_state_has_expiry() {
        let expected = OAuthState {
            value: "state".into(),
            created_at: 100,
        };
        let callback = OAuthCallback {
            code: "code".into(),
            state: "state".into(),
        };
        assert!(callback.validate_state(&expected));
        assert!(!OAuthCallback {
            state: "other".into(),
            ..callback.clone()
        }
        .validate_state(&expected));
        assert!(expected.is_fresh(120, 30));
        assert!(!expected.is_fresh(131, 30));
    }
}
