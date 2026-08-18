//! Bounded query-client cache facade.
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryClientError {
    InvalidKey,
    Capacity,
    InvalidTtl,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryEntry {
    pub key: String,
    pub value: Vec<u8>,
    pub stored_at: u64,
    pub ttl: u64,
}
#[derive(Debug, Clone)]
pub struct QueryClient {
    entries: BTreeMap<String, QueryEntry>,
    capacity: usize,
}
impl QueryClient {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            capacity,
        }
    }
    pub fn set(
        &mut self,
        key: impl Into<String>,
        value: impl Into<Vec<u8>>,
        stored_at: u64,
        ttl: u64,
    ) -> Result<(), QueryClientError> {
        let key = valid(key.into())?;
        if ttl == 0 {
            return Err(QueryClientError::InvalidTtl);
        }
        if !self.entries.contains_key(&key) && self.entries.len() >= self.capacity {
            return Err(QueryClientError::Capacity);
        }
        self.entries.insert(
            key.clone(),
            QueryEntry {
                key,
                value: value.into(),
                stored_at,
                ttl,
            },
        );
        Ok(())
    }
    pub fn get(&self, key: &str, now: u64) -> Option<&QueryEntry> {
        self.entries
            .get(key)
            .filter(|e| now.saturating_sub(e.stored_at) <= e.ttl)
    }
    pub fn invalidate(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }
    pub fn invalidate_prefix(&mut self, prefix: &str) -> usize {
        let keys = self
            .entries
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect::<Vec<_>>();
        let n = keys.len();
        for k in keys {
            self.entries.remove(&k);
        }
        n
    }
    pub fn snapshot(&self) -> Vec<QueryEntry> {
        self.entries.values().cloned().collect()
    }
}
impl Default for QueryClient {
    fn default() -> Self {
        Self::new(1024)
    }
}
fn valid(k: String) -> Result<String, QueryClientError> {
    if k.is_empty() || k.len() > 256 || k.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(QueryClientError::InvalidKey)
    } else {
        Ok(k)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sets_gets_stale_and_invalidates() {
        let mut c = QueryClient::new(2);
        c.set("media:1", b"ok".to_vec(), 10, 5).unwrap();
        assert!(c.get("media:1", 15).is_some());
        assert!(c.get("media:1", 16).is_none());
        c.set("media:2", Vec::new(), 1, 2).unwrap();
        assert_eq!(c.invalidate_prefix("media:"), 2)
    }
    #[test]
    fn validates_ttl_keys_and_capacity() {
        let mut c = QueryClient::new(1);
        assert!(c.set("bad key", Vec::new(), 0, 1).is_err());
        assert!(c.set("a", Vec::new(), 0, 0).is_err());
        c.set("a", Vec::new(), 0, 1).unwrap();
        assert!(c.set("b", Vec::new(), 0, 1).is_err())
    }
}
