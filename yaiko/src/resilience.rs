//! Resilience primitives for outbound calls and worker execution.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResilienceError {
    OpenCircuit,
    BulkheadFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
struct CircuitInner {
    state: CircuitState,
    failures: u32,
    opened_at: Option<Instant>,
    probe_in_flight: bool,
}

#[derive(Clone)]
pub struct CircuitBreaker {
    inner: Arc<Mutex<CircuitInner>>,
    failure_threshold: u32,
    cooldown: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CircuitInner {
                state: CircuitState::Closed,
                failures: 0,
                opened_at: None,
                probe_in_flight: false,
            })),
            failure_threshold: failure_threshold.max(1),
            cooldown: if cooldown.is_zero() {
                Duration::from_millis(1)
            } else {
                cooldown
            },
        }
    }
    pub async fn state(&self) -> CircuitState {
        self.inner.lock().await.state
    }
    pub async fn allow(&self) -> Result<(), ResilienceError> {
        let mut inner = self.inner.lock().await;
        match inner.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open
                if inner
                    .opened_at
                    .map(|at| at.elapsed() >= self.cooldown)
                    .unwrap_or(false) =>
            {
                inner.state = CircuitState::HalfOpen;
                if inner.probe_in_flight {
                    return Err(ResilienceError::OpenCircuit);
                }
                inner.probe_in_flight = true;
                Ok(())
            }
            CircuitState::HalfOpen if !inner.probe_in_flight => {
                inner.probe_in_flight = true;
                Ok(())
            }
            _ => Err(ResilienceError::OpenCircuit),
        }
    }
    pub async fn success(&self) {
        let mut inner = self.inner.lock().await;
        inner.state = CircuitState::Closed;
        inner.failures = 0;
        inner.opened_at = None;
        inner.probe_in_flight = false;
    }
    pub async fn failure(&self) {
        let mut inner = self.inner.lock().await;
        inner.probe_in_flight = false;
        inner.failures = inner.failures.saturating_add(1);
        if inner.failures >= self.failure_threshold {
            inner.state = CircuitState::Open;
            inner.opened_at = Some(Instant::now());
        }
    }
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<Result<T, E>, ResilienceError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        self.allow().await?;
        let result = operation().await;
        if result.is_ok() {
            self.success().await;
        } else {
            self.failure().await;
        }
        Ok(result)
    }
}

#[derive(Clone)]
pub struct Bulkhead {
    permits: Arc<Semaphore>,
}

pub struct BulkheadPermit {
    _permit: OwnedSemaphorePermit,
}

impl Bulkhead {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max_concurrency.max(1))),
        }
    }
    pub fn available(&self) -> usize {
        self.permits.available_permits()
    }
    pub async fn try_acquire(&self) -> Result<BulkheadPermit, ResilienceError> {
        self.permits
            .clone()
            .try_acquire_owned()
            .map(|permit| BulkheadPermit { _permit: permit })
            .map_err(|_| ResilienceError::BulkheadFull)
    }
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, ResilienceError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let _permit = self.try_acquire().await?;
        Ok(operation().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn opens_after_threshold_and_recovers_after_cooldown() {
        let breaker = CircuitBreaker::new(2, Duration::from_millis(10));
        assert_eq!(breaker.state().await, CircuitState::Closed);
        breaker.failure().await;
        assert!(breaker.allow().await.is_ok());
        breaker.failure().await;
        assert_eq!(breaker.state().await, CircuitState::Open);
        assert_eq!(breaker.allow().await, Err(ResilienceError::OpenCircuit));
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert!(breaker.allow().await.is_ok());
        assert_eq!(breaker.state().await, CircuitState::HalfOpen);
        breaker.success().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }
    #[tokio::test]
    async fn successful_calls_reset_failures() {
        let breaker = CircuitBreaker::new(2, Duration::from_secs(1));
        breaker.failure().await;
        breaker.success().await;
        breaker.failure().await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
        assert!(breaker.allow().await.is_ok());
    }
    #[tokio::test]
    async fn bulkhead_rejects_over_capacity_and_releases() {
        let bulkhead = Bulkhead::new(1);
        let permit = bulkhead.try_acquire().await.unwrap();
        assert!(matches!(
            bulkhead.try_acquire().await,
            Err(ResilienceError::BulkheadFull)
        ));
        drop(permit);
        assert!(bulkhead.try_acquire().await.is_ok());
    }
    #[tokio::test]
    async fn execute_composes_bulkhead_and_async_work() {
        let bulkhead = Bulkhead::new(1);
        let value = bulkhead.execute(|| async { 42 }).await.unwrap();
        assert_eq!(value, 42);
    }
}
