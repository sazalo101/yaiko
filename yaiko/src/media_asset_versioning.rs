//! Immutable, scoped versions for media assets and derived editor outputs.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetVersionState {
    Draft,
    Published,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetVersionError {
    InvalidId,
    InvalidScope,
    InvalidPath,
    InvalidChecksum,
    Missing,
    Duplicate,
    Capacity,
    RevisionConflict,
    StateConflict,
    ScopeMismatch,
    LineageTooLong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAssetVersion {
    pub asset_id: String,
    pub version_id: String,
    pub scope: String,
    pub path: String,
    pub checksum_sha256: String,
    pub bytes: u64,
    pub parent_version_id: Option<String>,
    pub lineage_depth: u8,
    pub revision: u64,
    pub state: AssetVersionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetVersionSnapshot {
    pub asset_id: String,
    pub scope: String,
    pub versions: Vec<MediaAssetVersion>,
}

#[derive(Debug, Clone)]
pub struct MediaAssetVersionStore {
    inner: Arc<Mutex<HashMap<String, Vec<MediaAssetVersion>>>>,
    max_assets: usize,
    max_versions_per_asset: usize,
}

impl MediaAssetVersionStore {
    pub fn new(max_assets: usize, max_versions_per_asset: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_assets: max_assets.max(1),
            max_versions_per_asset: max_versions_per_asset.max(1),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        asset_id: impl Into<String>,
        version_id: impl Into<String>,
        scope: impl Into<String>,
        path: impl Into<String>,
        checksum_sha256: impl Into<String>,
        bytes: u64,
        parent_version_id: Option<String>,
    ) -> Result<MediaAssetVersion, AssetVersionError> {
        let asset_id = valid_id(asset_id.into())?;
        let version_id = valid_id(version_id.into())?;
        let scope = valid_scope(scope.into())?;
        let path = valid_path(path.into())?;
        let checksum_sha256 = valid_checksum(checksum_sha256.into())?;
        if bytes == 0 {
            return Err(AssetVersionError::InvalidChecksum);
        }
        let mut guard = self.inner.lock().expect("asset versions poisoned");
        if !guard.contains_key(&asset_id) && guard.len() >= self.max_assets {
            return Err(AssetVersionError::Capacity);
        }
        let versions = guard.entry(asset_id.clone()).or_default();
        if versions.len() >= self.max_versions_per_asset {
            return Err(AssetVersionError::Capacity);
        }
        if versions
            .iter()
            .any(|version| version.version_id == version_id)
        {
            return Err(AssetVersionError::Duplicate);
        }
        let lineage_depth = match parent_version_id.as_ref() {
            Some(parent) => versions
                .iter()
                .find(|version| version.version_id == *parent)
                .map(|version| version.lineage_depth.saturating_add(1))
                .ok_or(AssetVersionError::Missing)?,
            None => 0,
        };
        if lineage_depth > 32 {
            return Err(AssetVersionError::LineageTooLong);
        }
        let version = MediaAssetVersion {
            asset_id,
            version_id,
            scope,
            path,
            checksum_sha256,
            bytes,
            parent_version_id,
            lineage_depth,
            revision: 1,
            state: AssetVersionState::Draft,
        };
        versions.push(version.clone());
        Ok(version)
    }

    pub fn publish(
        &self,
        asset_id: &str,
        scope: &str,
        version_id: &str,
        expected_revision: u64,
    ) -> Result<AssetVersionSnapshot, AssetVersionError> {
        let mut guard = self.inner.lock().expect("asset versions poisoned");
        let versions = guard.get_mut(asset_id).ok_or(AssetVersionError::Missing)?;
        let target_index = versions
            .iter()
            .position(|version| version.version_id == version_id)
            .ok_or(AssetVersionError::Missing)?;
        let target = &versions[target_index];
        if target.scope != scope {
            return Err(AssetVersionError::ScopeMismatch);
        }
        if target.revision != expected_revision {
            return Err(AssetVersionError::RevisionConflict);
        }
        if target.state != AssetVersionState::Draft {
            return Err(AssetVersionError::StateConflict);
        }
        for (index, version) in versions.iter_mut().enumerate() {
            if index != target_index
                && version.scope == scope
                && version.state == AssetVersionState::Published
            {
                version.state = AssetVersionState::Archived;
                version.revision = version.revision.saturating_add(1);
            }
        }
        let target = &mut versions[target_index];
        target.state = AssetVersionState::Published;
        target.revision = target.revision.saturating_add(1);
        Ok(snapshot(asset_id, scope, versions))
    }

    pub fn archive(
        &self,
        asset_id: &str,
        scope: &str,
        version_id: &str,
        expected_revision: u64,
    ) -> Result<AssetVersionSnapshot, AssetVersionError> {
        let mut guard = self.inner.lock().expect("asset versions poisoned");
        let versions = guard.get_mut(asset_id).ok_or(AssetVersionError::Missing)?;
        let target = versions
            .iter_mut()
            .find(|version| version.version_id == version_id)
            .ok_or(AssetVersionError::Missing)?;
        if target.scope != scope {
            return Err(AssetVersionError::ScopeMismatch);
        }
        if target.revision != expected_revision {
            return Err(AssetVersionError::RevisionConflict);
        }
        if target.state == AssetVersionState::Archived {
            return Err(AssetVersionError::StateConflict);
        }
        target.state = AssetVersionState::Archived;
        target.revision = target.revision.saturating_add(1);
        Ok(snapshot(asset_id, scope, versions))
    }

    pub fn snapshot(
        &self,
        asset_id: &str,
        scope: &str,
    ) -> Result<AssetVersionSnapshot, AssetVersionError> {
        let guard = self.inner.lock().expect("asset versions poisoned");
        let versions = guard.get(asset_id).ok_or(AssetVersionError::Missing)?;
        if versions.iter().any(|version| version.scope != scope) {
            return Err(AssetVersionError::ScopeMismatch);
        }
        Ok(snapshot(asset_id, scope, versions))
    }
}

fn snapshot(asset_id: &str, scope: &str, versions: &[MediaAssetVersion]) -> AssetVersionSnapshot {
    let mut versions = versions.to_vec();
    versions.sort_by(|left, right| left.version_id.cmp(&right.version_id));
    AssetVersionSnapshot {
        asset_id: asset_id.into(),
        scope: scope.into(),
        versions,
    }
}

fn valid_id(value: String) -> Result<String, AssetVersionError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\'))
    {
        return Err(AssetVersionError::InvalidId);
    }
    Ok(value)
}

fn valid_scope(value: String) -> Result<String, AssetVersionError> {
    valid_id(value).map_err(|_| AssetVersionError::InvalidScope)
}

fn valid_path(value: String) -> Result<String, AssetVersionError> {
    if value.is_empty()
        || value.len() > 1024
        || value.starts_with('/')
        || value.split('/').any(|part| part == ".." || part.is_empty())
    {
        return Err(AssetVersionError::InvalidPath);
    }
    Ok(value)
}

fn valid_checksum(value: String) -> Result<String, AssetVersionError> {
    if value.len() != 71
        || !value.starts_with("sha256:")
        || value[7..]
            .chars()
            .any(|character| !character.is_ascii_hexdigit())
    {
        return Err(AssetVersionError::InvalidChecksum);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checksum() -> &'static str {
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    }

    #[test]
    fn creates_lineage_publishes_and_archives_previous_version() {
        let store = MediaAssetVersionStore::new(2, 4);
        let first = store
            .create(
                "asset",
                "v2",
                "tenant",
                "renders/v2.mp4",
                checksum(),
                20,
                None,
            )
            .unwrap();
        let second = store
            .create(
                "asset",
                "v1",
                "tenant",
                "renders/v1.mp4",
                checksum(),
                10,
                Some("v2".into()),
            )
            .unwrap();
        assert_eq!(second.lineage_depth, 1);
        let published = store
            .publish("asset", "tenant", "v2", first.revision)
            .unwrap();
        assert_eq!(
            published
                .versions
                .iter()
                .find(|v| v.version_id == "v2")
                .unwrap()
                .state,
            AssetVersionState::Published
        );
        let switched = store
            .publish("asset", "tenant", "v1", second.revision)
            .unwrap();
        assert_eq!(
            switched
                .versions
                .iter()
                .find(|v| v.version_id == "v2")
                .unwrap()
                .state,
            AssetVersionState::Archived
        );
        assert_eq!(
            switched
                .versions
                .iter()
                .find(|v| v.version_id == "v1")
                .unwrap()
                .state,
            AssetVersionState::Published
        );
    }

    #[test]
    fn enforces_scope_revision_path_checksum_and_capacity() {
        let store = MediaAssetVersionStore::new(1, 1);
        assert_eq!(
            store.create("asset", "v1", "tenant", "../bad", checksum(), 1, None),
            Err(AssetVersionError::InvalidPath)
        );
        assert_eq!(
            store.create("asset", "v1", "tenant", "a.mp4", "bad", 1, None),
            Err(AssetVersionError::InvalidChecksum)
        );
        let version = store
            .create("asset", "v1", "tenant", "a.mp4", checksum(), 1, None)
            .unwrap();
        assert_eq!(
            store.publish("asset", "other", "v1", version.revision),
            Err(AssetVersionError::ScopeMismatch)
        );
        assert_eq!(
            store.publish("asset", "tenant", "v1", 99),
            Err(AssetVersionError::RevisionConflict)
        );
        assert_eq!(
            store.create("asset", "v2", "tenant", "b.mp4", checksum(), 1, None),
            Err(AssetVersionError::Capacity)
        );
    }
}
