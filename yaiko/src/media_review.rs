//! Scoped review workflow for collaborative media projects.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Pending,
    ChangesRequested,
    Approved,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewError {
    Invalid,
    Missing,
    Duplicate,
    Capacity,
    ScopeMismatch,
    RevisionConflict,
    StateConflict,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewRequest {
    pub id: String,
    pub project_id: String,
    pub scope: String,
    pub asset_version_id: String,
    pub requester: String,
    pub reviewer: Option<String>,
    pub feedback: Vec<String>,
    pub revision: u64,
    pub state: ReviewState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSnapshot {
    pub project_id: String,
    pub scope: String,
    pub requests: Vec<ReviewRequest>,
}
#[derive(Debug, Clone)]
pub struct MediaReviewStore {
    inner: Arc<Mutex<HashMap<String, Vec<ReviewRequest>>>>,
    max_projects: usize,
    max_feedback: usize,
}
impl MediaReviewStore {
    pub fn new(max_projects: usize, max_feedback: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_projects: max_projects.max(1),
            max_feedback: max_feedback.max(1),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
        id: impl Into<String>,
        asset_version_id: impl Into<String>,
        requester: impl Into<String>,
    ) -> Result<ReviewSnapshot, ReviewError> {
        let project_id = v(project_id.into())?;
        let scope = v(scope.into())?;
        let id = v(id.into())?;
        let asset_version_id = v(asset_version_id.into())?;
        let requester = v(requester.into())?;
        let mut g = self.inner.lock().unwrap();
        if !g.contains_key(&project_id) && g.len() >= self.max_projects {
            return Err(ReviewError::Capacity);
        }
        let r = g.entry(project_id.clone()).or_default();
        if r.iter().any(|x| x.id == id) {
            return Err(ReviewError::Duplicate);
        }
        r.push(ReviewRequest {
            id,
            project_id: project_id.clone(),
            scope: scope.clone(),
            asset_version_id,
            requester,
            reviewer: None,
            feedback: Vec::new(),
            revision: 1,
            state: ReviewState::Pending,
        });
        Ok(snap(&project_id, &scope, r))
    }
    pub fn assign(
        &self,
        project: &str,
        scope: &str,
        id: &str,
        expected: u64,
        reviewer: impl Into<String>,
    ) -> Result<ReviewSnapshot, ReviewError> {
        let reviewer = v(reviewer.into())?;
        let mut g = self.inner.lock().unwrap();
        let r = g.get_mut(project).ok_or(ReviewError::Missing)?;
        let x = r
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or(ReviewError::Missing)?;
        check(x, scope, expected)?;
        if x.reviewer.is_some() {
            return Err(ReviewError::StateConflict);
        }
        x.reviewer = Some(reviewer);
        x.revision += 1;
        Ok(snap(project, scope, r))
    }
    pub fn feedback(
        &self,
        project: &str,
        scope: &str,
        id: &str,
        expected: u64,
        text: impl Into<String>,
    ) -> Result<ReviewSnapshot, ReviewError> {
        let text = text.into();
        if text.is_empty() || text.len() > 4096 || text.chars().any(char::is_control) {
            return Err(ReviewError::Invalid);
        }
        let mut g = self.inner.lock().unwrap();
        let r = g.get_mut(project).ok_or(ReviewError::Missing)?;
        let x = r
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or(ReviewError::Missing)?;
        check(x, scope, expected)?;
        if x.feedback.len() >= self.max_feedback {
            return Err(ReviewError::Capacity);
        }
        x.feedback.push(text);
        x.revision += 1;
        Ok(snap(project, scope, r))
    }
    pub fn decide(
        &self,
        project: &str,
        scope: &str,
        id: &str,
        expected: u64,
        state: ReviewState,
    ) -> Result<ReviewSnapshot, ReviewError> {
        if state == ReviewState::Pending {
            return Err(ReviewError::StateConflict);
        }
        let mut g = self.inner.lock().unwrap();
        let r = g.get_mut(project).ok_or(ReviewError::Missing)?;
        let x = r
            .iter_mut()
            .find(|x| x.id == id)
            .ok_or(ReviewError::Missing)?;
        check(x, scope, expected)?;
        if x.reviewer.is_none() {
            return Err(ReviewError::StateConflict);
        }
        if x.state == state {
            return Err(ReviewError::StateConflict);
        }
        x.state = state;
        x.revision += 1;
        Ok(snap(project, scope, r))
    }
    pub fn snapshot(&self, project: &str, scope: &str) -> Result<ReviewSnapshot, ReviewError> {
        let g = self.inner.lock().unwrap();
        let r = g.get(project).ok_or(ReviewError::Missing)?;
        if r.iter().any(|x| x.scope != scope) {
            return Err(ReviewError::ScopeMismatch);
        }
        Ok(snap(project, scope, r))
    }
}
fn check(x: &ReviewRequest, scope: &str, expected: u64) -> Result<(), ReviewError> {
    if x.scope != scope {
        return Err(ReviewError::ScopeMismatch);
    }
    if x.revision != expected {
        return Err(ReviewError::RevisionConflict);
    }
    Ok(())
}
fn v(x: String) -> Result<String, ReviewError> {
    if x.is_empty() || x.len() > 128 || x.chars().any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        Err(ReviewError::Invalid)
    } else {
        Ok(x)
    }
}
fn snap(p: &str, s: &str, r: &[ReviewRequest]) -> ReviewSnapshot {
    let mut r = r.to_vec();
    r.sort_by(|a, b| a.id.cmp(&b.id));
    ReviewSnapshot {
        project_id: p.into(),
        scope: s.into(),
        requests: r,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assigns_feedback_and_decides() {
        let s = MediaReviewStore::new(2, 2);
        let x = s.create("p", "t", "r", "v1", "owner").unwrap().requests[0].clone();
        let a = s
            .assign("p", "t", "r", x.revision, "reviewer")
            .unwrap()
            .requests[0]
            .clone();
        let f = s
            .feedback("p", "t", "r", a.revision, "fix audio")
            .unwrap()
            .requests[0]
            .clone();
        let d = s
            .decide("p", "t", "r", f.revision, ReviewState::Approved)
            .unwrap();
        assert_eq!(d.requests[0].state, ReviewState::Approved)
    }
    #[test]
    fn rejects_scope_conflict_unassigned_and_capacity() {
        let s = MediaReviewStore::new(1, 1);
        let x = s.create("p", "t", "r", "v", "o").unwrap().requests[0].clone();
        assert_eq!(
            s.assign("p", "x", "r", 1, "u"),
            Err(ReviewError::ScopeMismatch)
        );
        assert_eq!(
            s.decide("p", "t", "r", x.revision, ReviewState::Approved),
            Err(ReviewError::StateConflict)
        );
        assert_eq!(
            s.feedback("p", "t", "r", x.revision, "a"),
            Ok(s.snapshot("p", "t").unwrap())
        );
        assert_eq!(
            s.feedback("p", "t", "r", 2, "b"),
            Err(ReviewError::Capacity)
        );
    }
}
