//! Cross-origin and header policy helpers for media delivery.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryPolicyError {
    InvalidOrigin,
    InvalidFilename,
    WildcardWithCredentials,
    OriginDenied,
    RangeDenied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaDeliveryPolicy {
    origins: BTreeSet<String>,
    allow_credentials: bool,
    allow_ranges: bool,
    expose_etag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryHeaders {
    pub allow_origin: Option<String>,
    pub allow_credentials: bool,
    pub expose_headers: Vec<String>,
    pub content_disposition: Option<String>,
    pub accept_ranges: bool,
}

impl MediaDeliveryPolicy {
    pub fn new(
        origins: impl IntoIterator<Item = String>,
        allow_credentials: bool,
        allow_ranges: bool,
        expose_etag: bool,
    ) -> Result<Self, DeliveryPolicyError> {
        let origins = origins
            .into_iter()
            .map(validate_origin)
            .collect::<Result<BTreeSet<_>, _>>()?;
        if allow_credentials && origins.contains("*") {
            return Err(DeliveryPolicyError::WildcardWithCredentials);
        }
        Ok(Self {
            origins,
            allow_credentials,
            allow_ranges,
            expose_etag,
        })
    }
    pub fn evaluate(
        &self,
        origin: Option<&str>,
        range_requested: bool,
        download_name: Option<&str>,
    ) -> Result<DeliveryHeaders, DeliveryPolicyError> {
        let allow_origin = match origin {
            None => None,
            Some(value) => {
                let value = validate_origin(value.to_string())?;
                if self.origins.contains("*") || self.origins.contains(&value) {
                    Some(value)
                } else {
                    return Err(DeliveryPolicyError::OriginDenied);
                }
            }
        };
        if range_requested && !self.allow_ranges {
            return Err(DeliveryPolicyError::RangeDenied);
        }
        let content_disposition = download_name.map(safe_disposition).transpose()?;
        let expose_headers = if self.expose_etag {
            vec!["ETag".into(), "Content-Range".into()]
        } else {
            vec!["Content-Range".into()]
        };
        Ok(DeliveryHeaders {
            allow_origin,
            allow_credentials: self.allow_credentials,
            expose_headers,
            content_disposition,
            accept_ranges: self.allow_ranges,
        })
    }
}

fn validate_origin(origin: String) -> Result<String, DeliveryPolicyError> {
    if origin == "*" {
        return Ok(origin);
    }
    if origin.len() > 512
        || origin.chars().any(|c| c.is_control() || c.is_whitespace())
        || !(origin.starts_with("https://") || origin.starts_with("http://"))
        || origin.contains('/') && origin[origin.find("//").unwrap_or(0) + 2..].contains('/')
    {
        return Err(DeliveryPolicyError::InvalidOrigin);
    }
    Ok(origin)
}
fn safe_disposition(name: &str) -> Result<String, DeliveryPolicyError> {
    let filename = name.rsplit(['/', '\\']).next().unwrap_or_default();
    if filename.is_empty()
        || filename.len() > 255
        || filename
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '\r' | '\n'))
    {
        return Err(DeliveryPolicyError::InvalidFilename);
    }
    let sanitized: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(format!("attachment; filename=\"{}\"", sanitized))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evaluates_allowed_origin_headers_and_filename() {
        let policy =
            MediaDeliveryPolicy::new(vec!["https://editor.example".into()], true, true, true)
                .unwrap();
        let headers = policy
            .evaluate(
                Some("https://editor.example"),
                true,
                Some("../final cut.mp4"),
            )
            .unwrap();
        assert_eq!(
            headers.allow_origin.as_deref(),
            Some("https://editor.example")
        );
        assert!(headers.allow_credentials);
        assert!(headers.accept_ranges);
        assert_eq!(
            headers.content_disposition.as_deref(),
            Some("attachment; filename=\"final_cut.mp4\"")
        );
        assert!(headers.expose_headers.contains(&"ETag".into()));
    }
    #[test]
    fn rejects_denied_origins_ranges_bad_origins_and_wildcard_credentials() {
        let policy =
            MediaDeliveryPolicy::new(vec!["https://editor.example".into()], false, false, false)
                .unwrap();
        assert_eq!(
            policy.evaluate(Some("https://evil.example"), false, None),
            Err(DeliveryPolicyError::OriginDenied)
        );
        assert_eq!(
            policy.evaluate(None, true, None),
            Err(DeliveryPolicyError::RangeDenied)
        );
        assert_eq!(
            MediaDeliveryPolicy::new(vec!["*".into()], true, true, false),
            Err(DeliveryPolicyError::WildcardWithCredentials)
        );
        assert_eq!(
            MediaDeliveryPolicy::new(vec!["not-origin".into()], false, true, false),
            Err(DeliveryPolicyError::InvalidOrigin)
        );
    }
    #[test]
    fn sanitizes_and_rejects_unsafe_download_names() {
        let policy = MediaDeliveryPolicy::new(Vec::new(), false, true, false).unwrap();
        assert_eq!(
            policy
                .evaluate(None, false, Some("report final.mp4"))
                .unwrap()
                .content_disposition
                .as_deref(),
            Some("attachment; filename=\"report_final.mp4\"")
        );
        assert_eq!(
            policy.evaluate(None, false, Some("bad\nname.mp4")),
            Err(DeliveryPolicyError::InvalidFilename)
        );
    }
}
