//! Bounded worker task state machine.
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    Capacity,
    NotFound,
    InvalidState,
    InvalidRetries,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerTask {
    pub id: String,
    pub name: String,
    pub attempts: u32,
    pub max_retries: u32,
    pub state: WorkerState,
}
#[derive(Debug, Clone)]
pub struct WorkerPool {
    tasks: BTreeMap<String, WorkerTask>,
    capacity: usize,
    next_id: u64,
}
impl WorkerPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            tasks: BTreeMap::new(),
            capacity,
            next_id: 0,
        }
    }
    pub fn submit(
        &mut self,
        name: impl Into<String>,
        max_retries: u32,
    ) -> Result<WorkerTask, WorkerError> {
        if self.tasks.len() >= self.capacity {
            return Err(WorkerError::Capacity);
        }
        if max_retries > 32 {
            return Err(WorkerError::InvalidRetries);
        }
        self.next_id += 1;
        let t = WorkerTask {
            id: format!("worker-{}", self.next_id),
            name: name.into(),
            attempts: 0,
            max_retries,
            state: WorkerState::Queued,
        };
        self.tasks.insert(t.id.clone(), t.clone());
        Ok(t)
    }
    pub fn start(&mut self, id: &str) -> Result<WorkerTask, WorkerError> {
        let t = self.tasks.get_mut(id).ok_or(WorkerError::NotFound)?;
        if t.state != WorkerState::Queued {
            return Err(WorkerError::InvalidState);
        }
        t.state = WorkerState::Running;
        Ok(t.clone())
    }
    pub fn succeed(&mut self, id: &str) -> Result<WorkerTask, WorkerError> {
        let t = self.tasks.get_mut(id).ok_or(WorkerError::NotFound)?;
        if t.state != WorkerState::Running {
            return Err(WorkerError::InvalidState);
        }
        t.state = WorkerState::Succeeded;
        Ok(t.clone())
    }
    pub fn fail(&mut self, id: &str) -> Result<WorkerTask, WorkerError> {
        let t = self.tasks.get_mut(id).ok_or(WorkerError::NotFound)?;
        if t.state != WorkerState::Running {
            return Err(WorkerError::InvalidState);
        }
        t.attempts += 1;
        if t.attempts <= t.max_retries {
            t.state = WorkerState::Queued
        } else {
            t.state = WorkerState::Failed
        }
        Ok(t.clone())
    }
    pub fn cancel(&mut self, id: &str) -> Result<WorkerTask, WorkerError> {
        let t = self.tasks.get_mut(id).ok_or(WorkerError::NotFound)?;
        if matches!(
            t.state,
            WorkerState::Succeeded | WorkerState::Failed | WorkerState::Cancelled
        ) {
            return Err(WorkerError::InvalidState);
        }
        t.state = WorkerState::Cancelled;
        Ok(t.clone())
    }
    pub fn snapshot(&self) -> Vec<WorkerTask> {
        self.tasks.values().cloned().collect()
    }
}
impl Default for WorkerPool {
    fn default() -> Self {
        Self::new(1024)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_then_succeeds() {
        let mut p = WorkerPool::new(2);
        let t = p.submit("render", 1).unwrap();
        p.start(&t.id).unwrap();
        assert_eq!(p.fail(&t.id).unwrap().state, WorkerState::Queued);
        p.start(&t.id).unwrap();
        assert_eq!(p.succeed(&t.id).unwrap().state, WorkerState::Succeeded)
    }
    #[test]
    fn enforces_capacity_cancellation_and_retry_bounds() {
        let mut p = WorkerPool::new(1);
        let t = p.submit("x", 0).unwrap();
        assert_eq!(p.submit("y", 0), Err(WorkerError::Capacity));
        assert!(p.submit("z", 33).is_err());
        assert_eq!(p.cancel(&t.id).unwrap().state, WorkerState::Cancelled);
    }
}
