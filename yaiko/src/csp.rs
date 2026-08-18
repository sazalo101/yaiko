//! Typed Content Security Policy and security-header helpers.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CspNonce(String);

impl CspNonce {
    pub fn generate() -> Self {
        Self(URL_SAFE_NO_PAD.encode(Uuid::new_v4().as_bytes()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn source(&self) -> String {
        format!("'nonce-{}'", self.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ContentSecurityPolicy {
    directives: BTreeMap<String, Vec<String>>,
}

impl ContentSecurityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn directive(
        mut self,
        name: impl Into<String>,
        sources: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.directives
            .insert(name.into(), sources.into_iter().map(Into::into).collect());
        self
    }

    pub fn default_self(self) -> Self {
        self.directive("default-src", ["'self'"])
    }
    pub fn script_self(self) -> Self {
        self.directive("script-src", ["'self'"])
    }
    pub fn style_self(self) -> Self {
        self.directive("style-src", ["'self'"])
    }
    pub fn script_nonce(self, nonce: &CspNonce) -> Self {
        self.directive("script-src", vec!["'self'".to_string(), nonce.source()])
    }
    pub fn report_only_header(&self) -> String {
        self.serialize()
    }
    pub fn header(&self) -> String {
        self.serialize()
    }

    fn serialize(&self) -> String {
        self.directives
            .iter()
            .map(|(name, sources)| {
                if sources.is_empty() {
                    name.clone()
                } else {
                    format!("{} {}", name, sources.join(" "))
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityPolicyHeaders {
    pub headers: BTreeMap<String, String>,
}

impl SecurityPolicyHeaders {
    pub fn secure(csp: ContentSecurityPolicy) -> Self {
        let mut headers = BTreeMap::from([
            ("X-Content-Type-Options".to_string(), "nosniff".to_string()),
            ("X-Frame-Options".to_string(), "DENY".to_string()),
            (
                "Referrer-Policy".to_string(),
                "strict-origin-when-cross-origin".to_string(),
            ),
            ("Content-Security-Policy".to_string(), csp.header()),
        ]);
        headers.insert(
            "Permissions-Policy".to_string(),
            "geolocation=(), microphone=(), camera=()".to_string(),
        );
        Self { headers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_typed_csp_directives() {
        let policy = ContentSecurityPolicy::new()
            .default_self()
            .script_self()
            .style_self();
        assert_eq!(
            policy.header(),
            "default-src 'self'; script-src 'self'; style-src 'self'"
        );
        assert!(!policy.header().contains("unsafe-eval"));
    }

    #[test]
    fn generated_nonces_are_url_safe_and_unique() {
        let first = CspNonce::generate();
        let second = CspNonce::generate();
        assert_ne!(first, second);
        assert!(first
            .as_str()
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')));
        assert!(ContentSecurityPolicy::new()
            .script_nonce(&first)
            .header()
            .contains(&first.source()));
    }

    #[test]
    fn secure_headers_include_csp_and_baseline_protections() {
        let headers =
            SecurityPolicyHeaders::secure(ContentSecurityPolicy::new().default_self()).headers;
        assert_eq!(headers["X-Content-Type-Options"], "nosniff");
        assert_eq!(headers["X-Frame-Options"], "DENY");
        assert_eq!(headers["Content-Security-Policy"], "default-src 'self'");
    }
}
