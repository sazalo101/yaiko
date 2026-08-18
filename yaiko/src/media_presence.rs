//! Bounded collaboration presence for shared media-editor projects.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceRole {
    Viewer,
    Editor,
    Owner,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresenceError {
    InvalidId,
    InvalidRole,
    NotFound,
    Conflict,
    Capacity,
    Expired,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub user_id: String,
    pub role: PresenceRole,
    pub last_seen: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSnapshot {
    pub project_id: String,
    pub scope: String,
    pub participants: Vec<Participant>,
}
type ProjectPresence = (String, HashMap<String, Participant>);

pub struct MediaPresenceStore {
    inner: Arc<Mutex<HashMap<String, ProjectPresence>>>,
    max_projects: usize,
    max_participants: usize,
    heartbeat_ttl: Duration,
}
impl MediaPresenceStore {
    pub fn new(max_projects: usize, max_participants: usize, heartbeat_ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_projects: max_projects.max(1),
            max_participants: max_participants.max(1),
            heartbeat_ttl,
        }
    }
    pub fn join(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
        user_id: impl Into<String>,
        role: PresenceRole,
        now: SystemTime,
    ) -> Result<PresenceSnapshot, PresenceError> {
        let project_id = valid(project_id.into())?;
        let scope = valid(scope.into())?;
        let user_id = valid(user_id.into())?;
        let mut guard = self.inner.lock().unwrap();
        if !guard.contains_key(&project_id) && guard.len() >= self.max_projects {
            return Err(PresenceError::Capacity);
        }
        let entry = guard
            .entry(project_id.clone())
            .or_insert_with(|| (scope.clone(), HashMap::new()));
        if entry.0 != scope {
            return Err(PresenceError::NotFound);
        }
        if entry.1.contains_key(&user_id) {
            return Err(PresenceError::Conflict);
        }
        if entry.1.len() >= self.max_participants {
            return Err(PresenceError::Capacity);
        }
        entry.1.insert(
            user_id.clone(),
            Participant {
                user_id,
                role,
                last_seen: now,
            },
        );
        Ok(snapshot(&project_id, &entry.0, &entry.1))
    }
    pub fn heartbeat(
        &self,
        project_id: &str,
        scope: &str,
        user_id: &str,
        now: SystemTime,
    ) -> Result<PresenceSnapshot, PresenceError> {
        let mut guard = self.inner.lock().unwrap();
        let (stored_scope, participants) =
            guard.get_mut(project_id).ok_or(PresenceError::NotFound)?;
        if stored_scope != scope {
            return Err(PresenceError::NotFound);
        }
        let participant = participants
            .get_mut(user_id)
            .ok_or(PresenceError::NotFound)?;
        if now
            .duration_since(participant.last_seen)
            .unwrap_or_default()
            > self.heartbeat_ttl
        {
            return Err(PresenceError::Expired);
        }
        participant.last_seen = now;
        Ok(snapshot(project_id, stored_scope, participants))
    }
    pub fn leave(
        &self,
        project_id: &str,
        scope: &str,
        user_id: &str,
    ) -> Result<PresenceSnapshot, PresenceError> {
        let mut guard = self.inner.lock().unwrap();
        let (stored_scope, participants) =
            guard.get_mut(project_id).ok_or(PresenceError::NotFound)?;
        if stored_scope != scope {
            return Err(PresenceError::NotFound);
        }
        participants
            .remove(user_id)
            .ok_or(PresenceError::NotFound)?;
        let result = snapshot(project_id, stored_scope, participants);
        if participants.is_empty() {
            guard.remove(project_id);
        }
        Ok(result)
    }
    pub fn cleanup_expired(&self, now: SystemTime) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let ttl = self.heartbeat_ttl;
        let mut removed = 0;
        for (_, participants) in guard.values_mut() {
            participants.retain(|_, participant| {
                let active = now
                    .duration_since(participant.last_seen)
                    .unwrap_or_default()
                    <= ttl;
                if !active {
                    removed += 1;
                }
                active
            });
        }
        guard.retain(|_, (_, participants)| !participants.is_empty());
        removed
    }
    pub fn snapshot(
        &self,
        project_id: &str,
        scope: &str,
    ) -> Result<PresenceSnapshot, PresenceError> {
        let guard = self.inner.lock().unwrap();
        let (stored_scope, participants) = guard.get(project_id).ok_or(PresenceError::NotFound)?;
        if stored_scope != scope {
            return Err(PresenceError::NotFound);
        }
        Ok(snapshot(project_id, stored_scope, participants))
    }
}
fn valid(value: String) -> Result<String, PresenceError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(PresenceError::InvalidId);
    }
    Ok(value)
}
fn snapshot(
    project_id: &str,
    scope: &str,
    participants: &HashMap<String, Participant>,
) -> PresenceSnapshot {
    let mut values: Vec<_> = participants.values().cloned().collect();
    values.sort_by(|a, b| a.user_id.cmp(&b.user_id));
    PresenceSnapshot {
        project_id: project_id.into(),
        scope: scope.into(),
        participants: values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }
    #[test]
    fn joins_heartbeats_leaves_and_sorts_participants() {
        let store = MediaPresenceStore::new(2, 2, Duration::from_secs(10));
        store
            .join("project", "tenant", "user-b", PresenceRole::Editor, now(1))
            .unwrap();
        let joined = store
            .join("project", "tenant", "user-a", PresenceRole::Viewer, now(2))
            .unwrap();
        assert_eq!(joined.participants[0].user_id, "user-a");
        let heartbeat = store
            .heartbeat("project", "tenant", "user-a", now(3))
            .unwrap();
        assert_eq!(heartbeat.participants.len(), 2);
        let left = store.leave("project", "tenant", "user-b").unwrap();
        assert_eq!(left.participants.len(), 1);
    }
    #[test]
    fn isolates_scopes_bounds_capacity_and_expires() {
        let store = MediaPresenceStore::new(1, 1, Duration::from_secs(5));
        store
            .join("project", "tenant", "user", PresenceRole::Owner, now(1))
            .unwrap();
        assert_eq!(
            store.join("project", "tenant", "other", PresenceRole::Viewer, now(2)),
            Err(PresenceError::Capacity)
        );
        assert_eq!(
            store.snapshot("project", "other"),
            Err(PresenceError::NotFound)
        );
        assert_eq!(store.cleanup_expired(now(7)), 1);
        assert_eq!(
            store.snapshot("project", "tenant"),
            Err(PresenceError::NotFound)
        );
    }
}
