//! Configurable CORS policy evaluation with secure credential defaults.

use hyper::Method;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OriginRule {
    Any,
    Exact(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsPolicy {
    pub origins: Vec<OriginRule>,
    pub methods: Vec<Method>,
    pub allowed_headers: Vec<String>,
    pub exposed_headers: Vec<String>,
    pub credentials: bool,
    pub max_age_seconds: Option<u64>,
}

impl Default for CorsPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsPolicy {
    pub fn new() -> Self {
        Self {
            origins: Vec::new(),
            methods: vec![Method::GET, Method::HEAD, Method::OPTIONS],
            allowed_headers: Vec::new(),
            exposed_headers: Vec::new(),
            credentials: false,
            max_age_seconds: Some(600),
        }
    }

    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.origins.push(OriginRule::Exact(origin.into()));
        self
    }
    pub fn allow_any_origin(mut self) -> Self {
        self.origins.push(OriginRule::Any);
        self
    }
    pub fn allow_methods(mut self, methods: impl IntoIterator<Item = Method>) -> Self {
        self.methods = methods.into_iter().collect();
        self
    }
    pub fn allow_headers(mut self, headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.allowed_headers = headers.into_iter().map(|h| h.into()).collect();
        self
    }
    pub fn expose_headers(mut self, headers: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.exposed_headers = headers.into_iter().map(|h| h.into()).collect();
        self
    }
    pub fn credentials(mut self, enabled: bool) -> Self {
        self.credentials = enabled;
        self
    }
    pub fn max_age(mut self, seconds: Option<u64>) -> Self {
        self.max_age_seconds = seconds;
        self
    }

    pub fn evaluate(
        &self,
        origin: Option<&str>,
        method: &Method,
        requested_headers: &[&str],
    ) -> CorsDecision {
        let Some(origin) = origin.filter(|origin| !origin.is_empty()) else {
            return CorsDecision::denied(CorsDenial::MissingOrigin);
        };
        let origin_allowed = self.origins.iter().any(|rule| match rule {
            OriginRule::Any => !self.credentials,
            OriginRule::Exact(value) => value == origin,
        });
        if !origin_allowed {
            return CorsDecision::denied(CorsDenial::OriginNotAllowed);
        }
        if !self.methods.iter().any(|allowed| allowed == method) {
            return CorsDecision::denied(CorsDenial::MethodNotAllowed);
        }
        let missing = requested_headers
            .iter()
            .filter(|requested| {
                !self
                    .allowed_headers
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(requested))
            })
            .map(|header| (*header).to_string())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return CorsDecision::denied(CorsDenial::HeaderNotAllowed(missing));
        }
        let mut headers = BTreeMap::from([
            (
                "Access-Control-Allow-Origin".to_string(),
                origin.to_string(),
            ),
            ("Vary".to_string(), "Origin".to_string()),
        ]);
        if self.credentials {
            headers.insert(
                "Access-Control-Allow-Credentials".to_string(),
                "true".to_string(),
            );
        }
        if !self.methods.is_empty() {
            headers.insert(
                "Access-Control-Allow-Methods".to_string(),
                self.methods
                    .iter()
                    .map(Method::as_str)
                    .collect::<Vec<_>>()
                    .join(", "),
            );
        }
        if !self.allowed_headers.is_empty() {
            headers.insert(
                "Access-Control-Allow-Headers".to_string(),
                self.allowed_headers.join(", "),
            );
        }
        if !self.exposed_headers.is_empty() {
            headers.insert(
                "Access-Control-Expose-Headers".to_string(),
                self.exposed_headers.join(", "),
            );
        }
        if let Some(max_age) = self.max_age_seconds {
            headers.insert("Access-Control-Max-Age".to_string(), max_age.to_string());
        }
        CorsDecision {
            allowed: true,
            headers,
            denial: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsDecision {
    pub allowed: bool,
    pub headers: BTreeMap<String, String>,
    pub denial: Option<CorsDenial>,
}

impl CorsDecision {
    fn denied(reason: CorsDenial) -> Self {
        Self {
            allowed: false,
            headers: BTreeMap::new(),
            denial: Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorsDenial {
    MissingOrigin,
    OriginNotAllowed,
    MethodNotAllowed,
    HeaderNotAllowed(Vec<String>),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_exact_origin_and_preflight_metadata() {
        let policy = CorsPolicy::new()
            .allow_origin("https://app.example")
            .allow_methods([Method::GET, Method::POST])
            .allow_headers(["Content-Type", "X-Request-ID"])
            .credentials(true);
        let decision = policy.evaluate(
            Some("https://app.example"),
            &Method::POST,
            &["content-type"],
        );
        assert!(decision.allowed);
        assert_eq!(
            decision.headers["Access-Control-Allow-Origin"],
            "https://app.example"
        );
        assert_eq!(decision.headers["Access-Control-Allow-Credentials"], "true");
    }

    #[test]
    fn rejects_wrong_origin_method_and_header() {
        let policy = CorsPolicy::new()
            .allow_origin("https://app.example")
            .allow_methods([Method::GET])
            .allow_headers(["Content-Type"]);
        assert_eq!(
            policy
                .evaluate(Some("https://evil.example"), &Method::GET, &[])
                .denial,
            Some(CorsDenial::OriginNotAllowed)
        );
        assert_eq!(
            policy
                .evaluate(Some("https://app.example"), &Method::POST, &[])
                .denial,
            Some(CorsDenial::MethodNotAllowed)
        );
        assert_eq!(
            policy
                .evaluate(Some("https://app.example"), &Method::GET, &["X-Admin"])
                .denial,
            Some(CorsDenial::HeaderNotAllowed(vec!["X-Admin".into()]))
        );
    }

    #[test]
    fn wildcard_origin_cannot_be_combined_with_credentials() {
        let policy = CorsPolicy::new().allow_any_origin().credentials(true);
        let decision = policy.evaluate(Some("https://app.example"), &Method::GET, &[]);
        assert!(!decision.allowed);
        assert_eq!(decision.denial, Some(CorsDenial::OriginNotAllowed));
    }
}
