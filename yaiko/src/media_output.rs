//! Validation for generated media outputs before publishing or completing tasks.

use crate::media_processing::MediaPath;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaOutputError {
    Missing,
    UnsafePath,
    UnsupportedFormat,
    Empty,
    TooLarge,
    InvalidSignature,
    Io,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedMediaOutput {
    pub path: MediaPath,
    pub size_bytes: u64,
    pub checksum_sha256: String,
    pub content_type: String,
}

#[derive(Debug, Clone)]
pub struct MediaOutputValidator {
    root: PathBuf,
    max_bytes: u64,
}

impl MediaOutputValidator {
    pub fn new(root: impl Into<PathBuf>, max_bytes: u64) -> Self {
        Self {
            root: root.into(),
            max_bytes: max_bytes.max(1),
        }
    }

    pub fn validate(
        &self,
        relative: impl Into<PathBuf>,
    ) -> Result<ValidatedMediaOutput, MediaOutputError> {
        let relative = MediaPath::new(relative).map_err(|_| MediaOutputError::UnsafePath)?;
        let extension = relative
            .as_path()
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        let content_type = match extension {
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "mkv" => "video/x-matroska",
            _ => return Err(MediaOutputError::UnsupportedFormat),
        };
        let path = self.root.join(relative.as_path());
        let metadata = fs::metadata(&path).map_err(|_| MediaOutputError::Missing)?;
        if !metadata.is_file() {
            return Err(MediaOutputError::Missing);
        }
        if metadata.len() == 0 {
            return Err(MediaOutputError::Empty);
        }
        if metadata.len() > self.max_bytes {
            return Err(MediaOutputError::TooLarge);
        }
        let bytes = fs::read(&path).map_err(|_| MediaOutputError::Io)?;
        if !has_media_signature(extension, &bytes) {
            return Err(MediaOutputError::InvalidSignature);
        }
        Ok(ValidatedMediaOutput {
            path: relative,
            size_bytes: metadata.len(),
            checksum_sha256: format!("sha256:{:x}", Sha256::digest(&bytes)),
            content_type: content_type.to_string(),
        })
    }
}

fn has_media_signature(extension: &str, bytes: &[u8]) -> bool {
    match extension {
        "webm" | "mkv" => bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        "mp4" => bytes.len() >= 8 && &bytes[4..8] == b"ftyp",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yaiko-media-output-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn validates_format_signature_size_and_checksum() {
        let root = temp_dir();
        let path = root.join("render.mp4");
        let mut bytes = vec![0, 0, 0, 24];
        bytes.extend_from_slice(b"ftypisom");
        bytes.extend_from_slice(b"payload");
        fs::write(&path, &bytes).unwrap();
        let output = MediaOutputValidator::new(&root, 1024)
            .validate("render.mp4")
            .unwrap();
        assert_eq!(output.content_type, "video/mp4");
        assert_eq!(output.size_bytes, bytes.len() as u64);
        assert!(output.checksum_sha256.starts_with("sha256:"));
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn rejects_unsafe_missing_large_and_malformed_outputs() {
        let root = temp_dir();
        let validator = MediaOutputValidator::new(&root, 4);
        assert_eq!(
            validator.validate("../escape.mp4"),
            Err(MediaOutputError::UnsafePath)
        );
        assert_eq!(
            validator.validate("missing.mp4"),
            Err(MediaOutputError::Missing)
        );
        fs::write(root.join("bad.mp4"), b"bad").unwrap();
        assert_eq!(
            validator.validate("bad.mp4"),
            Err(MediaOutputError::InvalidSignature)
        );
        fs::write(root.join("large.mp4"), b"0123456789").unwrap();
        assert_eq!(
            validator.validate("large.mp4"),
            Err(MediaOutputError::TooLarge)
        );
        let _ = fs::remove_dir_all(root);
    }
    #[test]
    fn accepts_webm_and_mkv_ebml_signature() {
        let root = temp_dir();
        for extension in ["webm", "mkv"] {
            let name = format!("clip.{extension}");
            fs::write(root.join(&name), [0x1A, 0x45, 0xDF, 0xA3, 0]).unwrap();
            assert!(MediaOutputValidator::new(&root, 100).validate(name).is_ok());
        }
        let _ = fs::remove_dir_all(root);
    }
}
