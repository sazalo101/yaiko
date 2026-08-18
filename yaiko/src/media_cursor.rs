//! Collaborative cursor and selection state for shared media-editor sessions.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorError {
    InvalidId,
    InvalidCoordinate,
    NotFound,
    ScopeMismatch,
    StaleRevision,
    Capacity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub x_milli: u32,
    pub y_milli: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorState {
    pub user_id: String,
    pub position: CursorPosition,
    pub selection_start_ms: Option<u64>,
    pub selection_end_ms: Option<u64>,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSnapshot {
    pub project_id: String,
    pub scope: String,
    pub cursors: Vec<CursorState>,
}
#[derive(Debug, Clone)]
struct ProjectCursors {
    scope: String,
    users: HashMap<String, CursorState>,
}
#[derive(Debug, Clone)]
pub struct MediaCursorStore {
    inner: Arc<Mutex<HashMap<String, ProjectCursors>>>,
    max_projects: usize,
    max_users: usize,
}
impl MediaCursorStore {
    pub fn new(max_projects: usize, max_users: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_projects: max_projects.max(1),
            max_users: max_users.max(1),
        }
    }
    pub fn update(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
        user_id: impl Into<String>,
        position: CursorPosition,
        selection: Option<(u64, u64)>,
        revision: u64,
    ) -> Result<CursorSnapshot, CursorError> {
        let project_id = valid(project_id.into())?;
        let scope = valid(scope.into())?;
        let user_id = valid(user_id.into())?;
        validate_position(position)?;
        if let Some((start, end)) = selection {
            if end <= start {
                return Err(CursorError::InvalidCoordinate);
            }
        }
        let mut guard = self.inner.lock().unwrap();
        if !guard.contains_key(&project_id) && guard.len() >= self.max_projects {
            return Err(CursorError::Capacity);
        }
        let project = guard
            .entry(project_id.clone())
            .or_insert_with(|| ProjectCursors {
                scope: scope.clone(),
                users: HashMap::new(),
            });
        if project.scope != scope {
            return Err(CursorError::ScopeMismatch);
        }
        if !project.users.contains_key(&user_id) && project.users.len() >= self.max_users {
            return Err(CursorError::Capacity);
        }
        if project
            .users
            .get(&user_id)
            .is_some_and(|existing| revision < existing.revision)
        {
            return Err(CursorError::StaleRevision);
        }
        let (selection_start_ms, selection_end_ms) =
            selection.map_or((None, None), |(start, end)| (Some(start), Some(end)));
        project.users.insert(
            user_id.clone(),
            CursorState {
                user_id,
                position,
                selection_start_ms,
                selection_end_ms,
                revision,
            },
        );
        Ok(snapshot(&project_id, &project.scope, &project.users))
    }
    pub fn remove(
        &self,
        project_id: &str,
        scope: &str,
        user_id: &str,
    ) -> Result<CursorSnapshot, CursorError> {
        let mut guard = self.inner.lock().unwrap();
        let project = guard.get_mut(project_id).ok_or(CursorError::NotFound)?;
        if project.scope != scope {
            return Err(CursorError::ScopeMismatch);
        }
        project.users.remove(user_id).ok_or(CursorError::NotFound)?;
        let result = snapshot(project_id, &project.scope, &project.users);
        if project.users.is_empty() {
            guard.remove(project_id);
        }
        Ok(result)
    }
    pub fn snapshot(&self, project_id: &str, scope: &str) -> Result<CursorSnapshot, CursorError> {
        let guard = self.inner.lock().unwrap();
        let project = guard.get(project_id).ok_or(CursorError::NotFound)?;
        if project.scope != scope {
            return Err(CursorError::ScopeMismatch);
        }
        Ok(snapshot(project_id, &project.scope, &project.users))
    }
}
fn valid(value: String) -> Result<String, CursorError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(CursorError::InvalidId);
    }
    Ok(value)
}
fn validate_position(position: CursorPosition) -> Result<(), CursorError> {
    if position.x_milli > 100_000 || position.y_milli > 100_000 {
        return Err(CursorError::InvalidCoordinate);
    }
    Ok(())
}
fn snapshot(project_id: &str, scope: &str, users: &HashMap<String, CursorState>) -> CursorSnapshot {
    let mut cursors: Vec<_> = users.values().cloned().collect();
    cursors.sort_by(|a, b| a.user_id.cmp(&b.user_id));
    CursorSnapshot {
        project_id: project_id.into(),
        scope: scope.into(),
        cursors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn position(x: u32, y: u32) -> CursorPosition {
        CursorPosition {
            x_milli: x,
            y_milli: y,
        }
    }
    #[test]
    fn updates_and_sorts_cursor_selection_snapshots() {
        let store = MediaCursorStore::new(2, 2);
        store
            .update(
                "project",
                "tenant",
                "user-b",
                position(20_000, 30_000),
                None,
                1,
            )
            .unwrap();
        let snapshot = store
            .update(
                "project",
                "tenant",
                "user-a",
                position(40_000, 50_000),
                Some((100, 200)),
                2,
            )
            .unwrap();
        assert_eq!(snapshot.cursors[0].user_id, "user-a");
        assert_eq!(snapshot.cursors[0].selection_end_ms, Some(200));
    }
    #[test]
    fn rejects_stale_scope_capacity_and_invalid_updates() {
        let store = MediaCursorStore::new(1, 1);
        store
            .update("project", "tenant", "user", position(0, 0), None, 3)
            .unwrap();
        assert_eq!(
            store.update("project", "tenant", "user", position(0, 0), None, 2),
            Err(CursorError::StaleRevision)
        );
        assert_eq!(
            store.update("project", "other", "user", position(0, 0), None, 4),
            Err(CursorError::ScopeMismatch)
        );
        assert_eq!(
            store.update("project", "tenant", "other", position(0, 0), None, 4),
            Err(CursorError::Capacity)
        );
        assert_eq!(
            store.update("project", "tenant", "user", position(100_001, 0), None, 4),
            Err(CursorError::InvalidCoordinate)
        );
        assert_eq!(
            store.update("project", "tenant", "user", position(0, 0), Some((5, 5)), 4),
            Err(CursorError::InvalidCoordinate)
        );
    }
}
