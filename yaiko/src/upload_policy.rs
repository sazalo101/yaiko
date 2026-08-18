//! Hardened upload metadata and validation primitives.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPolicy {
    pub max_bytes: usize,
    pub allowed_content_types: BTreeSet<String>,
}

impl UploadPolicy {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            allowed_content_types: BTreeSet::new(),
        }
    }
    pub fn allow_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.allowed_content_types.insert(content_type.into());
        self
    }
    pub fn validate(
        &self,
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Result<UploadMetadata, UploadError> {
        let filename = sanitize_filename(filename)?;
        if bytes.len() > self.max_bytes {
            return Err(UploadError::TooLarge);
        }
        if !self.allowed_content_types.is_empty()
            && !self.allowed_content_types.contains(content_type)
        {
            return Err(UploadError::UnsupportedContentType);
        }
        Ok(UploadMetadata {
            filename,
            content_type: content_type.to_string(),
            size: bytes.len(),
            sha256: checksum(bytes),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadMetadata {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    TooLarge,
    UnsupportedContentType,
    InvalidFilename,
}

pub fn sanitize_filename(filename: &str) -> Result<String, UploadError> {
    let filename = filename.trim();
    if filename.is_empty()
        || filename == "."
        || filename == ".."
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains("..")
        || filename.chars().any(|character| character.is_control())
    {
        return Err(UploadError::InvalidFilename);
    }
    Ok(filename.to_string())
}

pub fn checksum(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Clone, Default)]
pub struct TempUploadGuard {
    paths: Arc<Mutex<Vec<String>>>,
}

impl TempUploadGuard {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn track(&self, path: impl Into<String>) {
        self.paths
            .lock()
            .expect("upload guard poisoned")
            .push(path.into());
    }
    pub fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.paths.lock().expect("upload guard poisoned"))
    }
}

impl Drop for TempUploadGuard {
    fn drop(&mut self) {
        if Arc::strong_count(&self.paths) == 1 {
            self.take();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_uploads_and_records_checksum_metadata() {
        let policy = UploadPolicy::new(10).allow_content_type("text/plain");
        let metadata = policy
            .validate("notes.txt", "text/plain", b"hello")
            .unwrap();
        assert_eq!(metadata.size, 5);
        assert_eq!(metadata.sha256, checksum(b"hello"));
    }

    #[test]
    fn rejects_size_type_and_path_violations() {
        let policy = UploadPolicy::new(2).allow_content_type("text/plain");
        assert_eq!(
            policy.validate("ok.txt", "text/plain", b"long"),
            Err(UploadError::TooLarge)
        );
        assert_eq!(
            policy.validate("ok.txt", "image/png", b"x"),
            Err(UploadError::UnsupportedContentType)
        );
        assert_eq!(
            policy.validate("../secret", "text/plain", b"x"),
            Err(UploadError::InvalidFilename)
        );
    }

    #[test]
    fn tracks_and_takes_temporary_paths() {
        let guard = TempUploadGuard::new();
        guard.track("/tmp/upload-1");
        guard.track("/tmp/upload-2");
        assert_eq!(guard.take(), vec!["/tmp/upload-1", "/tmp/upload-2"]);
        assert!(guard.take().is_empty());
    }
}
