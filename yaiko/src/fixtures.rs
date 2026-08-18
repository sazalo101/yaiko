//! Deterministic test-data factories and cleanup helpers.

use serde_json::{Map, Value};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct FixtureFactory {
    seed: u64,
}

impl FixtureFactory {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
    pub fn sequence(&self, prefix: &str, index: u64) -> String {
        format!("{prefix}-{}", self.seed.saturating_add(index))
    }
    pub fn object(&self, fields: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
        Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect::<Map<_, _>>(),
        )
    }
    pub fn nested(&self, path: &str, value: Value) -> Value {
        let mut root = value;
        for key in path.split('.').rev() {
            root = self.object([(key.to_string(), root)]);
        }
        root
    }
}

type CleanupAction = Box<dyn FnOnce() + Send>;

#[derive(Clone, Default)]
pub struct CleanupGuard {
    actions: Arc<Mutex<Vec<CleanupAction>>>,
}

impl CleanupGuard {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push<F: FnOnce() + Send + 'static>(&self, cleanup: F) {
        self.actions
            .lock()
            .expect("cleanup guard poisoned")
            .push(Box::new(cleanup));
    }
    pub fn run(&self) {
        let actions = std::mem::take(&mut *self.actions.lock().expect("cleanup guard poisoned"));
        for action in actions.into_iter().rev() {
            action();
        }
    }
    pub fn is_empty(&self) -> bool {
        self.actions
            .lock()
            .expect("cleanup guard poisoned")
            .is_empty()
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if Arc::strong_count(&self.actions) == 1 {
            self.run();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn produces_deterministic_sequences_and_nested_objects() {
        let factory = FixtureFactory::new(100);
        assert_eq!(factory.sequence("user", 0), "user-100");
        assert_eq!(factory.sequence("user", 2), "user-102");
        assert_eq!(
            factory.nested("user.profile.name", Value::String("Ada".into())),
            serde_json::json!({"user":{"profile":{"name":"Ada"}}})
        );
    }

    #[test]
    fn cleanup_runs_in_reverse_order_and_is_idempotent() {
        let guard = CleanupGuard::new();
        let order = Arc::new(Mutex::new(Vec::new()));
        let first = order.clone();
        guard.push(move || first.lock().unwrap().push(1));
        let second = order.clone();
        guard.push(move || second.lock().unwrap().push(2));
        guard.run();
        guard.run();
        assert_eq!(*order.lock().unwrap(), vec![2, 1]);
        assert!(guard.is_empty());
    }

    #[test]
    fn cleanup_runs_on_drop() {
        let counter = Arc::new(AtomicUsize::new(0));
        {
            let guard = CleanupGuard::new();
            let counter = counter.clone();
            guard.push(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
