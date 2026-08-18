//! Async rate limiting primitives suitable for local or distributed backends.
//!
//! A production Redis or database adapter can implement `RateLimitStore` using
//! an atomic increment-and-expiry operation. The included memory backend is
//! deterministic and bounded for tests and single-process deployments.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributedRateLimitError {
    Backend,
    InvalidPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPolicy {
    pub limit: u32,
    pub window: Duration,
}

impl WindowPolicy {
    pub fn new(limit: u32, window: Duration) -> Result<Self, DistributedRateLimitError> {
        if limit == 0 || window.is_zero() {
            return Err(DistributedRateLimitError::InvalidPolicy);
        }
        Ok(Self { limit, window })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistributedDecision {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub retry_after: Duration,
}

#[async_trait]
pub trait RateLimitStore: Clone + Send + Sync + 'static {
    async fn increment(
        &self,
        key: &str,
        policy: WindowPolicy,
    ) -> Result<DistributedDecision, DistributedRateLimitError>;
}

#[derive(Clone)]
pub struct DistributedRateLimiter<S> {
    store: S,
    policy: WindowPolicy,
}

impl<S: RateLimitStore> DistributedRateLimiter<S> {
    pub fn new(store: S, policy: WindowPolicy) -> Self {
        Self { store, policy }
    }
    pub async fn check(
        &self,
        key: impl AsRef<str>,
    ) -> Result<DistributedDecision, DistributedRateLimitError> {
        self.store.increment(key.as_ref(), self.policy).await
    }
    pub fn policy(&self) -> WindowPolicy {
        self.policy
    }
}

#[derive(Debug, Clone)]
struct Window {
    count: u32,
    reset_at: Instant,
}

#[derive(Clone)]
pub struct MemoryRateLimitStore {
    entries: Arc<Mutex<HashMap<String, Window>>>,
    max_entries: usize,
}

impl MemoryRateLimitStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            max_entries: max_entries.max(1),
        }
    }
    pub fn len(&self) -> usize {
        self.entries
            .lock()
            .expect("rate limit store lock poisoned")
            .len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&self) {
        self.entries
            .lock()
            .expect("rate limit store lock poisoned")
            .clear();
    }
}

#[async_trait]
impl RateLimitStore for MemoryRateLimitStore {
    async fn increment(
        &self,
        key: &str,
        policy: WindowPolicy,
    ) -> Result<DistributedDecision, DistributedRateLimitError> {
        let now = Instant::now();
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| DistributedRateLimitError::Backend)?;
        if entries.len() >= self.max_entries && !entries.contains_key(key) {
            if let Some(oldest) = entries
                .iter()
                .min_by_key(|(_, value)| value.reset_at)
                .map(|(key, _)| key.clone())
            {
                entries.remove(&oldest);
            }
        }
        let window = entries.entry(key.to_owned()).or_insert(Window {
            count: 0,
            reset_at: now + policy.window,
        });
        if now >= window.reset_at {
            window.count = 0;
            window.reset_at = now + policy.window;
        }
        window.count = window.count.saturating_add(1);
        let allowed = window.count <= policy.limit;
        let remaining = policy.limit.saturating_sub(window.count.min(policy.limit));
        Ok(DistributedDecision {
            allowed,
            limit: policy.limit,
            remaining,
            retry_after: if allowed {
                Duration::ZERO
            } else {
                window.reset_at.saturating_duration_since(now)
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn atomically_enforces_a_window() {
        let store = MemoryRateLimitStore::new(10);
        let policy = WindowPolicy::new(2, Duration::from_secs(60)).unwrap();
        let limiter = Arc::new(DistributedRateLimiter::new(store, policy));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let limiter = limiter.clone();
            tasks.push(tokio::spawn(async move {
                limiter.check("same-key").await.unwrap().allowed
            }));
        }
        let mut allowed = 0;
        for task in tasks {
            if task.await.unwrap() {
                allowed += 1;
            }
        }
        assert_eq!(allowed, 2);
    }
    #[tokio::test]
    async fn isolates_keys_and_bounds_storage() {
        let store = MemoryRateLimitStore::new(2);
        let policy = WindowPolicy::new(1, Duration::from_secs(60)).unwrap();
        let limiter = DistributedRateLimiter::new(store.clone(), policy);
        assert!(limiter.check("a").await.unwrap().allowed);
        assert!(limiter.check("b").await.unwrap().allowed);
        assert_eq!(store.len(), 2);
        assert!(limiter.check("c").await.unwrap().allowed);
        assert_eq!(store.len(), 2);
    }
    #[tokio::test]
    async fn reports_retry_after_when_denied() {
        let store = MemoryRateLimitStore::new(2);
        let limiter = DistributedRateLimiter::new(
            store,
            WindowPolicy::new(1, Duration::from_secs(60)).unwrap(),
        );
        assert!(limiter.check("key").await.unwrap().allowed);
        let denied = limiter.check("key").await.unwrap();
        assert!(!denied.allowed);
        assert!(denied.retry_after > Duration::ZERO);
        assert_eq!(denied.remaining, 0);
    }
}
