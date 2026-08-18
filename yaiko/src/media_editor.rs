//! Bounded media-editor project sessions with optimistic revisions.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorError {
    InvalidId,
    Capacity,
    NotFound,
    Conflict,
    TooManyAssets,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSnapshot {
    pub project_id: String,
    pub scope: String,
    pub revision: u64,
    pub assets: Vec<String>,
    pub timeline: String,
}
#[derive(Debug, Clone)]
struct Project {
    scope: String,
    revision: u64,
    assets: Vec<String>,
    timeline: String,
}
#[derive(Debug, Clone)]
pub struct MediaEditorStore {
    inner: Arc<Mutex<HashMap<String, Project>>>,
    max_projects: usize,
    max_assets: usize,
}
impl MediaEditorStore {
    pub fn new(max_projects: usize, max_assets: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_projects: max_projects.max(1),
            max_assets: max_assets.max(1),
        }
    }
    pub fn create(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
    ) -> Result<EditorSnapshot, EditorError> {
        let project_id = valid_id(project_id.into())?;
        let scope = valid_id(scope.into())?;
        let mut guard = self.inner.lock().unwrap();
        if guard.contains_key(&project_id) {
            return Err(EditorError::Conflict);
        }
        if guard.len() >= self.max_projects {
            return Err(EditorError::Capacity);
        }
        guard.insert(
            project_id.clone(),
            Project {
                scope: scope.clone(),
                revision: 0,
                assets: Vec::new(),
                timeline: String::new(),
            },
        );
        Ok(snapshot(
            &project_id,
            guard.get(&project_id).expect("inserted"),
        ))
    }
    pub fn add_asset(
        &self,
        project_id: &str,
        scope: &str,
        expected_revision: u64,
        asset_id: impl Into<String>,
    ) -> Result<EditorSnapshot, EditorError> {
        let asset_id = asset_id.into();
        self.update(project_id, scope, expected_revision, |project| {
            if project.assets.len() >= self.max_assets {
                return Err(EditorError::TooManyAssets);
            }
            let asset = valid_id(asset_id)?;
            if project.assets.contains(&asset) {
                return Err(EditorError::Conflict);
            }
            project.assets.push(asset);
            Ok(())
        })
    }
    pub fn set_timeline(
        &self,
        project_id: &str,
        scope: &str,
        expected_revision: u64,
        timeline: impl Into<String>,
    ) -> Result<EditorSnapshot, EditorError> {
        let timeline = timeline.into();
        self.update(project_id, scope, expected_revision, |project| {
            if timeline.len() > 16_384 {
                return Err(EditorError::TooManyAssets);
            }
            project.timeline = timeline;
            Ok(())
        })
    }
    pub fn snapshot(&self, project_id: &str, scope: &str) -> Result<EditorSnapshot, EditorError> {
        let project_id = valid_id(project_id.to_string())?;
        let scope = valid_id(scope.to_string())?;
        let guard = self.inner.lock().unwrap();
        let project = guard.get(&project_id).ok_or(EditorError::NotFound)?;
        if project.scope != scope {
            return Err(EditorError::NotFound);
        }
        Ok(snapshot(&project_id, project))
    }
    fn update<F>(
        &self,
        project_id: &str,
        scope: &str,
        expected_revision: u64,
        apply: F,
    ) -> Result<EditorSnapshot, EditorError>
    where
        F: FnOnce(&mut Project) -> Result<(), EditorError>,
    {
        let project_id = valid_id(project_id.to_string())?;
        let scope = valid_id(scope.to_string())?;
        let mut guard = self.inner.lock().unwrap();
        let project = guard.get_mut(&project_id).ok_or(EditorError::NotFound)?;
        if project.scope != scope {
            return Err(EditorError::NotFound);
        }
        if project.revision != expected_revision {
            return Err(EditorError::Conflict);
        }
        apply(project)?;
        project.revision = project
            .revision
            .checked_add(1)
            .ok_or(EditorError::Conflict)?;
        Ok(snapshot(&project_id, project))
    }
}
fn snapshot(project_id: &str, project: &Project) -> EditorSnapshot {
    EditorSnapshot {
        project_id: project_id.into(),
        scope: project.scope.clone(),
        revision: project.revision,
        assets: project.assets.clone(),
        timeline: project.timeline.clone(),
    }
}
fn valid_id(value: String) -> Result<String, EditorError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(EditorError::InvalidId);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_updates_and_enforces_revisions() {
        let store = MediaEditorStore::new(2, 2);
        let created = store.create("project-1", "tenant-1").unwrap();
        let updated = store
            .add_asset("project-1", "tenant-1", created.revision, "video.mp4")
            .unwrap();
        assert_eq!(updated.assets, vec!["video.mp4"]);
        assert_eq!(
            store.add_asset("project-1", "tenant-1", created.revision, "music.mp3"),
            Err(EditorError::Conflict)
        );
        let snapshot = store
            .set_timeline("project-1", "tenant-1", updated.revision, "timeline-v1")
            .unwrap();
        assert_eq!(snapshot.revision, 2);
    }
    #[test]
    fn isolates_scopes_and_bounds_assets_projects() {
        let store = MediaEditorStore::new(1, 1);
        let first = store.create("project-1", "tenant-1").unwrap();
        assert_eq!(
            store.snapshot("project-1", "tenant-2"),
            Err(EditorError::NotFound)
        );
        let asset = store
            .add_asset("project-1", "tenant-1", first.revision, "video.mp4")
            .unwrap();
        assert_eq!(
            store.add_asset("project-1", "tenant-1", asset.revision, "music.mp3"),
            Err(EditorError::TooManyAssets)
        );
        assert_eq!(
            store.create("project-2", "tenant-1"),
            Err(EditorError::Capacity)
        );
    }
}
