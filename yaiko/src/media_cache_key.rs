//! Deterministic cache keys for media preview artifacts.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheKeyError {
    InvalidPath,
    InvalidProfile,
    TooLong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailCacheKey {
    pub key: String,
    pub source: String,
    pub profile: String,
    pub version: u32,
}

impl ThumbnailCacheKey {
    pub fn new(
        source: impl Into<String>,
        profile: impl Into<String>,
        version: u32,
    ) -> Result<Self, CacheKeyError> {
        let source = normalize_source(source.into())?;
        let profile = normalize_profile(profile.into())?;
        if version == 0 {
            return Err(CacheKeyError::InvalidProfile);
        }
        let digest = Sha256::digest(format!("v{}\0{}\0{}", version, source, profile).as_bytes());
        let key = format!("media:thumb:v{}:{}", version, hex(&digest));
        if key.len() > 128 {
            return Err(CacheKeyError::TooLong);
        }
        Ok(Self {
            key,
            source,
            profile,
            version,
        })
    }
    pub fn matches_source(&self, source: &str) -> bool {
        normalize_source(source.to_string())
            .map(|value| value == self.source)
            .unwrap_or(false)
    }
    pub fn matches_profile(&self, profile: &str) -> bool {
        normalize_profile(profile.to_string())
            .map(|value| value == self.profile)
            .unwrap_or(false)
    }
}

fn normalize_source(source: String) -> Result<String, CacheKeyError> {
    let source = source.trim().replace('\\', "/");
    if source.is_empty()
        || source.len() > 512
        || source.starts_with('/')
        || source
            .split('/')
            .any(|part| part == ".." || part.is_empty() && !source.is_empty())
    {
        return Err(CacheKeyError::InvalidPath);
    }
    Ok(source)
}
fn normalize_profile(profile: String) -> Result<String, CacheKeyError> {
    let profile = profile.trim().to_ascii_lowercase();
    if profile.is_empty()
        || profile.len() > 256
        || profile.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err(CacheKeyError::InvalidProfile);
    }
    Ok(profile)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonicalizes_sources_and_profiles_deterministically() {
        let a = ThumbnailCacheKey::new("renders\\video.mp4", " JPEG_1280 ", 1).unwrap();
        let b = ThumbnailCacheKey::new("renders/video.mp4", "jpeg_1280", 1).unwrap();
        assert_eq!(a, b);
        assert!(a.key.starts_with("media:thumb:v1:"));
        assert!(a.key.len() <= 128);
    }
    #[test]
    fn differentiates_profiles_versions_and_sources() {
        let base = ThumbnailCacheKey::new("video.mp4", "jpeg_640", 1).unwrap();
        assert_ne!(
            base,
            ThumbnailCacheKey::new("video.mp4", "jpeg_1280", 1).unwrap()
        );
        assert_ne!(
            base,
            ThumbnailCacheKey::new("video.mp4", "jpeg_640", 2).unwrap()
        );
        assert_ne!(
            base,
            ThumbnailCacheKey::new("other.mp4", "jpeg_640", 1).unwrap()
        );
        assert!(base.matches_source("video.mp4"));
        assert!(base.matches_profile(" JPEG_640 "));
    }
    #[test]
    fn rejects_unsafe_sources_profiles_and_versions() {
        assert_eq!(
            ThumbnailCacheKey::new("../video.mp4", "jpeg", 1),
            Err(CacheKeyError::InvalidPath)
        );
        assert_eq!(
            ThumbnailCacheKey::new("/video.mp4", "jpeg", 1),
            Err(CacheKeyError::InvalidPath)
        );
        assert_eq!(
            ThumbnailCacheKey::new("video.mp4", "", 1),
            Err(CacheKeyError::InvalidProfile)
        );
        assert_eq!(
            ThumbnailCacheKey::new("video.mp4", "jpeg profile", 1),
            Err(CacheKeyError::InvalidProfile)
        );
        assert_eq!(
            ThumbnailCacheKey::new("video.mp4", "jpeg", 0),
            Err(CacheKeyError::InvalidProfile)
        );
    }
}
