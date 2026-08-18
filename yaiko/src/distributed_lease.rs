//! Ownership-safe distributed lease primitives for workers and schedulers.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseError {
    Backend,
    InvalidKey,
    InvalidDuration,
    Contended,
    NotOwner,
    Capacity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    pub key: String,
    pub owner: String,
    pub expires_in: Duration,
}

#[async_trait]
pub trait LeaseStore: Clone + Send + Sync + 'static {
    async fn acquire(&self, key: &str, ttl: Duration) -> Result<Lease, LeaseError>;
    async fn renew(&self, lease: &Lease, ttl: Duration) -> Result<Lease, LeaseError>;
    async fn release(&self, lease: &Lease) -> Result<(), LeaseError>;
}

#[derive(Clone)]
pub struct DistributedLease<S> {
    store: S,
}

impl<S: LeaseStore> DistributedLease<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
    pub async fn acquire(&self, key: impl AsRef<str>, ttl: Duration) -> Result<Lease, LeaseError> {
        self.store.acquire(key.as_ref(), ttl).await
    }
    pub async fn renew(&self, lease: &Lease, ttl: Duration) -> Result<Lease, LeaseError> {
        self.store.renew(lease, ttl).await
    }
    pub async fn release(&self, lease: &Lease) -> Result<(), LeaseError> {
        self.store.release(lease).await
    }
}

#[derive(Debug, Clone)]
struct Entry {
    owner: String,
    expires_at: Instant,
}
#[derive(Clone)]
pub struct MemoryLeaseStore {
    entries: Arc<Mutex<HashMap<String, Entry>>>,
    max_entries: usize,
}

impl MemoryLeaseStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            max_entries: max_entries.max(1),
        }
    }
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .expect("lease store lock poisoned")
            .len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&self) {
        self.entries
            .lock()
            .expect("lease store lock poisoned")
            .clear();
    }
}

fn validate(key: &str, ttl: Duration) -> Result<(), LeaseError> {
    if key.is_empty() || key.len() > 256 || key.chars().any(char::is_control) {
        return Err(LeaseError::InvalidKey);
    }
    if ttl.is_zero() {
        return Err(LeaseError::InvalidDuration);
    }
    Ok(())
}

#[async_trait]
impl LeaseStore for MemoryLeaseStore {
    async fn acquire(&self, key: &str, ttl: Duration) -> Result<Lease, LeaseError> {
        validate(key, ttl)?;
        let now = Instant::now();
        let mut entries = self.entries.lock().map_err(|_| LeaseError::Backend)?;
        if let Some(entry) = entries.get(key) {
            if entry.expires_at > now {
                return Err(LeaseError::Contended);
            }
        }
        if !entries.contains_key(key) && entries.len() >= self.max_entries {
            return Err(LeaseError::Capacity);
        }
        let owner = Uuid::new_v4().to_string();
        entries.insert(
            key.to_owned(),
            Entry {
                owner: owner.clone(),
                expires_at: now + ttl,
            },
        );
        Ok(Lease {
            key: key.to_owned(),
            owner,
            expires_in: ttl,
        })
    }
    async fn renew(&self, lease: &Lease, ttl: Duration) -> Result<Lease, LeaseError> {
        validate(&lease.key, ttl)?;
        let now = Instant::now();
        let mut entries = self.entries.lock().map_err(|_| LeaseError::Backend)?;
        let Some(entry) = entries.get_mut(&lease.key) else {
            return Err(LeaseError::NotOwner);
        };
        if entry.owner != lease.owner || entry.expires_at <= now {
            return Err(LeaseError::NotOwner);
        }
        entry.expires_at = now + ttl;
        Ok(Lease {
            key: lease.key.clone(),
            owner: lease.owner.clone(),
            expires_in: ttl,
        })
    }
    async fn release(&self, lease: &Lease) -> Result<(), LeaseError> {
        let mut entries = self.entries.lock().map_err(|_| LeaseError::Backend)?;
        let Some(entry) = entries.get(&lease.key) else {
            return Err(LeaseError::NotOwner);
        };
        if entry.owner != lease.owner || entry.expires_at <= Instant::now() {
            return Err(LeaseError::NotOwner);
        }
        entries.remove(&lease.key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn contends_and_enforces_owner() {
        let store = MemoryLeaseStore::new(2);
        let lease = store
            .acquire("render:1", Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(
            store.acquire("render:1", Duration::from_secs(60)).await,
            Err(LeaseError::Contended)
        );
        let forged = Lease {
            key: lease.key.clone(),
            owner: "forged".into(),
            expires_in: lease.expires_in,
        };
        assert_eq!(store.release(&forged).await, Err(LeaseError::NotOwner));
        store.release(&lease).await.unwrap();
        assert!(store
            .acquire("render:1", Duration::from_secs(60))
            .await
            .is_ok());
    }
    #[tokio::test]
    async fn renews_only_live_owner() {
        let store = MemoryLeaseStore::new(2);
        let lease = store
            .acquire("job", Duration::from_millis(20))
            .await
            .unwrap();
        let renewed = store.renew(&lease, Duration::from_secs(1)).await.unwrap();
        assert_eq!(renewed.owner, lease.owner);
        tokio::time::sleep(Duration::from_millis(25)).await;
        assert!(store.renew(&renewed, Duration::from_secs(1)).await.is_ok());
    }
    #[tokio::test]
    async fn expires_and_reclaims_key() {
        let store = MemoryLeaseStore::new(1);
        let lease = store
            .acquire("job", Duration::from_millis(10))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(store.acquire("job", Duration::from_secs(1)).await.is_ok());
        assert_eq!(store.release(&lease).await, Err(LeaseError::NotOwner));
    }
    #[tokio::test]
    async fn enforces_capacity_and_validates_inputs() {
        let store = MemoryLeaseStore::new(1);
        assert_eq!(
            store.acquire("", Duration::from_secs(1)).await,
            Err(LeaseError::InvalidKey)
        );
        assert_eq!(
            store.acquire("a", Duration::ZERO).await,
            Err(LeaseError::InvalidDuration)
        );
        store.acquire("a", Duration::from_secs(1)).await.unwrap();
        assert_eq!(
            store.acquire("b", Duration::from_secs(1)).await,
            Err(LeaseError::Capacity)
        );
    }
}
