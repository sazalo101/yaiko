//! Collaborative timeline annotations for shared media-editor projects.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnotationError {
    InvalidId,
    InvalidText,
    InvalidRange,
    NotFound,
    ScopeMismatch,
    RevisionConflict,
    Capacity,
    StateConflict,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationState {
    Open,
    Resolved,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationReply {
    pub id: String,
    pub author: String,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAnnotation {
    pub id: String,
    pub project_id: String,
    pub scope: String,
    pub author: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub revision: u64,
    pub text: String,
    pub state: AnnotationState,
    pub replies: Vec<AnnotationReply>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationSnapshot {
    pub project_id: String,
    pub scope: String,
    pub annotations: Vec<MediaAnnotation>,
}
#[derive(Debug, Clone)]
pub struct MediaAnnotationStore {
    inner: Arc<Mutex<HashMap<String, Vec<MediaAnnotation>>>>,
    max_projects: usize,
    max_annotations: usize,
    max_replies: usize,
}
impl MediaAnnotationStore {
    pub fn new(max_projects: usize, max_annotations: usize, max_replies: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_projects: max_projects.max(1),
            max_annotations: max_annotations.max(1),
            max_replies: max_replies.max(1),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
        author: impl Into<String>,
        id: impl Into<String>,
        start_ms: u64,
        end_ms: u64,
        revision: u64,
        text: impl Into<String>,
    ) -> Result<AnnotationSnapshot, AnnotationError> {
        let project_id = valid(project_id.into())?;
        let scope = valid(scope.into())?;
        let author = valid(author.into())?;
        let id = valid(id.into())?;
        let text = valid_text(text.into())?;
        if end_ms <= start_ms {
            return Err(AnnotationError::InvalidRange);
        }
        if revision == 0 {
            return Err(AnnotationError::RevisionConflict);
        }
        let mut guard = self.inner.lock().unwrap();
        if !guard.contains_key(&project_id) && guard.len() >= self.max_projects {
            return Err(AnnotationError::Capacity);
        }
        let annotations = guard.entry(project_id.clone()).or_default();
        if annotations.len() >= self.max_annotations
            || annotations.iter().any(|annotation| annotation.id == id)
        {
            return Err(if annotations.len() >= self.max_annotations {
                AnnotationError::Capacity
            } else {
                AnnotationError::StateConflict
            });
        }
        annotations.push(MediaAnnotation {
            id,
            project_id: project_id.clone(),
            scope: scope.clone(),
            author,
            start_ms,
            end_ms,
            revision,
            text,
            state: AnnotationState::Open,
            replies: Vec::new(),
        });
        Ok(snapshot(&project_id, &scope, annotations))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn reply(
        &self,
        project_id: &str,
        scope: &str,
        annotation_id: &str,
        expected_revision: u64,
        reply_id: impl Into<String>,
        author: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<AnnotationSnapshot, AnnotationError> {
        let reply_id = valid(reply_id.into())?;
        let author = valid(author.into())?;
        let text = valid_text(text.into())?;
        let mut guard = self.inner.lock().unwrap();
        let annotations = guard.get_mut(project_id).ok_or(AnnotationError::NotFound)?;
        let annotation = annotations
            .iter_mut()
            .find(|annotation| annotation.id == annotation_id)
            .ok_or(AnnotationError::NotFound)?;
        if annotation.scope != scope {
            return Err(AnnotationError::ScopeMismatch);
        }
        if annotation.revision != expected_revision {
            return Err(AnnotationError::RevisionConflict);
        }
        if annotation.replies.len() >= self.max_replies
            || annotation.replies.iter().any(|reply| reply.id == reply_id)
        {
            return Err(AnnotationError::Capacity);
        }
        annotation.replies.push(AnnotationReply {
            id: reply_id,
            author,
            text,
        });
        Ok(snapshot(project_id, scope, annotations))
    }
    pub fn set_state(
        &self,
        project_id: &str,
        scope: &str,
        annotation_id: &str,
        expected_revision: u64,
        state: AnnotationState,
    ) -> Result<AnnotationSnapshot, AnnotationError> {
        let mut guard = self.inner.lock().unwrap();
        let annotations = guard.get_mut(project_id).ok_or(AnnotationError::NotFound)?;
        let annotation = annotations
            .iter_mut()
            .find(|annotation| annotation.id == annotation_id)
            .ok_or(AnnotationError::NotFound)?;
        if annotation.scope != scope {
            return Err(AnnotationError::ScopeMismatch);
        }
        if annotation.revision != expected_revision {
            return Err(AnnotationError::RevisionConflict);
        }
        if annotation.state == state {
            return Err(AnnotationError::StateConflict);
        }
        annotation.state = state;
        annotation.revision = annotation
            .revision
            .checked_add(1)
            .ok_or(AnnotationError::RevisionConflict)?;
        Ok(snapshot(project_id, scope, annotations))
    }
    pub fn snapshot(
        &self,
        project_id: &str,
        scope: &str,
    ) -> Result<AnnotationSnapshot, AnnotationError> {
        let guard = self.inner.lock().unwrap();
        let annotations = guard.get(project_id).ok_or(AnnotationError::NotFound)?;
        if annotations
            .iter()
            .any(|annotation| annotation.scope != scope)
        {
            return Err(AnnotationError::ScopeMismatch);
        }
        Ok(snapshot(project_id, scope, annotations))
    }
}
fn valid(value: String) -> Result<String, AnnotationError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(AnnotationError::InvalidId);
    }
    Ok(value)
}
fn valid_text(value: String) -> Result<String, AnnotationError> {
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(AnnotationError::InvalidText);
    }
    Ok(value)
}
fn snapshot(project_id: &str, scope: &str, annotations: &[MediaAnnotation]) -> AnnotationSnapshot {
    let mut annotations = annotations.to_vec();
    annotations.sort_by(|a, b| a.start_ms.cmp(&b.start_ms).then_with(|| a.id.cmp(&b.id)));
    AnnotationSnapshot {
        project_id: project_id.into(),
        scope: scope.into(),
        annotations,
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    #[test]
    fn creates_replies_resolves_and_orders() {
        let store = MediaAnnotationStore::new(2, 4, 2);
        let first = store
            .create(
                "project", "tenant", "author", "note-b", 100, 200, 1, "later",
            )
            .unwrap();
        let second = store
            .create("project", "tenant", "author", "note-a", 0, 50, 1, "intro")
            .unwrap();
        assert_eq!(second.annotations[0].id, "note-a");
        let replied = store
            .reply(
                "project",
                "tenant",
                "note-a",
                1,
                "reply-1",
                "reviewer",
                "looks good",
            )
            .unwrap();
        assert_eq!(replied.annotations[0].replies.len(), 1);
        let resolved = store
            .set_state("project", "tenant", "note-a", 1, AnnotationState::Resolved)
            .unwrap();
        assert_eq!(resolved.annotations[0].state, AnnotationState::Resolved);
        assert_eq!(first.annotations.len(), 1);
    }
    #[test]
    fn rejects_invalid_scope_revision_and_bounds() {
        let store = MediaAnnotationStore::new(1, 1, 1);
        store
            .create("project", "tenant", "author", "note", 0, 100, 1, "text")
            .unwrap();
        assert_eq!(
            store.reply("project", "other", "note", 1, "reply", "author", "text"),
            Err(AnnotationError::ScopeMismatch)
        );
        assert_eq!(
            store.set_state("project", "tenant", "note", 2, AnnotationState::Resolved),
            Err(AnnotationError::RevisionConflict)
        );
        assert_eq!(
            store.create("project", "tenant", "author", "second", 0, 1, 1, "text"),
            Err(AnnotationError::Capacity)
        );
    }
}
