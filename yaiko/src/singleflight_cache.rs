//! Bounded cache with single-flight miss protection.

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheError {
    Capacity,
    Producer(String),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheLookup<V> {
    Fresh(V),
    Stale(V),
}

#[derive(Clone)]
pub struct SingleFlightCache<V> {
    state: Arc<Mutex<State<V>>>,
    capacity: usize,
}

struct Entry<V> {
    value: V,
    stale_at: Instant,
    expires_at: Instant,
}
struct State<V> {
    entries: HashMap<String, Entry<V>>,
    flights: HashMap<String, Vec<oneshot::Sender<Result<V, String>>>>,
}

impl<V: Clone + Send + Sync + 'static> SingleFlightCache<V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                entries: HashMap::new(),
                flights: HashMap::new(),
            })),
            capacity: capacity.max(1),
        }
    }
    pub async fn get(&self, key: &str) -> Option<CacheLookup<V>> {
        let now = Instant::now();
        let mut state = self.state.lock().await;
        let entry = state.entries.get(key)?;
        if now >= entry.expires_at {
            state.entries.remove(key);
            return None;
        }
        Some(if now >= entry.stale_at {
            CacheLookup::Stale(entry.value.clone())
        } else {
            CacheLookup::Fresh(entry.value.clone())
        })
    }
    pub async fn insert(
        &self,
        key: impl Into<String>,
        value: V,
        ttl: Duration,
        stale_window: Duration,
    ) -> Result<(), CacheError> {
        if ttl.is_zero() || stale_window > ttl {
            return Err(CacheError::Capacity);
        }
        let key = key.into();
        let now = Instant::now();
        let mut state = self.state.lock().await;
        if state.entries.len() >= self.capacity && !state.entries.contains_key(&key) {
            let victim = state
                .entries
                .keys()
                .next()
                .cloned()
                .ok_or(CacheError::Capacity)?;
            state.entries.remove(&victim);
        }
        state.entries.insert(
            key,
            Entry {
                value,
                stale_at: now + ttl.saturating_sub(stale_window),
                expires_at: now + ttl,
            },
        );
        Ok(())
    }
    pub async fn invalidate(&self, key: &str) {
        self.state.lock().await.entries.remove(key);
    }
    pub async fn len(&self) -> usize {
        self.state.lock().await.entries.len()
    }
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
    pub async fn get_or_compute<F, Fut>(
        &self,
        key: impl Into<String>,
        ttl: Duration,
        stale_window: Duration,
        producer: F,
    ) -> Result<V, CacheError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<V, String>> + Send,
    {
        let key = key.into();
        if let Some(CacheLookup::Fresh(value) | CacheLookup::Stale(value)) = self.get(&key).await {
            return Ok(value);
        }
        let receiver = {
            let mut state = self.state.lock().await;
            if let Some(waiters) = state.flights.get_mut(&key) {
                let (sender, receiver) = oneshot::channel();
                waiters.push(sender);
                Some(receiver)
            } else {
                state.flights.insert(key.clone(), Vec::new());
                None
            }
        };
        if let Some(receiver) = receiver {
            return receiver
                .await
                .map_err(|_| CacheError::Cancelled)?
                .map_err(CacheError::Producer);
        }
        let result = producer().await;
        let waiters = {
            let mut state = self.state.lock().await;
            if let Ok(value) = &result {
                if !ttl.is_zero() && stale_window <= ttl {
                    if state.entries.len() >= self.capacity && !state.entries.contains_key(&key) {
                        if let Some(victim) = state.entries.keys().next().cloned() {
                            state.entries.remove(&victim);
                        }
                    }
                    state.entries.insert(
                        key.clone(),
                        Entry {
                            value: value.clone(),
                            stale_at: Instant::now() + ttl.saturating_sub(stale_window),
                            expires_at: Instant::now() + ttl,
                        },
                    );
                }
            }
            state.flights.remove(&key).unwrap_or_default()
        };
        for waiter in waiters {
            let _ = waiter.send(result.clone());
        }
        result.map_err(CacheError::Producer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[tokio::test]
    async fn caches_fresh_and_stale_values() {
        let cache = SingleFlightCache::new(2);
        cache
            .insert("a", 1, Duration::from_secs(1), Duration::from_millis(900))
            .await
            .unwrap();
        assert_eq!(cache.get("a").await, Some(CacheLookup::Fresh(1)));
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(matches!(cache.get("a").await, Some(CacheLookup::Stale(1))));
    }
    #[tokio::test]
    async fn deduplicates_concurrent_misses() {
        let cache = Arc::new(SingleFlightCache::new(2));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let calls = calls.clone();
            tasks.push(tokio::spawn(async move {
                cache
                    .get_or_compute("key", Duration::from_secs(1), Duration::ZERO, || async {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok::<_, String>(42)
                    })
                    .await
                    .unwrap()
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap(), 42);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn expires_and_invalidates_entries() {
        let cache = SingleFlightCache::new(1);
        cache
            .insert("a", "x", Duration::from_millis(10), Duration::ZERO)
            .await
            .unwrap();
        cache.invalidate("a").await;
        assert!(cache.is_empty().await);
    }
    #[tokio::test]
    async fn bounds_capacity() {
        let cache = SingleFlightCache::new(1);
        cache
            .insert("a", 1, Duration::from_secs(1), Duration::ZERO)
            .await
            .unwrap();
        cache
            .insert("b", 2, Duration::from_secs(1), Duration::ZERO)
            .await
            .unwrap();
        assert_eq!(cache.len().await, 1);
    }
}
