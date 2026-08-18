//! Deterministic file-watch policy metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEvent {
    Create,
    Modify,
    Remove,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchError {
    InvalidPath,
    InvalidDebounce,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchRegistration {
    pub path: String,
    pub events: Vec<WatchEvent>,
    pub debounce_ms: u64,
}
#[derive(Debug, Clone, Default)]
pub struct Watcher {
    registrations: Vec<WatchRegistration>,
    capacity: usize,
}
impl Watcher {
    pub fn new(capacity: usize) -> Self {
        Self {
            registrations: Vec::new(),
            capacity,
        }
    }
    pub fn register(
        mut self,
        path: impl Into<String>,
        events: impl Into<Vec<WatchEvent>>,
        debounce_ms: u64,
    ) -> Result<Self, WatchError> {
        let path = path.into();
        if path.is_empty()
            || !path.starts_with('/')
            || path.contains("..")
            || path.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(WatchError::InvalidPath);
        }
        if debounce_ms == 0 {
            return Err(WatchError::InvalidDebounce);
        }
        if self.registrations.len() >= self.capacity {
            return Err(WatchError::Capacity);
        }
        let mut events = events.into();
        events.sort_by_key(|e| match e {
            WatchEvent::Create => 0,
            WatchEvent::Modify => 1,
            WatchEvent::Remove => 2,
        });
        events.dedup();
        self.registrations.push(WatchRegistration {
            path,
            events,
            debounce_ms,
        });
        self.registrations.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(self)
    }
    pub fn matches(&self, path: &str, event: WatchEvent) -> bool {
        self.registrations
            .iter()
            .any(|r| path.starts_with(&r.path) && r.events.contains(&event))
    }
    pub fn snapshot(&self) -> &[WatchRegistration] {
        &self.registrations
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registers_sorted_and_filters_events() {
        let w = Watcher::new(2)
            .register(
                "/src",
                vec![WatchEvent::Modify, WatchEvent::Create, WatchEvent::Create],
                50,
            )
            .unwrap()
            .register("/assets", vec![WatchEvent::Remove], 10)
            .unwrap();
        assert_eq!(w.snapshot()[0].path, "/assets");
        assert!(w.matches("/src/lib.rs", WatchEvent::Modify));
        assert!(!w.matches("/src/lib.rs", WatchEvent::Remove))
    }
    #[test]
    fn validates_paths_debounce_and_capacity() {
        assert!(Watcher::new(1).register("../src", Vec::new(), 1).is_err());
        assert!(Watcher::new(1).register("/src", Vec::new(), 0).is_err());
        let w = Watcher::new(1).register("/src", Vec::new(), 1).unwrap();
        assert!(w.register("/assets", Vec::new(), 1).is_err())
    }
}
