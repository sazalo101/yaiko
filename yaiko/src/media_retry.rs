//! Bounded retry policies for media-processing jobs.

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFailure {
    TransientProcess,
    StorageUnavailable,
    RateLimited,
    InvalidInput,
    Cancelled,
    Permanent,
}

impl MediaFailure {
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::TransientProcess | Self::StorageUnavailable | Self::RateLimited
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryError {
    InvalidAttempts,
    InvalidDelay,
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryDecision {
    pub retry: bool,
    pub attempt: u32,
    pub delay: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaRetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter_percent: u8,
}

impl MediaRetryPolicy {
    pub fn new(
        max_attempts: u32,
        base_delay: Duration,
        max_delay: Duration,
        jitter_percent: u8,
    ) -> Result<Self, RetryError> {
        if !(1..=32).contains(&max_attempts) {
            return Err(RetryError::InvalidAttempts);
        }
        if base_delay.is_zero() || max_delay < base_delay || max_delay > Duration::from_secs(86_400)
        {
            return Err(RetryError::InvalidDelay);
        }
        if jitter_percent > 100 {
            return Err(RetryError::InvalidDelay);
        }
        Ok(Self {
            max_attempts,
            base_delay,
            max_delay,
            jitter_percent,
        })
    }
    pub fn decide(
        &self,
        attempt: u32,
        failure: MediaFailure,
        jitter_seed: u64,
    ) -> Result<RetryDecision, RetryError> {
        if attempt == 0 {
            return Err(RetryError::InvalidAttempts);
        }
        if attempt >= self.max_attempts
            || !failure.retryable()
            || matches!(failure, MediaFailure::Cancelled)
        {
            return Ok(RetryDecision {
                retry: false,
                attempt,
                delay: Duration::ZERO,
            });
        }
        let exponent = attempt.saturating_sub(1).min(31);
        let multiplier = 1u64.checked_shl(exponent).ok_or(RetryError::Overflow)?;
        let raw_ms = self
            .base_delay
            .as_millis()
            .checked_mul(u128::from(multiplier))
            .ok_or(RetryError::Overflow)?;
        let capped_ms = raw_ms.min(self.max_delay.as_millis());
        let jitter_range = capped_ms.saturating_mul(u128::from(self.jitter_percent)) / 100;
        let pseudo = u128::from(
            jitter_seed
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1),
        );
        let offset = if jitter_range == 0 {
            0
        } else {
            pseudo % (jitter_range + 1)
        };
        let adjusted_ms = capped_ms
            .saturating_sub(jitter_range / 2)
            .saturating_add(offset);
        let delay = Duration::from_millis(adjusted_ms.min(u128::from(u64::MAX)) as u64);
        Ok(RetryDecision {
            retry: true,
            attempt,
            delay,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retries_transient_failures_with_bounded_backoff() {
        let policy =
            MediaRetryPolicy::new(4, Duration::from_secs(1), Duration::from_secs(5), 0).unwrap();
        assert_eq!(
            policy.decide(1, MediaFailure::TransientProcess, 1).unwrap(),
            RetryDecision {
                retry: true,
                attempt: 1,
                delay: Duration::from_secs(1)
            }
        );
        assert_eq!(
            policy
                .decide(2, MediaFailure::StorageUnavailable, 1)
                .unwrap()
                .delay,
            Duration::from_secs(2)
        );
        assert_eq!(
            policy
                .decide(3, MediaFailure::RateLimited, 1)
                .unwrap()
                .delay,
            Duration::from_secs(4)
        );
        assert!(
            !policy
                .decide(4, MediaFailure::TransientProcess, 1)
                .unwrap()
                .retry
        );
    }
    #[test]
    fn rejects_permanent_and_cancelled_failures() {
        let policy =
            MediaRetryPolicy::new(3, Duration::from_secs(1), Duration::from_secs(10), 0).unwrap();
        for failure in [
            MediaFailure::InvalidInput,
            MediaFailure::Cancelled,
            MediaFailure::Permanent,
        ] {
            assert!(!policy.decide(1, failure, 1).unwrap().retry);
        }
    }
    #[test]
    fn validates_policy_attempts_delays_and_jitter() {
        assert_eq!(
            MediaRetryPolicy::new(0, Duration::from_secs(1), Duration::from_secs(2), 0),
            Err(RetryError::InvalidAttempts)
        );
        assert_eq!(
            MediaRetryPolicy::new(2, Duration::from_secs(2), Duration::from_secs(1), 0),
            Err(RetryError::InvalidDelay)
        );
        assert_eq!(
            MediaRetryPolicy::new(2, Duration::from_secs(1), Duration::from_secs(2), 101),
            Err(RetryError::InvalidDelay)
        );
    }
    #[test]
    fn jitter_is_bounded_and_seed_deterministic() {
        let policy =
            MediaRetryPolicy::new(3, Duration::from_secs(10), Duration::from_secs(10), 20).unwrap();
        let a = policy.decide(1, MediaFailure::TransientProcess, 7).unwrap();
        let b = policy.decide(1, MediaFailure::TransientProcess, 7).unwrap();
        assert_eq!(a, b);
        assert!(a.delay >= Duration::from_secs(9) && a.delay <= Duration::from_secs(11));
    }
}
