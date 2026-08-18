//! Deterministic bounded serialization for media-editor project snapshots.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotError {
    InvalidId,
    TooManyAssets,
    TimelineTooLarge,
    MetadataTooLarge,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotAsset {
    pub id: String,
    pub kind: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaProjectSnapshot {
    pub project_id: String,
    pub scope: String,
    pub revision: u64,
    pub assets: Vec<SnapshotAsset>,
    pub timeline: String,
}
#[derive(Debug, Clone, Serialize)]
struct WireSnapshot<'a> {
    project_id: &'a str,
    scope: &'a str,
    revision: u64,
    assets: &'a [SnapshotAsset],
    timeline: &'a str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedProjectSnapshot {
    pub json: String,
    pub revision: u64,
}

pub fn serialize_project_snapshot(
    snapshot: &MediaProjectSnapshot,
    max_assets: usize,
    max_timeline_bytes: usize,
    max_json_bytes: usize,
) -> Result<SerializedProjectSnapshot, SnapshotError> {
    validate_id(&snapshot.project_id)?;
    validate_id(&snapshot.scope)?;
    if snapshot.assets.len() > max_assets || max_assets == 0 {
        return Err(SnapshotError::TooManyAssets);
    }
    if snapshot.timeline.len() > max_timeline_bytes || max_timeline_bytes == 0 {
        return Err(SnapshotError::TimelineTooLarge);
    }
    if snapshot.assets.iter().any(|asset| {
        asset.id.is_empty()
            || asset.id.len() > 128
            || asset.kind.is_empty()
            || asset.kind.len() > 64
    }) {
        return Err(SnapshotError::InvalidId);
    }
    let wire = WireSnapshot {
        project_id: &snapshot.project_id,
        scope: &snapshot.scope,
        revision: snapshot.revision,
        assets: &snapshot.assets,
        timeline: &snapshot.timeline,
    };
    let json = serde_json::to_string(&wire).map_err(|_| SnapshotError::MetadataTooLarge)?;
    if json.len() > max_json_bytes || max_json_bytes == 0 {
        return Err(SnapshotError::MetadataTooLarge);
    }
    Ok(SerializedProjectSnapshot {
        json,
        revision: snapshot.revision,
    })
}
fn validate_id(value: &str) -> Result<(), SnapshotError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(SnapshotError::InvalidId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> MediaProjectSnapshot {
        MediaProjectSnapshot {
            project_id: "project-1".into(),
            scope: "tenant-1".into(),
            revision: 4,
            assets: vec![SnapshotAsset {
                id: "video.mp4".into(),
                kind: "video".into(),
            }],
            timeline: "timeline-v1".into(),
        }
    }
    #[test]
    fn serializes_deterministic_revisioned_snapshot() {
        let a = serialize_project_snapshot(&sample(), 4, 1024, 4096).unwrap();
        let b = serialize_project_snapshot(&sample(), 4, 1024, 4096).unwrap();
        assert_eq!(a, b);
        assert!(a.json.contains("project-1"));
        assert!(a.json.contains("\"revision\":4"));
        assert_eq!(a.revision, 4);
    }
    #[test]
    fn rejects_invalid_ids_and_bounds() {
        let mut snapshot = sample();
        snapshot.project_id = "../project".into();
        assert_eq!(
            serialize_project_snapshot(&snapshot, 4, 1024, 4096),
            Err(SnapshotError::InvalidId)
        );
        let mut snapshot = sample();
        snapshot.assets.push(SnapshotAsset {
            id: "audio.mp3".into(),
            kind: "audio".into(),
        });
        assert_eq!(
            serialize_project_snapshot(&snapshot, 1, 1024, 4096),
            Err(SnapshotError::TooManyAssets)
        );
        let mut snapshot = sample();
        snapshot.timeline = "x".repeat(10);
        assert_eq!(
            serialize_project_snapshot(&snapshot, 4, 4, 4096),
            Err(SnapshotError::TimelineTooLarge)
        );
    }
}
