//! Safe public static-asset policy metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    NoCache,
    Immutable { max_age: u32 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticAssetError {
    InvalidPath,
    UnsupportedType,
    InvalidCache,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticAsset {
    pub path: String,
    pub content_type: String,
    pub cache: CachePolicy,
}
impl StaticAsset {
    pub fn new(path: impl Into<String>) -> Result<Self, StaticAssetError> {
        let path = path.into();
        if !path.starts_with('/')
            || path.contains("..")
            || path.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(StaticAssetError::InvalidPath);
        }
        let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let content_type = match ext.as_str() {
            "css" => "text/css",
            "js" => "text/javascript",
            "json" => "application/json",
            "html" => "text/html",
            "svg" => "image/svg+xml",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "avif" => "image/avif",
            "woff2" => "font/woff2",
            _ => return Err(StaticAssetError::UnsupportedType),
        }
        .into();
        Ok(Self {
            path,
            content_type,
            cache: CachePolicy::NoCache,
        })
    }
    pub fn cache(mut self, policy: CachePolicy) -> Result<Self, StaticAssetError> {
        if let CachePolicy::Immutable { max_age } = policy {
            if max_age == 0 {
                return Err(StaticAssetError::InvalidCache);
            }
        }
        self.cache = policy;
        Ok(self)
    }
    pub fn headers(&self) -> [(&str, String); 2] {
        let cache = match self.cache {
            CachePolicy::NoCache => "no-cache".into(),
            CachePolicy::Immutable { max_age } => format!("public, max-age={max_age}, immutable"),
        };
        [
            ("Content-Type", self.content_type.clone()),
            ("Cache-Control", cache),
        ]
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolves_types_and_cache_headers() {
        let a = StaticAsset::new("/app.js")
            .unwrap()
            .cache(CachePolicy::Immutable { max_age: 31536000 })
            .unwrap();
        assert_eq!(a.headers()[0], ("Content-Type", "text/javascript".into()));
        assert!(a.headers()[1].1.contains("immutable"))
    }
    #[test]
    fn rejects_traversal_unknown_types_and_bad_cache() {
        assert_eq!(
            StaticAsset::new("/../secret.js"),
            Err(StaticAssetError::InvalidPath)
        );
        assert_eq!(
            StaticAsset::new("/file.bin"),
            Err(StaticAssetError::UnsupportedType)
        );
        assert!(StaticAsset::new("/x.css")
            .unwrap()
            .cache(CachePolicy::Immutable { max_age: 0 })
            .is_err())
    }
}
