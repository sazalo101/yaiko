//! Runtime feature flags and configuration reload primitives.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FeatureValue {
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Json(Value),
}

impl FeatureValue {
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(value) = self {
            Some(*value)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FeatureSnapshot {
    pub version: u64,
    pub flags: BTreeMap<String, FeatureValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureChange {
    pub version: u64,
    pub changed: Vec<String>,
}

#[derive(Clone)]
pub struct FeatureFlagStore {
    snapshot: Arc<RwLock<FeatureSnapshot>>,
    changes: broadcast::Sender<FeatureChange>,
}

impl FeatureFlagStore {
    pub fn new(initial: FeatureSnapshot) -> Self {
        let (changes, _) = broadcast::channel(32);
        Self {
            snapshot: Arc::new(RwLock::new(initial)),
            changes,
        }
    }

    pub fn empty() -> Self {
        Self::new(FeatureSnapshot::default())
    }
    pub fn subscribe(&self) -> broadcast::Receiver<FeatureChange> {
        self.changes.subscribe()
    }

    pub async fn snapshot(&self) -> FeatureSnapshot {
        self.snapshot.read().await.clone()
    }

    pub async fn get(&self, name: &str) -> Option<FeatureValue> {
        self.snapshot.read().await.flags.get(name).cloned()
    }

    pub async fn enabled(&self, name: &str) -> bool {
        self.get(name)
            .await
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub async fn set(&self, name: impl Into<String>, value: FeatureValue) -> FeatureChange {
        let name = name.into();
        let mut snapshot = self.snapshot.write().await;
        snapshot.version += 1;
        snapshot.flags.insert(name.clone(), value);
        let change = FeatureChange {
            version: snapshot.version,
            changed: vec![name],
        };
        let _ = self.changes.send(change.clone());
        change
    }

    pub async fn remove(&self, name: &str) -> Option<FeatureChange> {
        let mut snapshot = self.snapshot.write().await;
        snapshot.flags.remove(name)?;
        snapshot.version += 1;
        let change = FeatureChange {
            version: snapshot.version,
            changed: vec![name.to_string()],
        };
        let _ = self.changes.send(change.clone());
        Some(change)
    }

    pub async fn replace(&self, flags: BTreeMap<String, FeatureValue>) -> FeatureChange {
        let mut snapshot = self.snapshot.write().await;
        let changed = snapshot
            .flags
            .keys()
            .chain(flags.keys())
            .filter(|key| snapshot.flags.get(*key) != flags.get(*key))
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        snapshot.version += 1;
        snapshot.flags = flags;
        let change = FeatureChange {
            version: snapshot.version,
            changed,
        };
        let _ = self.changes.send(change.clone());
        change
    }

    pub async fn load_json(&self, json: &str) -> Result<FeatureChange, serde_json::Error> {
        let flags = serde_json::from_str::<BTreeMap<String, FeatureValue>>(json)?;
        Ok(self.replace(flags).await)
    }
}

impl Default for FeatureFlagStore {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn supports_typed_flags_and_json_reload() {
        let store = FeatureFlagStore::empty();
        store.set("captions", FeatureValue::Bool(true)).await;
        assert!(store.enabled("captions").await);
        store
            .load_json(r#"{"captions":false,"max_tracks":4}"#)
            .await
            .unwrap();
        assert!(!store.enabled("captions").await);
        assert_eq!(
            store.get("max_tracks").await,
            Some(FeatureValue::Integer(4))
        );
        assert_eq!(store.snapshot().await.version, 2);
    }

    #[tokio::test]
    async fn replacement_is_atomic_and_notifies_subscribers() {
        let store = FeatureFlagStore::empty();
        let mut receiver = store.subscribe();
        let change = store.set("beta", FeatureValue::Bool(true)).await;
        assert_eq!(change.version, 1);
        let notification = receiver.recv().await.unwrap();
        assert_eq!(notification, change);
        let mut flags = BTreeMap::new();
        flags.insert("stable".to_string(), FeatureValue::Bool(true));
        let replacement = store.replace(flags).await;
        assert_eq!(
            replacement.changed,
            vec!["beta".to_string(), "stable".to_string()]
        );
        assert!(store.enabled("stable").await);
        assert!(!store.enabled("beta").await);
    }

    #[tokio::test]
    async fn removing_unknown_flag_is_noop() {
        let store = FeatureFlagStore::empty();
        assert!(store.remove("missing").await.is_none());
        assert_eq!(store.snapshot().await.version, 0);
    }
}
