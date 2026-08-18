//! Cache stampede protection and request coalescing primitives.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedValue {
    pub value: Vec<u8>,
    pub stale_after: u64,
    pub expires_at: u64,
}

impl CachedValue {
    pub fn is_stale(&self, now: u64) -> bool {
        now >= self.stale_after && now < self.expires_at
    }
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum CoalesceError {
    AlreadyOwned,
    TooManyWaiters,
    Missing,
    ProducerFailed(String),
}

struct Flight {
    waiters: usize,
    subscribers: Vec<oneshot::Sender<Result<Vec<u8>, String>>>,
}

#[derive(Clone)]
pub struct RequestCoalescer {
    flights: Arc<Mutex<BTreeMap<String, Flight>>>,
    max_waiters: usize,
}

impl RequestCoalescer {
    pub fn new(max_waiters: usize) -> Self {
        Self {
            flights: Arc::new(Mutex::new(BTreeMap::new())),
            max_waiters,
        }
    }

    pub fn begin(&self, key: impl Into<String>) -> Result<FlightOwner, CoalesceError> {
        let key = key.into();
        let mut flights = self.flights.lock().expect("coalescer poisoned");
        if flights.contains_key(&key) {
            return Err(CoalesceError::AlreadyOwned);
        }
        flights.insert(
            key.clone(),
            Flight {
                waiters: 0,
                subscribers: Vec::new(),
            },
        );
        Ok(FlightOwner {
            key,
            coalescer: self.clone(),
        })
    }

    pub fn wait(
        &self,
        key: &str,
    ) -> Result<oneshot::Receiver<Result<Vec<u8>, String>>, CoalesceError> {
        let mut flights = self.flights.lock().expect("coalescer poisoned");
        let flight = flights.get_mut(key).ok_or(CoalesceError::Missing)?;
        if flight.waiters >= self.max_waiters {
            return Err(CoalesceError::TooManyWaiters);
        }
        flight.waiters += 1;
        let (sender, receiver) = oneshot::channel();
        flight.subscribers.push(sender);
        Ok(receiver)
    }

    fn finish(&self, key: &str, result: Result<Vec<u8>, String>) {
        if let Some(flight) = self.flights.lock().expect("coalescer poisoned").remove(key) {
            for subscriber in flight.subscribers {
                let _ = subscriber.send(result.clone());
            }
        }
    }
}

pub struct FlightOwner {
    key: String,
    coalescer: RequestCoalescer,
}

impl FlightOwner {
    pub fn complete(self, value: Vec<u8>) {
        self.coalescer.finish(&self.key, Ok(value));
    }
    pub fn fail(self, reason: impl Into<String>) {
        self.coalescer.finish(&self.key, Err(reason.into()));
    }
}

pub fn stale_window(stale_after: u64, expires_at: u64) -> Duration {
    Duration::from_secs(expires_at.saturating_sub(stale_after))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn coalesces_waiters_and_propagates_success() {
        let coalescer = RequestCoalescer::new(2);
        let owner = coalescer.begin("key").unwrap();
        let first = coalescer.wait("key").unwrap();
        let second = coalescer.wait("key").unwrap();
        owner.complete(b"value".to_vec());
        assert_eq!(first.await.unwrap().unwrap(), b"value");
        assert_eq!(second.await.unwrap().unwrap(), b"value");
    }

    #[tokio::test]
    async fn bounds_waiters_and_propagates_failure() {
        let coalescer = RequestCoalescer::new(1);
        let owner = coalescer.begin("key").unwrap();
        let receiver = coalescer.wait("key").unwrap();
        assert!(matches!(
            coalescer.wait("key"),
            Err(CoalesceError::TooManyWaiters)
        ));
        owner.fail("backend unavailable");
        assert_eq!(receiver.await.unwrap(), Err("backend unavailable".into()));
    }

    #[test]
    fn classifies_stale_and_expired_values() {
        let value = CachedValue {
            value: b"x".to_vec(),
            stale_after: 10,
            expires_at: 20,
        };
        assert!(value.is_stale(15));
        assert!(value.is_expired(20));
        assert_eq!(stale_window(10, 20), Duration::from_secs(10));
    }
}
