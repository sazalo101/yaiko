//! Typed health aggregation facade.
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentStatus {
    Up,
    Degraded,
    Down,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthError {
    InvalidName,
    InvalidMessage,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentReport {
    pub name: String,
    pub status: ComponentStatus,
    pub message: String,
}
#[derive(Debug, Clone, Default)]
pub struct HealthFacade {
    components: BTreeMap<String, ComponentReport>,
    capacity: usize,
}
impl HealthFacade {
    pub fn new(capacity: usize) -> Self {
        Self {
            components: BTreeMap::new(),
            capacity,
        }
    }
    pub fn report(
        &mut self,
        name: impl Into<String>,
        status: ComponentStatus,
        message: impl Into<String>,
    ) -> Result<(), HealthError> {
        let name = name.into();
        let message = message.into();
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(HealthError::InvalidName);
        }
        if message.len() > 512 {
            return Err(HealthError::InvalidMessage);
        }
        if !self.components.contains_key(&name) && self.components.len() >= self.capacity {
            return Err(HealthError::Capacity);
        }
        self.components.insert(
            name.clone(),
            ComponentReport {
                name,
                status,
                message,
            },
        );
        Ok(())
    }
    pub fn status(&self) -> HealthStatus {
        if self
            .components
            .values()
            .any(|c| c.status == ComponentStatus::Down)
        {
            HealthStatus::Unhealthy
        } else if self
            .components
            .values()
            .any(|c| c.status == ComponentStatus::Degraded)
        {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
    pub fn ready(&self) -> bool {
        self.components
            .values()
            .all(|c| c.status != ComponentStatus::Down)
    }
    pub fn live(&self) -> bool {
        true
    }
    pub fn snapshot(&self) -> Vec<ComponentReport> {
        self.components.values().cloned().collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aggregates_status_in_deterministic_order() {
        let mut h = HealthFacade::new(2);
        h.report("db", ComponentStatus::Up, "ok").unwrap();
        h.report("cache", ComponentStatus::Degraded, "slow")
            .unwrap();
        assert_eq!(h.status(), HealthStatus::Degraded);
        assert!(h.ready() && h.live());
        assert_eq!(h.snapshot()[0].name, "cache")
    }
    #[test]
    fn detects_down_and_validates_bounds() {
        let mut h = HealthFacade::new(1);
        h.report("db", ComponentStatus::Down, "failed").unwrap();
        assert_eq!(h.status(), HealthStatus::Unhealthy);
        assert!(!h.ready());
        assert!(h.report("bad name", ComponentStatus::Up, "ok").is_err());
        assert!(h.report("x", ComponentStatus::Up, "x".repeat(513)).is_err())
    }
}
