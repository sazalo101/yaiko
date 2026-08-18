//! Bounded revision history for collaborative media-editor projects.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionError {
    InvalidId,
    NotFound,
    Conflict,
    Capacity,
    RevisionUnavailable,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionRecord {
    pub project_id: String,
    pub scope: String,
    pub revision: u64,
    pub assets: Vec<String>,
    pub timeline: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionDiff {
    pub from: u64,
    pub to: u64,
    pub assets_changed: bool,
    pub timeline_changed: bool,
}
#[derive(Debug, Clone)]
pub struct MediaRevisionStore {
    inner: Arc<Mutex<HashMap<String, VecDeque<RevisionRecord>>>>,
    max_projects: usize,
    history_limit: usize,
}
impl MediaRevisionStore {
    pub fn new(max_projects: usize, history_limit: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_projects: max_projects.max(1),
            history_limit: history_limit.max(1),
        }
    }
    pub fn record(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
        revision: u64,
        assets: Vec<String>,
        timeline: impl Into<String>,
    ) -> Result<RevisionRecord, RevisionError> {
        let project_id = valid(project_id.into())?;
        let scope = valid(scope.into())?;
        if revision == 0 {
            return Err(RevisionError::Conflict);
        }
        let mut guard = self.inner.lock().unwrap();
        if !guard.contains_key(&project_id) && guard.len() >= self.max_projects {
            return Err(RevisionError::Capacity);
        }
        let history = guard.entry(project_id.clone()).or_default();
        if history
            .back()
            .is_some_and(|last| last.scope != scope || last.revision >= revision)
        {
            return Err(RevisionError::Conflict);
        }
        let record = RevisionRecord {
            project_id,
            scope,
            revision,
            assets,
            timeline: timeline.into(),
        };
        history.push_back(record.clone());
        while history.len() > self.history_limit {
            history.pop_front();
        }
        Ok(record)
    }
    pub fn get(
        &self,
        project_id: &str,
        scope: &str,
        revision: u64,
    ) -> Result<RevisionRecord, RevisionError> {
        let project_id = valid(project_id.to_string())?;
        let scope = valid(scope.to_string())?;
        let guard = self.inner.lock().unwrap();
        let history = guard.get(&project_id).ok_or(RevisionError::NotFound)?;
        history
            .iter()
            .find(|record| record.scope == scope && record.revision == revision)
            .cloned()
            .ok_or(RevisionError::RevisionUnavailable)
    }
    pub fn rollback(
        &self,
        project_id: &str,
        scope: &str,
        expected_revision: u64,
        target_revision: u64,
    ) -> Result<RevisionRecord, RevisionError> {
        let current = self.get(project_id, scope, expected_revision)?;
        let target = self.get(project_id, scope, target_revision)?;
        if target.revision >= current.revision {
            return Err(RevisionError::Conflict);
        }
        self.record(
            project_id,
            scope,
            current
                .revision
                .checked_add(1)
                .ok_or(RevisionError::Conflict)?,
            target.assets,
            target.timeline,
        )
    }
    pub fn diff(
        &self,
        project_id: &str,
        scope: &str,
        from: u64,
        to: u64,
    ) -> Result<RevisionDiff, RevisionError> {
        let before = self.get(project_id, scope, from)?;
        let after = self.get(project_id, scope, to)?;
        Ok(RevisionDiff {
            from,
            to,
            assets_changed: before.assets != after.assets,
            timeline_changed: before.timeline != after.timeline,
        })
    }
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
fn valid(value: String) -> Result<String, RevisionError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(RevisionError::InvalidId);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store() -> MediaRevisionStore {
        MediaRevisionStore::new(2, 3)
    }
    #[test]
    fn records_diffs_and_rolls_back() {
        let store = store();
        store
            .record("project", "tenant", 1, vec!["video".into()], "timeline-1")
            .unwrap();
        store
            .record(
                "project",
                "tenant",
                2,
                vec!["video", "music"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
                "timeline-2",
            )
            .unwrap();
        let diff = store.diff("project", "tenant", 1, 2).unwrap();
        assert!(diff.assets_changed && diff.timeline_changed);
        let rollback = store.rollback("project", "tenant", 2, 1).unwrap();
        assert_eq!(
            (rollback.revision, rollback.timeline),
            (3, "timeline-1".to_string())
        );
    }
    #[test]
    fn enforces_scope_order_retention_and_capacity() {
        let store = MediaRevisionStore::new(1, 2);
        store
            .record("project", "tenant", 1, Vec::new(), "a")
            .unwrap();
        store
            .record("project", "tenant", 2, Vec::new(), "b")
            .unwrap();
        store
            .record("project", "tenant", 3, Vec::new(), "c")
            .unwrap();
        assert_eq!(
            store.get("project", "tenant", 1),
            Err(RevisionError::RevisionUnavailable)
        );
        assert_eq!(
            store.record("project", "tenant", 3, Vec::new(), "dup"),
            Err(RevisionError::Conflict)
        );
        assert_eq!(
            store.record("other", "tenant", 1, Vec::new(), "x"),
            Err(RevisionError::Capacity)
        );
        assert_eq!(
            store.get("project", "other", 3),
            Err(RevisionError::RevisionUnavailable)
        );
    }
}
