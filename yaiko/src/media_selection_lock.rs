//! Scoped timeline-region selection locks for collaborative media editing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionLockError {
    InvalidId,
    InvalidRange,
    NotFound,
    ScopeMismatch,
    Conflict,
    Ownership,
    Expired,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionLock {
    pub id: String,
    pub project_id: String,
    pub scope: String,
    pub owner: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub token: String,
    pub expires_at: SystemTime,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionLockSnapshot {
    pub project_id: String,
    pub scope: String,
    pub locks: Vec<SelectionLock>,
}
#[derive(Debug, Clone)]
pub struct MediaSelectionLockStore {
    inner: Arc<Mutex<HashMap<String, Vec<SelectionLock>>>>,
    max_locks_per_project: usize,
    ttl: Duration,
}
impl MediaSelectionLockStore {
    pub fn new(max_locks_per_project: usize, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_locks_per_project: max_locks_per_project.max(1),
            ttl,
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn acquire(
        &self,
        project_id: impl Into<String>,
        scope: impl Into<String>,
        owner: impl Into<String>,
        id: impl Into<String>,
        start_ms: u64,
        end_ms: u64,
        token: impl Into<String>,
        now: SystemTime,
    ) -> Result<SelectionLockSnapshot, SelectionLockError> {
        let project_id = valid(project_id.into())?;
        let scope = valid(scope.into())?;
        let owner = valid(owner.into())?;
        let id = valid(id.into())?;
        let token = valid(token.into())?;
        validate_range(start_ms, end_ms)?;
        let mut guard = self.inner.lock().unwrap();
        let locks = guard.entry(project_id.clone()).or_default();
        locks.retain(|lock| lock.expires_at > now);
        if locks.len() >= self.max_locks_per_project {
            return Err(SelectionLockError::Capacity);
        }
        if locks
            .iter()
            .any(|lock| lock.scope == scope && lock.start_ms < end_ms && start_ms < lock.end_ms)
        {
            return Err(SelectionLockError::Conflict);
        }
        locks.push(SelectionLock {
            id,
            project_id: project_id.clone(),
            scope: scope.clone(),
            owner,
            start_ms,
            end_ms,
            token,
            expires_at: now + self.ttl,
        });
        Ok(snapshot(&project_id, &scope, locks))
    }
    pub fn renew(
        &self,
        project_id: &str,
        scope: &str,
        id: &str,
        token: &str,
        now: SystemTime,
    ) -> Result<SelectionLockSnapshot, SelectionLockError> {
        let mut guard = self.inner.lock().unwrap();
        let locks = guard
            .get_mut(project_id)
            .ok_or(SelectionLockError::NotFound)?;
        let lock = locks
            .iter_mut()
            .find(|lock| lock.id == id)
            .ok_or(SelectionLockError::NotFound)?;
        if lock.scope != scope {
            return Err(SelectionLockError::ScopeMismatch);
        }
        if lock.token != token {
            return Err(SelectionLockError::Ownership);
        }
        if lock.expires_at <= now {
            return Err(SelectionLockError::Expired);
        }
        lock.expires_at = now + self.ttl;
        Ok(snapshot(project_id, scope, locks))
    }
    pub fn release(
        &self,
        project_id: &str,
        scope: &str,
        id: &str,
        token: &str,
    ) -> Result<SelectionLockSnapshot, SelectionLockError> {
        let mut guard = self.inner.lock().unwrap();
        let locks = guard
            .get_mut(project_id)
            .ok_or(SelectionLockError::NotFound)?;
        let index = locks
            .iter()
            .position(|lock| lock.id == id)
            .ok_or(SelectionLockError::NotFound)?;
        if locks[index].scope != scope {
            return Err(SelectionLockError::ScopeMismatch);
        }
        if locks[index].token != token {
            return Err(SelectionLockError::Ownership);
        }
        locks.remove(index);
        let result = snapshot(project_id, scope, locks);
        if locks.is_empty() {
            guard.remove(project_id);
        }
        Ok(result)
    }
    pub fn cleanup_expired(&self, now: SystemTime) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let mut removed = 0;
        for locks in guard.values_mut() {
            let before = locks.len();
            locks.retain(|lock| lock.expires_at > now);
            removed += before - locks.len();
        }
        guard.retain(|_, locks| !locks.is_empty());
        removed
    }
}
fn valid(value: String) -> Result<String, SelectionLockError> {
    if value.is_empty()
        || value.len() > 128
        || value
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(SelectionLockError::InvalidId);
    }
    Ok(value)
}
fn validate_range(start_ms: u64, end_ms: u64) -> Result<(), SelectionLockError> {
    if end_ms <= start_ms {
        return Err(SelectionLockError::InvalidRange);
    }
    Ok(())
}
fn snapshot(project_id: &str, scope: &str, locks: &[SelectionLock]) -> SelectionLockSnapshot {
    let mut locks = locks.to_vec();
    locks.sort_by(|a, b| a.start_ms.cmp(&b.start_ms).then_with(|| a.id.cmp(&b.id)));
    SelectionLockSnapshot {
        project_id: project_id.into(),
        scope: scope.into(),
        locks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now(seconds: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
    }
    #[test]
    fn acquires_renews_releases_and_orders_locks() {
        let store = MediaSelectionLockStore::new(3, Duration::from_secs(10));
        let first = store
            .acquire(
                "project",
                "tenant",
                "user-a",
                "lock-a",
                100,
                200,
                "token-a",
                now(1),
            )
            .unwrap();
        let second = store
            .acquire(
                "project",
                "tenant",
                "user-b",
                "lock-b",
                0,
                50,
                "token-b",
                now(1),
            )
            .unwrap();
        assert_eq!(second.locks[0].id, "lock-b");
        let renewed = store
            .renew("project", "tenant", "lock-a", "token-a", now(2))
            .unwrap();
        assert!(renewed.locks.iter().any(|lock| lock.expires_at == now(12)));
        let released = store
            .release("project", "tenant", "lock-b", "token-b")
            .unwrap();
        assert_eq!(released.locks.len(), 1);
        assert_eq!(first.locks.len(), 1);
    }
    #[test]
    fn rejects_overlap_ownership_scope_expiry_and_invalid_ranges() {
        let store = MediaSelectionLockStore::new(2, Duration::from_secs(5));
        store
            .acquire(
                "project",
                "tenant",
                "user",
                "lock-a",
                0,
                100,
                "token-a",
                now(1),
            )
            .unwrap();
        assert_eq!(
            store.acquire(
                "project",
                "tenant",
                "other",
                "lock-b",
                50,
                150,
                "token-b",
                now(1)
            ),
            Err(SelectionLockError::Conflict)
        );
        assert_eq!(
            store.renew("project", "tenant", "lock-a", "bad", now(2)),
            Err(SelectionLockError::Ownership)
        );
        assert_eq!(
            store.renew("project", "other", "lock-a", "token-a", now(2)),
            Err(SelectionLockError::ScopeMismatch)
        );
        assert_eq!(
            store.renew("project", "tenant", "lock-a", "token-a", now(7)),
            Err(SelectionLockError::Expired)
        );
        assert_eq!(
            store.acquire("project", "tenant", "user", "bad", 10, 10, "token", now(1)),
            Err(SelectionLockError::InvalidRange)
        );
    }
}
