//! Delayed task scheduling primitives.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduledTaskState {
    Pending,
    Claimed,
    Cancelled,
    Completed,
    Misfired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub payload: Vec<u8>,
    pub run_at: u64,
    pub state: ScheduledTaskState,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    Capacity,
    NotFound,
    InvalidState,
}

#[derive(Clone)]
pub struct TaskScheduler {
    tasks: Arc<Mutex<BTreeMap<String, ScheduledTask>>>,
    capacity: usize,
    misfire_grace: Duration,
}

impl TaskScheduler {
    pub fn new(capacity: usize, misfire_grace: Duration) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(BTreeMap::new())),
            capacity,
            misfire_grace,
        }
    }

    pub fn schedule(
        &self,
        name: impl Into<String>,
        payload: impl Into<Vec<u8>>,
        run_at: u64,
    ) -> Result<ScheduledTask, ScheduleError> {
        let mut tasks = self.tasks.lock().expect("task scheduler poisoned");
        if tasks.len() >= self.capacity {
            return Err(ScheduleError::Capacity);
        }
        let task = ScheduledTask {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            payload: payload.into(),
            run_at,
            state: ScheduledTaskState::Pending,
            created_at: now(),
        };
        tasks.insert(task.id.clone(), task.clone());
        Ok(task)
    }

    pub fn claim_due(&self, now: u64) -> Vec<ScheduledTask> {
        let mut tasks = self.tasks.lock().expect("task scheduler poisoned");
        let grace = self.misfire_grace.as_secs();
        let mut claimed = Vec::new();
        for task in tasks.values_mut() {
            if !matches!(task.state, ScheduledTaskState::Pending) || task.run_at > now {
                continue;
            }
            if now.saturating_sub(task.run_at) > grace {
                task.state = ScheduledTaskState::Misfired;
                continue;
            }
            task.state = ScheduledTaskState::Claimed;
            claimed.push(task.clone());
        }
        claimed
    }

    pub fn cancel(&self, id: &str) -> Result<(), ScheduleError> {
        let mut tasks = self.tasks.lock().expect("task scheduler poisoned");
        let task = tasks.get_mut(id).ok_or(ScheduleError::NotFound)?;
        if !matches!(task.state, ScheduledTaskState::Pending) {
            return Err(ScheduleError::InvalidState);
        }
        task.state = ScheduledTaskState::Cancelled;
        Ok(())
    }

    pub fn complete(&self, id: &str) -> Result<(), ScheduleError> {
        let mut tasks = self.tasks.lock().expect("task scheduler poisoned");
        let task = tasks.get_mut(id).ok_or(ScheduleError::NotFound)?;
        if !matches!(task.state, ScheduledTaskState::Claimed) {
            return Err(ScheduleError::InvalidState);
        }
        task.state = ScheduledTaskState::Completed;
        Ok(())
    }

    pub fn cleanup(&self) -> usize {
        let mut tasks = self.tasks.lock().expect("task scheduler poisoned");
        let before = tasks.len();
        tasks.retain(|_, task| {
            matches!(
                task.state,
                ScheduledTaskState::Pending | ScheduledTaskState::Claimed
            )
        });
        before - tasks.len()
    }

    pub fn get(&self, id: &str) -> Option<ScheduledTask> {
        self.tasks
            .lock()
            .expect("task scheduler poisoned")
            .get(id)
            .cloned()
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new(10_000, Duration::from_secs(300))
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
    fn claims_due_tasks_and_completes_them() {
        let scheduler = TaskScheduler::new(10, Duration::from_secs(30));
        let task = scheduler
            .schedule("resize", b"video".to_vec(), 100)
            .unwrap();
        assert!(scheduler.claim_due(99).is_empty());
        assert_eq!(scheduler.claim_due(100)[0].id, task.id);
        scheduler.complete(&task.id).unwrap();
        assert_eq!(
            scheduler.get(&task.id).unwrap().state,
            ScheduledTaskState::Completed
        );
    }

    #[test]
    fn cancels_pending_tasks_and_rejects_capacity() {
        let scheduler = TaskScheduler::new(1, Duration::from_secs(30));
        let task = scheduler.schedule("one", Vec::new(), 100).unwrap();
        assert_eq!(
            scheduler.schedule("two", Vec::new(), 100),
            Err(ScheduleError::Capacity)
        );
        scheduler.cancel(&task.id).unwrap();
        assert_eq!(
            scheduler.get(&task.id).unwrap().state,
            ScheduledTaskState::Cancelled
        );
    }

    #[test]
    fn marks_late_tasks_as_misfired_and_cleans_terminal_state() {
        let scheduler = TaskScheduler::new(10, Duration::from_secs(10));
        let late = scheduler.schedule("late", Vec::new(), 100).unwrap();
        let done = scheduler.schedule("done", Vec::new(), 200).unwrap();
        assert!(scheduler.claim_due(111).is_empty());
        assert_eq!(
            scheduler.get(&late.id).unwrap().state,
            ScheduledTaskState::Misfired
        );
        scheduler.claim_due(200);
        scheduler.complete(&done.id).unwrap();
        assert_eq!(scheduler.cleanup(), 2);
    }
}
