//! Bounded queue primitives with retry and dead-letter states.
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueState {
    Queued,
    Claimed,
    Acknowledged,
    DeadLetter,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    Capacity,
    NotFound,
    InvalidState,
    InvalidRetries,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueItem {
    pub id: String,
    pub payload: Vec<u8>,
    pub attempts: u32,
    pub max_retries: u32,
    pub state: QueueState,
}
#[derive(Debug, Clone)]
pub struct Queue {
    items: BTreeMap<String, QueueItem>,
    capacity: usize,
    next_id: u64,
}
impl Queue {
    pub fn new(capacity: usize) -> Self {
        Self {
            items: BTreeMap::new(),
            capacity,
            next_id: 0,
        }
    }
    pub fn enqueue(
        &mut self,
        payload: impl Into<Vec<u8>>,
        max_retries: u32,
    ) -> Result<QueueItem, QueueError> {
        if self.items.len() >= self.capacity {
            return Err(QueueError::Capacity);
        }
        if max_retries > 32 {
            return Err(QueueError::InvalidRetries);
        }
        self.next_id += 1;
        let item = QueueItem {
            id: format!("queue-{}", self.next_id),
            payload: payload.into(),
            attempts: 0,
            max_retries,
            state: QueueState::Queued,
        };
        self.items.insert(item.id.clone(), item.clone());
        Ok(item)
    }
    pub fn claim(&mut self) -> Option<QueueItem> {
        let id = self
            .items
            .values()
            .find(|x| x.state == QueueState::Queued)
            .map(|x| x.id.clone())?;
        let item = self.items.get_mut(&id).unwrap();
        item.state = QueueState::Claimed;
        Some(item.clone())
    }
    pub fn retry(&mut self, id: &str) -> Result<QueueItem, QueueError> {
        let item = self.items.get_mut(id).ok_or(QueueError::NotFound)?;
        if item.state != QueueState::Claimed {
            return Err(QueueError::InvalidState);
        }
        item.attempts += 1;
        if item.attempts > item.max_retries {
            item.state = QueueState::DeadLetter
        } else {
            item.state = QueueState::Queued
        }
        Ok(item.clone())
    }
    pub fn acknowledge(&mut self, id: &str) -> Result<QueueItem, QueueError> {
        let item = self.items.get_mut(id).ok_or(QueueError::NotFound)?;
        if item.state != QueueState::Claimed {
            return Err(QueueError::InvalidState);
        }
        item.state = QueueState::Acknowledged;
        Ok(item.clone())
    }
    pub fn snapshot(&self) -> Vec<QueueItem> {
        self.items.values().cloned().collect()
    }
}
impl Default for Queue {
    fn default() -> Self {
        Self::new(1024)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn claims_retries_and_acknowledges() {
        let mut q = Queue::new(2);
        let i = q.enqueue(b"render".to_vec(), 1).unwrap();
        assert_eq!(q.claim().unwrap().id, i.id);
        assert_eq!(q.retry(&i.id).unwrap().state, QueueState::Queued);
        q.claim();
        assert_eq!(
            q.acknowledge(&i.id).unwrap().state,
            QueueState::Acknowledged
        )
    }
    #[test]
    fn dead_letters_and_enforces_bounds() {
        let mut q = Queue::new(1);
        let i = q.enqueue(Vec::new(), 0).unwrap();
        q.claim();
        assert_eq!(q.retry(&i.id).unwrap().state, QueueState::DeadLetter);
        assert!(q.enqueue(Vec::new(), 0).is_err());
        assert!(q.enqueue(Vec::new(), 33).is_err())
    }
}
