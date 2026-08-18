//! Content-encoding negotiation and compression policy helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionEncoding {
    Brotli,
    Gzip,
    Identity,
}

impl CompressionEncoding {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brotli => "br",
            Self::Gzip => "gzip",
            Self::Identity => "identity",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressionPolicy {
    pub min_size: usize,
    pub compressible_types: Vec<String>,
}

impl Default for CompressionPolicy {
    fn default() -> Self {
        Self {
            min_size: 1024,
            compressible_types: vec![
                "text/".into(),
                "application/json".into(),
                "application/javascript".into(),
                "image/svg+xml".into(),
            ],
        }
    }
}

impl CompressionPolicy {
    pub fn min_size(mut self, min_size: usize) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn should_compress(&self, content_type: Option<&str>, body_len: usize) -> bool {
        body_len >= self.min_size
            && content_type
                .map(|content_type| {
                    self.compressible_types
                        .iter()
                        .any(|allowed| content_type.starts_with(allowed))
                })
                .unwrap_or(false)
    }

    pub fn negotiate(
        &self,
        accept_encoding: &str,
        content_type: Option<&str>,
        body_len: usize,
    ) -> CompressionDecision {
        if !self.should_compress(content_type, body_len) {
            return CompressionDecision {
                encoding: CompressionEncoding::Identity,
                vary_accept_encoding: false,
            };
        }
        let mut br = 0.0;
        let mut gzip = 0.0;
        let mut wildcard = 0.0;
        for item in accept_encoding.split(',') {
            let mut parts = item.trim().split(';');
            let name = parts.next().unwrap_or_default().trim();
            let quality = parts
                .find_map(|part| {
                    part.trim()
                        .strip_prefix("q=")
                        .and_then(|value| value.parse::<f32>().ok())
                })
                .unwrap_or(1.0);
            match name {
                "br" => br = quality,
                "gzip" => gzip = quality,
                "*" => wildcard = quality,
                _ => {}
            }
        }
        let br = if br == 0.0 { wildcard } else { br };
        let gzip = if gzip == 0.0 { wildcard } else { gzip };
        let encoding = if br > 0.0 && br >= gzip {
            CompressionEncoding::Brotli
        } else if gzip > 0.0 {
            CompressionEncoding::Gzip
        } else {
            CompressionEncoding::Identity
        };
        CompressionDecision {
            encoding,
            vary_accept_encoding: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionDecision {
    pub encoding: CompressionEncoding,
    pub vary_accept_encoding: bool,
}

impl CompressionDecision {
    pub fn response_headers(self) -> Vec<(&'static str, String)> {
        let mut headers = vec![("Vary", "Accept-Encoding".to_string())];
        if self.encoding != CompressionEncoding::Identity {
            headers.push(("Content-Encoding", self.encoding.as_str().to_string()));
        }
        headers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiates_quality_values_and_prefers_brotli_on_tie() {
        let policy = CompressionPolicy::default().min_size(10);
        let decision = policy.negotiate("gzip;q=0.8, br;q=1.0", Some("application/json"), 100);
        assert_eq!(decision.encoding, CompressionEncoding::Brotli);
        let gzip = policy.negotiate("gzip;q=1, br;q=0", Some("application/json"), 100);
        assert_eq!(gzip.encoding, CompressionEncoding::Gzip);
    }

    #[test]
    fn honors_wildcard_and_rejects_unaccepted_encodings() {
        let policy = CompressionPolicy::default().min_size(10);
        assert_eq!(
            policy
                .negotiate("*;q=0.5", Some("text/plain"), 100)
                .encoding,
            CompressionEncoding::Brotli
        );
        assert_eq!(
            policy
                .negotiate("identity", Some("text/plain"), 100)
                .encoding,
            CompressionEncoding::Identity
        );
    }

    #[test]
    fn skips_small_and_incompressible_payloads() {
        let policy = CompressionPolicy::default().min_size(100);
        assert_eq!(
            policy
                .negotiate("br, gzip", Some("image/png"), 1000)
                .encoding,
            CompressionEncoding::Identity
        );
        assert!(!policy.should_compress(Some("application/octet-stream"), 1000));
        assert!(!policy.should_compress(Some("text/plain"), 99));
    }

    #[test]
    fn emits_vary_and_content_encoding_headers() {
        let headers = CompressionDecision {
            encoding: CompressionEncoding::Gzip,
            vary_accept_encoding: true,
        }
        .response_headers();
        assert_eq!(headers[0], ("Vary", "Accept-Encoding".to_string()));
        assert_eq!(headers[1], ("Content-Encoding", "gzip".to_string()));
    }
}
