//! Immutable metadata manifests for uploaded and generated media assets.

use crate::media_processing::MediaPath;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    UnsafePath,
    InvalidScope,
    InvalidMediaType,
    InvalidChecksum,
    InvalidDimensions,
    InvalidDuration,
    DuplicateId,
    Missing,
    Capacity,
    LineageTooLong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaManifest {
    pub id: String,
    pub scope: String,
    pub path: MediaPath,
    pub media_type: String,
    pub size_bytes: u64,
    pub checksum_sha256: String,
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
    pub parent_id: Option<String>,
    pub lineage_depth: u8,
}

#[derive(Debug, Clone)]
pub struct MediaManifestStore {
    inner: Arc<Mutex<HashMap<String, MediaManifest>>>,
    max_entries: usize,
}

impl MediaManifestStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_entries: max_entries.max(1),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &self,
        id: impl Into<String>,
        scope: impl Into<String>,
        path: impl Into<PathBuf>,
        media_type: impl Into<String>,
        bytes: &[u8],
        width: u32,
        height: u32,
        duration_ms: u64,
        parent_id: Option<String>,
    ) -> Result<MediaManifest, ManifestError> {
        let id = validate_id(id.into())?;
        let scope = validate_scope(scope.into())?;
        let path = MediaPath::new(path).map_err(|_| ManifestError::UnsafePath)?;
        let media_type = media_type.into();
        if !matches!(
            media_type.as_str(),
            "video/mp4"
                | "video/webm"
                | "video/x-matroska"
                | "audio/mpeg"
                | "audio/mp4"
                | "audio/wav"
        ) {
            return Err(ManifestError::InvalidMediaType);
        }
        if bytes.is_empty() {
            return Err(ManifestError::InvalidChecksum);
        }
        if width > 7680 || height > 4320 || (width == 0) != (height == 0) {
            return Err(ManifestError::InvalidDimensions);
        }
        if duration_ms > 86_400_000 {
            return Err(ManifestError::InvalidDuration);
        }
        let mut guard = self.inner.lock().unwrap();
        if guard.len() >= self.max_entries {
            return Err(ManifestError::Capacity);
        }
        if guard.contains_key(&id) {
            return Err(ManifestError::DuplicateId);
        }
        let lineage_depth = match parent_id.as_ref() {
            Some(parent) => guard
                .get(parent)
                .map(|manifest| manifest.lineage_depth.saturating_add(1))
                .ok_or(ManifestError::Missing)?,
            None => 0,
        };
        if lineage_depth > 32 {
            return Err(ManifestError::LineageTooLong);
        }
        let manifest = MediaManifest {
            id: id.clone(),
            scope,
            path,
            media_type,
            size_bytes: bytes.len() as u64,
            checksum_sha256: format!("sha256:{:x}", Sha256::digest(bytes)),
            width,
            height,
            duration_ms,
            parent_id,
            lineage_depth,
        };
        guard.insert(id, manifest.clone());
        Ok(manifest)
    }
    pub fn get(&self, scope: &str, id: &str) -> Result<MediaManifest, ManifestError> {
        let guard = self.inner.lock().unwrap();
        let manifest = guard.get(id).ok_or(ManifestError::Missing)?;
        if manifest.scope != scope {
            return Err(ManifestError::Missing);
        }
        Ok(manifest.clone())
    }
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn validate_id(id: String) -> Result<String, ManifestError> {
    if id.is_empty()
        || id.len() > 128
        || id
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(ManifestError::DuplicateId);
    }
    Ok(id)
}
fn validate_scope(scope: String) -> Result<String, ManifestError> {
    if scope.is_empty()
        || scope.len() > 128
        || scope
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(ManifestError::InvalidScope);
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> MediaManifestStore {
        MediaManifestStore::new(4)
    }
    #[test]
    fn inserts_and_scoped_looks_up_immutable_manifest() {
        let store = store();
        let manifest = store
            .insert(
                "asset-1",
                "tenant-a",
                "renders/video.mp4",
                "video/mp4",
                b"bytes",
                1920,
                1080,
                12_000,
                None,
            )
            .unwrap();
        assert_eq!(
            manifest.checksum_sha256,
            format!("sha256:{:x}", Sha256::digest(b"bytes"))
        );
        assert_eq!(store.get("tenant-a", "asset-1").unwrap(), manifest);
        assert_eq!(
            store.get("tenant-b", "asset-1"),
            Err(ManifestError::Missing)
        );
    }
    #[test]
    fn validates_types_dimensions_duration_paths_and_capacity() {
        let store = MediaManifestStore::new(1);
        assert_eq!(
            store.insert(
                "asset-1",
                "tenant-a",
                "../bad.mp4",
                "video/mp4",
                b"x",
                1,
                1,
                1,
                None
            ),
            Err(ManifestError::UnsafePath)
        );
        assert_eq!(
            store.insert(
                "asset-1",
                "tenant-a",
                "a.exe",
                "application/octet-stream",
                b"x",
                1,
                1,
                1,
                None
            ),
            Err(ManifestError::InvalidMediaType)
        );
        assert_eq!(
            store.insert(
                "asset-1",
                "tenant-a",
                "a.mp4",
                "video/mp4",
                b"x",
                1,
                0,
                1,
                None
            ),
            Err(ManifestError::InvalidDimensions)
        );
        assert_eq!(
            store.insert(
                "asset-1",
                "tenant-a",
                "a.mp4",
                "video/mp4",
                b"x",
                1,
                1,
                86_400_001,
                None
            ),
            Err(ManifestError::InvalidDuration)
        );
        store
            .insert(
                "asset-1",
                "tenant-a",
                "a.mp4",
                "video/mp4",
                b"x",
                1,
                1,
                1,
                None,
            )
            .unwrap();
        assert_eq!(
            store.insert(
                "asset-2",
                "tenant-a",
                "b.mp4",
                "video/mp4",
                b"x",
                1,
                1,
                1,
                None
            ),
            Err(ManifestError::Capacity)
        );
    }
    #[test]
    fn tracks_lineage_and_rejects_missing_parent_or_duplicates() {
        let store = store();
        store
            .insert(
                "source",
                "tenant-a",
                "source.mp4",
                "video/mp4",
                b"source",
                1280,
                720,
                1000,
                None,
            )
            .unwrap();
        let derived = store
            .insert(
                "derived",
                "tenant-a",
                "derived.mp4",
                "video/mp4",
                b"derived",
                1280,
                720,
                1000,
                Some("source".into()),
            )
            .unwrap();
        assert_eq!(derived.lineage_depth, 1);
        assert_eq!(
            store.insert(
                "derived",
                "tenant-a",
                "other.mp4",
                "video/mp4",
                b"x",
                1,
                1,
                1,
                None
            ),
            Err(ManifestError::DuplicateId)
        );
        assert_eq!(
            store.insert(
                "other",
                "tenant-a",
                "other.mp4",
                "video/mp4",
                b"x",
                1,
                1,
                1,
                Some("missing".into())
            ),
            Err(ManifestError::Missing)
        );
    }
}
