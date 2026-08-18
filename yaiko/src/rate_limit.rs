//! Provider-neutral token-bucket rate limiting primitives.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuotaPolicy {
    pub capacity: u32,
    pub refill_per_second: f64,
}

impl QuotaPolicy {
    pub fn new(capacity: u32, refill_per_second: f64) -> Self {
        Self {
            capacity: capacity.max(1),
            refill_per_second: refill_per_second.max(0.000_001),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RateLimitDecision {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub retry_after: Option<Duration>,
}

impl RateLimitDecision {
    pub fn headers(&self) -> BTreeMap<String, String> {
        let mut headers = BTreeMap::from([
            ("X-RateLimit-Limit".to_string(), self.limit.to_string()),
            (
                "X-RateLimit-Remaining".to_string(),
                self.remaining.to_string(),
            ),
        ]);
        if let Some(retry_after) = self.retry_after {
            headers.insert(
                "Retry-After".to_string(),
                retry_after.as_secs().max(1).to_string(),
            );
        }
        headers
    }
}

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

#[derive(Clone)]
pub struct MemoryRateLimiter {
    policy: QuotaPolicy,
    buckets: Arc<Mutex<HashMap<String, Bucket>>>,
}

impl MemoryRateLimiter {
    pub fn new(policy: QuotaPolicy) -> Self {
        Self {
            policy,
            buckets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn policy(&self) -> QuotaPolicy {
        self.policy
    }

    pub fn check(&self, key: impl Into<String>, cost: u32) -> RateLimitDecision {
        self.check_at(key, cost, Instant::now())
    }

    pub fn check_at(&self, key: impl Into<String>, cost: u32, now: Instant) -> RateLimitDecision {
        let cost = cost.max(1) as f64;
        let mut buckets = self.buckets.lock().expect("rate limiter lock poisoned");
        let bucket = buckets.entry(key.into()).or_insert(Bucket {
            tokens: self.policy.capacity as f64,
            last: now,
        });
        let elapsed = now.saturating_duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.policy.refill_per_second)
            .min(self.policy.capacity as f64);
        bucket.last = now;
        let allowed = bucket.tokens >= cost;
        if allowed {
            bucket.tokens -= cost;
        }
        let remaining = bucket.tokens.floor().max(0.0) as u32;
        let retry_after = if allowed {
            None
        } else {
            Some(Duration::from_secs_f64(
                (cost - bucket.tokens).max(0.0) / self.policy.refill_per_second,
            ))
        };
        RateLimitDecision {
            allowed,
            limit: self.policy.capacity,
            remaining,
            retry_after,
        }
    }

    pub fn clear(&self) {
        self.buckets
            .lock()
            .expect("rate limiter lock poisoned")
            .clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_burst_and_reports_retry_after() {
        let start = Instant::now();
        let limiter = MemoryRateLimiter::new(QuotaPolicy::new(2, 1.0));
        assert!(limiter.check_at("user-1", 1, start).allowed);
        assert!(limiter.check_at("user-1", 1, start).allowed);
        let denied = limiter.check_at("user-1", 1, start);
        assert!(!denied.allowed);
        assert_eq!(denied.remaining, 0);
        assert_eq!(denied.retry_after, Some(Duration::from_secs(1)));
        assert_eq!(denied.headers()["Retry-After"], "1");
    }

    #[test]
    fn refills_tokens_and_isolates_keys() {
        let start = Instant::now();
        let limiter = MemoryRateLimiter::new(QuotaPolicy::new(1, 2.0));
        assert!(limiter.check_at("a", 1, start).allowed);
        assert!(!limiter.check_at("a", 1, start).allowed);
        assert!(limiter.check_at("b", 1, start).allowed);
        assert!(
            limiter
                .check_at("a", 1, start + Duration::from_millis(500))
                .allowed
        );
    }

    #[test]
    fn concurrent_access_is_atomic() {
        let limiter = Arc::new(MemoryRateLimiter::new(QuotaPolicy::new(1, 0.1)));
        let start = Instant::now();
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let limiter = limiter.clone();
                std::thread::spawn(move || limiter.check_at("same", 1, start).allowed)
            })
            .collect();
        let allowed = threads
            .into_iter()
            .filter_map(|thread| thread.join().ok())
            .filter(|value| *value)
            .count();
        assert_eq!(allowed, 1);
    }
}
