//! Inspectable task lifecycle and result storage for workers and clients.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    Pending,
    Running,
    Succeeded,
    Failed,
    CancelRequested,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub id: String,
    pub state: TaskState,
    pub progress_percent: u8,
    pub result: Option<Vec<u8>>,
    pub error: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResultError {
    Capacity,
    NotFound,
    InvalidState,
    NotOwner,
    InvalidError,
}

#[derive(Clone)]
pub struct TaskResultStore {
    tasks: Arc<Mutex<BTreeMap<String, TaskResult>>>,
    capacity: usize,
}

impl TaskResultStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(BTreeMap::new())),
            capacity: capacity.max(1),
        }
    }
    pub fn create(&self) -> Result<TaskResult, TaskResultError> {
        let mut tasks = self.tasks.lock().expect("task result store poisoned");
        if tasks.len() >= self.capacity {
            return Err(TaskResultError::Capacity);
        }
        let now = now();
        let task = TaskResult {
            id: Uuid::new_v4().to_string(),
            state: TaskState::Pending,
            progress_percent: 0,
            result: None,
            error: None,
            created_at: now,
            updated_at: now,
        };
        tasks.insert(task.id.clone(), task.clone());
        Ok(task)
    }
    pub fn start(&self, id: &str) -> Result<(), TaskResultError> {
        self.transition(id, |task| {
            if task.state != TaskState::Pending {
                return Err(TaskResultError::InvalidState);
            }
            task.state = TaskState::Running;
            Ok(())
        })
    }
    pub fn progress(&self, id: &str, percent: u8) -> Result<u8, TaskResultError> {
        self.transition(id, |task| {
            if !matches!(task.state, TaskState::Running | TaskState::CancelRequested) {
                return Err(TaskResultError::InvalidState);
            }
            task.progress_percent = percent.min(100);
            Ok(task.progress_percent)
        })
    }
    pub fn succeed(&self, id: &str, result: Vec<u8>) -> Result<(), TaskResultError> {
        self.transition(id, |task| {
            if !matches!(task.state, TaskState::Running) {
                return Err(TaskResultError::InvalidState);
            }
            task.progress_percent = 100;
            task.result = Some(result);
            task.state = TaskState::Succeeded;
            Ok(())
        })
    }
    pub fn fail(&self, id: &str, error: impl Into<String>) -> Result<(), TaskResultError> {
        let error = error.into();
        if error.is_empty() || error.len() > 4096 {
            return Err(TaskResultError::InvalidError);
        }
        self.transition(id, |task| {
            if !matches!(task.state, TaskState::Running | TaskState::CancelRequested) {
                return Err(TaskResultError::InvalidState);
            }
            task.error = Some(error);
            task.state = TaskState::Failed;
            Ok(())
        })
    }
    pub fn request_cancel(&self, id: &str) -> Result<(), TaskResultError> {
        self.transition(id, |task| {
            if !matches!(task.state, TaskState::Pending | TaskState::Running) {
                return Err(TaskResultError::InvalidState);
            }
            task.state = if task.state == TaskState::Pending {
                TaskState::Cancelled
            } else {
                TaskState::CancelRequested
            };
            Ok(())
        })
    }
    pub fn cancel(&self, id: &str) -> Result<(), TaskResultError> {
        self.transition(id, |task| {
            if task.state != TaskState::CancelRequested {
                return Err(TaskResultError::NotOwner);
            }
            task.state = TaskState::Cancelled;
            Ok(())
        })
    }
    pub fn get(&self, id: &str) -> Option<TaskResult> {
        self.tasks
            .lock()
            .expect("task result store poisoned")
            .get(id)
            .cloned()
    }
    pub fn cleanup_before(&self, timestamp: u64) -> usize {
        let mut tasks = self.tasks.lock().expect("task result store poisoned");
        let before = tasks.len();
        tasks.retain(|_, task| {
            task.updated_at >= timestamp
                || matches!(
                    task.state,
                    TaskState::Pending | TaskState::Running | TaskState::CancelRequested
                )
        });
        before - tasks.len()
    }
    fn transition<F, T>(&self, id: &str, update: F) -> Result<T, TaskResultError>
    where
        F: FnOnce(&mut TaskResult) -> Result<T, TaskResultError>,
    {
        let mut tasks = self.tasks.lock().expect("task result store poisoned");
        let task = tasks.get_mut(id).ok_or(TaskResultError::NotFound)?;
        let result = update(task)?;
        task.updated_at = now();
        Ok(result)
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracks_lifecycle_and_clamps_progress() {
        let store = TaskResultStore::new(2);
        let task = store.create().unwrap();
        store.start(&task.id).unwrap();
        assert_eq!(store.progress(&task.id, 240).unwrap(), 100);
        store.succeed(&task.id, b"done".to_vec()).unwrap();
        let result = store.get(&task.id).unwrap();
        assert_eq!(result.state, TaskState::Succeeded);
        assert_eq!(result.result, Some(b"done".to_vec()));
    }
    #[test]
    fn supports_cancellation_and_rejects_invalid_transitions() {
        let store = TaskResultStore::new(2);
        let task = store.create().unwrap();
        store.start(&task.id).unwrap();
        store.request_cancel(&task.id).unwrap();
        assert_eq!(
            store.get(&task.id).unwrap().state,
            TaskState::CancelRequested
        );
        store.cancel(&task.id).unwrap();
        assert_eq!(store.get(&task.id).unwrap().state, TaskState::Cancelled);
        assert_eq!(
            store.succeed(&task.id, Vec::new()),
            Err(TaskResultError::InvalidState)
        );
    }
    #[test]
    fn bounds_capacity_and_errors() {
        let store = TaskResultStore::new(1);
        let task = store.create().unwrap();
        assert_eq!(store.create(), Err(TaskResultError::Capacity));
        store.start(&task.id).unwrap();
        assert_eq!(store.fail(&task.id, ""), Err(TaskResultError::InvalidError));
    }
    #[test]
    fn cleans_only_terminal_old_records() {
        let store = TaskResultStore::new(3);
        let task = store.create().unwrap();
        store.start(&task.id).unwrap();
        store.succeed(&task.id, Vec::new()).unwrap();
        assert_eq!(store.cleanup_before(u64::MAX), 1);
    }
}
