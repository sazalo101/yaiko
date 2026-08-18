//! Outbound delivery scheduling with deduplication and dead-letter transitions.

use crate::{NotificationEnvelope, RetryPolicy};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryState {
    Pending,
    InFlight,
    RetryAt(u64),
    Delivered { provider_id: String },
    DeadLetter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryRecord {
    pub id: String,
    pub deduplication_key: String,
    pub envelope: NotificationEnvelope,
    pub attempts: u32,
    pub state: DeliveryState,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryError {
    Duplicate,
    NotFound,
    InvalidState,
}

#[derive(Clone, Default)]
pub struct DeliveryScheduler {
    records: Arc<Mutex<BTreeMap<String, DeliveryRecord>>>,
}

impl DeliveryScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enqueue(
        &self,
        deduplication_key: impl Into<String>,
        envelope: NotificationEnvelope,
    ) -> Result<DeliveryRecord, DeliveryError> {
        let key = deduplication_key.into();
        let mut records = self.records.lock().expect("delivery scheduler poisoned");
        if records
            .values()
            .any(|record| record.deduplication_key == key)
        {
            return Err(DeliveryError::Duplicate);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let record = DeliveryRecord {
            id: id.clone(),
            deduplication_key: key,
            envelope,
            attempts: 0,
            state: DeliveryState::Pending,
            last_error: None,
        };
        records.insert(id, record.clone());
        Ok(record)
    }

    pub fn claim_due(&self, id: &str, now: u64) -> Result<Option<DeliveryRecord>, DeliveryError> {
        let mut records = self.records.lock().expect("delivery scheduler poisoned");
        let record = records.get_mut(id).ok_or(DeliveryError::NotFound)?;
        let due = matches!(record.state, DeliveryState::Pending)
            || matches!(record.state, DeliveryState::RetryAt(at) if at <= now);
        if !due {
            return Ok(None);
        }
        record.attempts += 1;
        record.state = DeliveryState::InFlight;
        Ok(Some(record.clone()))
    }

    pub fn complete(&self, id: &str, provider_id: impl Into<String>) -> Result<(), DeliveryError> {
        let mut records = self.records.lock().expect("delivery scheduler poisoned");
        let record = records.get_mut(id).ok_or(DeliveryError::NotFound)?;
        if !matches!(record.state, DeliveryState::InFlight) {
            return Err(DeliveryError::InvalidState);
        }
        record.state = DeliveryState::Delivered {
            provider_id: provider_id.into(),
        };
        Ok(())
    }

    pub fn fail(
        &self,
        id: &str,
        reason: impl Into<String>,
        retryable: bool,
        policy: &RetryPolicy,
        now: u64,
    ) -> Result<DeliveryState, DeliveryError> {
        let mut records = self.records.lock().expect("delivery scheduler poisoned");
        let record = records.get_mut(id).ok_or(DeliveryError::NotFound)?;
        if !matches!(record.state, DeliveryState::InFlight) {
            return Err(DeliveryError::InvalidState);
        }
        let reason = reason.into();
        record.last_error = Some(reason);
        record.state = if retryable {
            policy
                .next_delay(record.attempts)
                .map(|delay| DeliveryState::RetryAt(now.saturating_add(delay.as_secs())))
                .unwrap_or(DeliveryState::DeadLetter)
        } else {
            DeliveryState::DeadLetter
        };
        Ok(record.state.clone())
    }

    pub fn get(&self, id: &str) -> Option<DeliveryRecord> {
        self.records
            .lock()
            .expect("delivery scheduler poisoned")
            .get(id)
            .cloned()
    }
    pub fn dead_letters(&self) -> Vec<DeliveryRecord> {
        self.records
            .lock()
            .expect("delivery scheduler poisoned")
            .values()
            .filter(|record| matches!(record.state, DeliveryState::DeadLetter))
            .cloned()
            .collect()
    }
}

pub fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn envelope() -> NotificationEnvelope {
        NotificationEnvelope {
            to: "user@example.com".into(),
            from: None,
            subject: "Hello".into(),
            text: "Body".into(),
            html: None,
            idempotency_key: None,
        }
    }

    #[test]
    fn deduplicates_enqueues_and_completes_claimed_delivery() {
        let scheduler = DeliveryScheduler::new();
        let record = scheduler.enqueue("event-1", envelope()).unwrap();
        assert_eq!(
            scheduler.enqueue("event-1", envelope()),
            Err(DeliveryError::Duplicate)
        );
        let claimed = scheduler
            .claim_due(&record.id, unix_seconds())
            .unwrap()
            .unwrap();
        assert_eq!(claimed.attempts, 1);
        scheduler.complete(&record.id, "provider-1").unwrap();
        assert!(matches!(
            scheduler.get(&record.id).unwrap().state,
            DeliveryState::Delivered { .. }
        ));
        assert!(scheduler
            .claim_due(&record.id, unix_seconds())
            .unwrap()
            .is_none());
    }

    #[test]
    fn schedules_retry_with_backoff_then_dead_letters() {
        let scheduler = DeliveryScheduler::new();
        let policy = RetryPolicy {
            max_attempts: 2,
            initial_delay: Duration::from_secs(10),
            max_delay: Duration::from_secs(60),
        };
        let record = scheduler.enqueue("event-2", envelope()).unwrap();
        scheduler.claim_due(&record.id, 100).unwrap();
        assert_eq!(
            scheduler
                .fail(&record.id, "timeout", true, &policy, 100)
                .unwrap(),
            DeliveryState::RetryAt(120)
        );
        assert!(scheduler.claim_due(&record.id, 109).unwrap().is_none());
        scheduler.claim_due(&record.id, 120).unwrap();
        assert_eq!(
            scheduler
                .fail(&record.id, "timeout", true, &policy, 120)
                .unwrap(),
            DeliveryState::DeadLetter
        );
        assert_eq!(scheduler.dead_letters().len(), 1);
    }

    #[test]
    fn permanent_failures_dead_letter_immediately() {
        let scheduler = DeliveryScheduler::new();
        let policy = RetryPolicy::default();
        let record = scheduler.enqueue("event-3", envelope()).unwrap();
        scheduler.claim_due(&record.id, 100).unwrap();
        assert_eq!(
            scheduler
                .fail(&record.id, "invalid recipient", false, &policy, 100)
                .unwrap(),
            DeliveryState::DeadLetter
        );
    }
}
