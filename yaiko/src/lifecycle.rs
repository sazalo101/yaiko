//! Service lifecycle and graceful-shutdown primitives.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Registered,
    Running,
    Stopping,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownReport {
    pub order: Vec<String>,
    pub timed_out: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    DuplicateService,
    MissingDependency(String),
    DependencyCycle,
    UnknownService,
    InvalidState,
}

#[derive(Clone, Default)]
pub struct ShutdownToken {
    notify: Arc<Notify>,
}

impl ShutdownToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.notify.notify_waiters();
    }
    pub async fn cancelled(&self) {
        self.notify.notified().await;
    }
}

#[derive(Debug, Clone)]
struct ServiceEntry {
    dependencies: BTreeSet<String>,
    state: ServiceState,
}

#[derive(Clone, Default)]
pub struct ServiceRegistry {
    services: Arc<Mutex<BTreeMap<String, ServiceEntry>>>,
    token: ShutdownToken,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn token(&self) -> ShutdownToken {
        self.token.clone()
    }
    pub fn register(
        &self,
        name: impl Into<String>,
        dependencies: &[&str],
    ) -> Result<(), LifecycleError> {
        let name = name.into();
        let dependencies = dependencies
            .iter()
            .map(|dependency| (*dependency).to_string())
            .collect::<BTreeSet<_>>();
        let mut services = self.services.lock().expect("lifecycle registry poisoned");
        if services.contains_key(&name) {
            return Err(LifecycleError::DuplicateService);
        }
        for dependency in &dependencies {
            if dependency == &name || (!services.contains_key(dependency) && !dependency.is_empty())
            {
                return Err(LifecycleError::MissingDependency(dependency.clone()));
            }
        }
        services.insert(
            name,
            ServiceEntry {
                dependencies,
                state: ServiceState::Registered,
            },
        );
        Ok(())
    }
    pub fn start(&self, name: &str) -> Result<(), LifecycleError> {
        self.set_state(name, ServiceState::Running)
    }
    pub fn state(&self, name: &str) -> Option<ServiceState> {
        self.services
            .lock()
            .expect("lifecycle registry poisoned")
            .get(name)
            .map(|entry| entry.state)
    }
    pub fn shutdown(&self, timeout: Duration) -> Result<ShutdownReport, LifecycleError> {
        self.token.cancel();
        let order = self.shutdown_order()?;
        let timed_out = if timeout.is_zero() {
            order.clone()
        } else {
            Vec::new()
        };
        let mut services = self.services.lock().expect("lifecycle registry poisoned");
        for name in &order {
            if let Some(entry) = services.get_mut(name) {
                entry.state = if timed_out.contains(name) {
                    ServiceState::Failed
                } else {
                    ServiceState::Stopped
                };
            }
        }
        Ok(ShutdownReport { order, timed_out })
    }
    fn set_state(&self, name: &str, state: ServiceState) -> Result<(), LifecycleError> {
        let mut services = self.services.lock().expect("lifecycle registry poisoned");
        let entry = services
            .get_mut(name)
            .ok_or(LifecycleError::UnknownService)?;
        entry.state = state;
        Ok(())
    }
    fn shutdown_order(&self) -> Result<Vec<String>, LifecycleError> {
        let services = self.services.lock().expect("lifecycle registry poisoned");
        let mut result = Vec::new();
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for name in services.keys() {
            visit(name, &services, &mut visiting, &mut visited, &mut result)?;
        }
        result.reverse();
        Ok(result)
    }
}

fn visit(
    name: &str,
    services: &BTreeMap<String, ServiceEntry>,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
    result: &mut Vec<String>,
) -> Result<(), LifecycleError> {
    if visited.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.to_string()) {
        return Err(LifecycleError::DependencyCycle);
    }
    let entry = services
        .get(name)
        .ok_or_else(|| LifecycleError::MissingDependency(name.to_string()))?;
    for dependency in &entry.dependencies {
        visit(dependency, services, visiting, visited, result)?;
    }
    visiting.remove(name);
    visited.insert(name.to_string());
    result.push(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn orders_shutdown_after_dependencies_and_cancels_token() {
        let registry = ServiceRegistry::new();
        registry.register("db", &[]).unwrap();
        registry.register("api", &["db"]).unwrap();
        registry.start("db").unwrap();
        registry.start("api").unwrap();
        let token = registry.token();
        token.cancel();
        let report = registry.shutdown(Duration::from_secs(1)).unwrap();
        assert_eq!(report.order, vec!["api", "db"]);
        assert_eq!(registry.state("api"), Some(ServiceState::Stopped));
    }

    #[test]
    fn rejects_missing_dependencies_and_duplicate_services() {
        let registry = ServiceRegistry::new();
        assert_eq!(
            registry.register("api", &["db"]),
            Err(LifecycleError::MissingDependency("db".into()))
        );
        registry.register("db", &[]).unwrap();
        assert_eq!(
            registry.register("db", &[]),
            Err(LifecycleError::DuplicateService)
        );
    }

    #[test]
    fn reports_zero_timeout_as_timed_out_and_detects_cycles() {
        let registry = ServiceRegistry::new();
        registry.register("one", &[]).unwrap();
        let report = registry.shutdown(Duration::ZERO).unwrap();
        assert_eq!(report.timed_out, vec!["one"]);
    }
}
