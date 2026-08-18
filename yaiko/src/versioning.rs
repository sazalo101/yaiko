//! API version negotiation and compatibility metadata.

use crate::{Request, Response};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiVersion {
    pub major: u16,
    pub minor: u16,
}

impl ApiVersion {
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    pub fn parse(value: &str) -> Result<Self, VersionError> {
        let value = value.trim().strip_prefix('v').unwrap_or(value.trim());
        let mut parts = value.split('.');
        let major = parts
            .next()
            .and_then(|value| value.parse().ok())
            .ok_or(VersionError::Invalid)?;
        let minor = parts
            .next()
            .map(|value| value.parse().ok())
            .unwrap_or(Some(0))
            .ok_or(VersionError::Invalid)?;
        if parts.next().is_some() {
            return Err(VersionError::Invalid);
        }
        Ok(Self::new(major, minor))
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "v{}.{}", self.major, self.minor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionDecision {
    pub selected: ApiVersion,
    pub deprecated: bool,
    pub sunset: Option<String>,
}

impl VersionDecision {
    pub fn apply_headers(&self, response: Response) -> Response {
        let response = response.header("API-Version", &self.selected.to_string());
        if self.deprecated {
            let response = response.header("Deprecation", "true");
            if let Some(sunset) = &self.sunset {
                return response.header("Sunset", sunset);
            }
            response
        } else {
            response
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionError {
    Missing,
    Invalid,
    Unsupported,
}

pub struct VersionNegotiator {
    header_name: String,
    supported: Vec<ApiVersion>,
    default: Option<ApiVersion>,
    deprecated: Vec<(ApiVersion, Option<String>)>,
}

impl VersionNegotiator {
    pub fn new(supported: impl IntoIterator<Item = ApiVersion>) -> Self {
        Self {
            header_name: "Accept-Version".to_string(),
            supported: supported.into_iter().collect(),
            default: None,
            deprecated: Vec::new(),
        }
    }

    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }
    pub fn default(mut self, version: ApiVersion) -> Self {
        self.default = Some(version);
        self
    }
    pub fn deprecated(mut self, version: ApiVersion, sunset: Option<String>) -> Self {
        self.deprecated.push((version, sunset));
        self
    }

    pub fn negotiate(&self, request: &Request) -> Result<VersionDecision, VersionError> {
        let requested = request
            .header(&self.header_name)
            .map(ApiVersion::parse)
            .transpose()?;
        let selected = requested.or(self.default).ok_or(VersionError::Missing)?;
        if !self.supported.contains(&selected) {
            return Err(VersionError::Unsupported);
        }
        let sunset = self
            .deprecated
            .iter()
            .find(|(version, _)| *version == selected)
            .and_then(|(_, sunset)| sunset.clone());
        Ok(VersionDecision {
            selected,
            deprecated: sunset.is_some()
                || self
                    .deprecated
                    .iter()
                    .any(|(version, _)| *version == selected),
            sunset,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Body, Method, Request as HyperRequest};

    async fn request(version: Option<&str>) -> Request {
        let mut builder = HyperRequest::builder().method(Method::GET).uri("/");
        if let Some(version) = version {
            builder = builder.header("Accept-Version", version);
        }
        Request::from_hyper(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[test]
    fn parses_common_version_forms() {
        assert_eq!(ApiVersion::parse("v1").unwrap(), ApiVersion::new(1, 0));
        assert_eq!(ApiVersion::parse("1.2").unwrap(), ApiVersion::new(1, 2));
        assert!(matches!(
            ApiVersion::parse("v1.2.3"),
            Err(VersionError::Invalid)
        ));
    }

    #[tokio::test]
    async fn negotiates_default_and_deprecated_versions() {
        let negotiator = VersionNegotiator::new([ApiVersion::new(1, 0), ApiVersion::new(2, 0)])
            .default(ApiVersion::new(2, 0))
            .deprecated(ApiVersion::new(1, 0), Some("2027-01-01".to_string()));
        let default = negotiator.negotiate(&request(None).await).unwrap();
        assert_eq!(default.selected, ApiVersion::new(2, 0));
        let old = negotiator.negotiate(&request(Some("v1")).await).unwrap();
        assert!(old.deprecated);
        assert_eq!(old.sunset.as_deref(), Some("2027-01-01"));
    }

    #[tokio::test]
    async fn rejects_invalid_and_unsupported_versions() {
        let negotiator = VersionNegotiator::new([ApiVersion::new(1, 0)]);
        assert_eq!(
            negotiator.negotiate(&request(Some("bad")).await),
            Err(VersionError::Invalid)
        );
        assert_eq!(
            negotiator.negotiate(&request(Some("v3")).await),
            Err(VersionError::Unsupported)
        );
        assert_eq!(
            negotiator.negotiate(&request(None).await),
            Err(VersionError::Missing)
        );
    }
}
