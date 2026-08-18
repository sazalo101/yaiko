//! Validated database pool policy facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    InvalidConnections,
    InvalidTimeout,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolPolicy {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_ms: u64,
    pub idle_timeout_ms: u64,
}
impl PoolPolicy {
    pub fn new(max_connections: u32) -> Result<Self, PoolError> {
        if max_connections == 0 {
            return Err(PoolError::InvalidConnections);
        }
        Ok(Self {
            max_connections,
            min_connections: 0,
            acquire_timeout_ms: 30_000,
            idle_timeout_ms: 600_000,
        })
    }
    pub fn min_connections(mut self, min: u32) -> Result<Self, PoolError> {
        if min > self.max_connections {
            return Err(PoolError::InvalidConnections);
        }
        self.min_connections = min;
        Ok(self)
    }
    pub fn acquire_timeout_ms(mut self, ms: u64) -> Result<Self, PoolError> {
        if ms == 0 {
            return Err(PoolError::InvalidTimeout);
        }
        self.acquire_timeout_ms = ms;
        Ok(self)
    }
    pub fn idle_timeout_ms(mut self, ms: u64) -> Result<Self, PoolError> {
        if ms == 0 {
            return Err(PoolError::InvalidTimeout);
        }
        self.idle_timeout_ms = ms;
        Ok(self)
    }
}
impl Default for PoolPolicy {
    fn default() -> Self {
        Self::new(10).unwrap()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_deterministic_pool_policy() {
        let p = PoolPolicy::new(10)
            .unwrap()
            .min_connections(2)
            .unwrap()
            .acquire_timeout_ms(1000)
            .unwrap();
        assert_eq!(p.max_connections, 10);
        assert_eq!(p.min_connections, 2);
        assert_eq!(p.acquire_timeout_ms, 1000)
    }
    #[test]
    fn validates_bounds_and_timeouts() {
        assert!(PoolPolicy::new(0).is_err());
        assert!(PoolPolicy::new(2).unwrap().min_connections(3).is_err());
        assert!(PoolPolicy::new(1).unwrap().acquire_timeout_ms(0).is_err());
        assert!(PoolPolicy::new(1).unwrap().idle_timeout_ms(0).is_err())
    }
}
