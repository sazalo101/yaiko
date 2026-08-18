//! Bounded progress events for observable media-processing jobs.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressError {
    InvalidTask,
    InvalidStage,
    InvalidProgress,
    SequenceMismatch,
    NonMonotonic,
    Capacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaProgressEvent {
    pub task_id: String,
    pub sequence: u64,
    pub progress: u8,
    pub stage: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressSnapshot {
    pub latest: Option<MediaProgressEvent>,
    pub history: Vec<MediaProgressEvent>,
}

#[derive(Debug, Clone)]
struct TaskProgress {
    events: VecDeque<MediaProgressEvent>,
    latest_progress: u8,
    next_sequence: u64,
}

#[derive(Debug, Clone)]
pub struct MediaProgressStore {
    inner: Arc<Mutex<HashMap<String, TaskProgress>>>,
    max_tasks: usize,
    history_limit: usize,
}

impl MediaProgressStore {
    pub fn new(max_tasks: usize, history_limit: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_tasks: max_tasks.max(1),
            history_limit: history_limit.max(1),
        }
    }
    pub fn publish(
        &self,
        task_id: impl Into<String>,
        progress: u8,
        stage: impl Into<String>,
        detail: Option<String>,
    ) -> Result<MediaProgressEvent, ProgressError> {
        let task_id = validate_task(task_id.into())?;
        let stage = validate_stage(stage.into())?;
        if progress > 100 {
            return Err(ProgressError::InvalidProgress);
        }
        let mut guard = self.inner.lock().unwrap();
        if !guard.contains_key(&task_id) && guard.len() >= self.max_tasks {
            return Err(ProgressError::Capacity);
        }
        let task = guard
            .entry(task_id.clone())
            .or_insert_with(|| TaskProgress {
                events: VecDeque::new(),
                latest_progress: 0,
                next_sequence: 0,
            });
        if progress < task.latest_progress {
            return Err(ProgressError::NonMonotonic);
        }
        let event = MediaProgressEvent {
            task_id,
            sequence: task.next_sequence,
            progress,
            stage,
            detail,
        };
        task.next_sequence = task
            .next_sequence
            .checked_add(1)
            .ok_or(ProgressError::SequenceMismatch)?;
        task.latest_progress = progress;
        task.events.push_back(event.clone());
        while task.events.len() > self.history_limit {
            task.events.pop_front();
        }
        Ok(event)
    }
    pub fn snapshot(&self, task_id: &str) -> Result<ProgressSnapshot, ProgressError> {
        let task_id = validate_task(task_id.to_string())?;
        let guard = self.inner.lock().unwrap();
        let task = guard.get(&task_id);
        Ok(ProgressSnapshot {
            latest: task.and_then(|state| state.events.back().cloned()),
            history: task
                .map(|state| state.events.iter().cloned().collect())
                .unwrap_or_default(),
        })
    }
    pub fn remove(&self, task_id: &str) -> Result<(), ProgressError> {
        let task_id = validate_task(task_id.to_string())?;
        self.inner
            .lock()
            .unwrap()
            .remove(&task_id)
            .map(|_| ())
            .ok_or(ProgressError::InvalidTask)
    }
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn validate_task(task: String) -> Result<String, ProgressError> {
    if task.is_empty()
        || task.len() > 128
        || task
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
    {
        return Err(ProgressError::InvalidTask);
    }
    Ok(task)
}
fn validate_stage(stage: String) -> Result<String, ProgressError> {
    if stage.is_empty() || stage.len() > 128 || stage.chars().any(|c| c.is_control()) {
        return Err(ProgressError::InvalidStage);
    }
    Ok(stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publishes_monotonic_events_and_bounded_history() {
        let store = MediaProgressStore::new(2, 2);
        let first = store.publish("task-1", 0, "queued", None).unwrap();
        let second = store
            .publish("task-1", 50, "encoding", Some("half".into()))
            .unwrap();
        let third = store.publish("task-1", 100, "complete", None).unwrap();
        assert_eq!((first.sequence, second.sequence, third.sequence), (0, 1, 2));
        let snapshot = store.snapshot("task-1").unwrap();
        assert_eq!(snapshot.latest, Some(third));
        assert_eq!(snapshot.history.len(), 2);
        assert_eq!(snapshot.history[0].progress, 50);
    }
    #[test]
    fn rejects_invalid_and_decreasing_events() {
        let store = MediaProgressStore::new(2, 2);
        assert_eq!(
            store.publish("task-1", 101, "queued", None),
            Err(ProgressError::InvalidProgress)
        );
        assert_eq!(
            store.publish("task-1", 1, "", None),
            Err(ProgressError::InvalidStage)
        );
        store.publish("task-1", 50, "encoding", None).unwrap();
        assert_eq!(
            store.publish("task-1", 49, "encoding", None),
            Err(ProgressError::NonMonotonic)
        );
    }
    #[test]
    fn isolates_tasks_and_bounds_task_capacity() {
        let store = MediaProgressStore::new(1, 2);
        store.publish("task-1", 1, "queued", None).unwrap();
        assert_eq!(
            store.publish("task-2", 1, "queued", None),
            Err(ProgressError::Capacity)
        );
        assert!(store.snapshot("task-2").unwrap().history.is_empty());
        store.remove("task-1").unwrap();
        assert!(store.is_empty());
    }
}
