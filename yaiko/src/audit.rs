//! Structured audit logging primitives for security and business events.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: u64,
    pub request_id: Option<String>,
    pub actor_id: Option<String>,
    pub action: String,
    pub resource: Option<String>,
    pub outcome: String,
    pub metadata: BTreeMap<String, String>,
    pub integrity_sha256: String,
}

impl AuditEvent {
    pub fn builder(action: impl Into<String>, outcome: impl Into<String>) -> AuditEventBuilder {
        AuditEventBuilder {
            action: action.into(),
            outcome: outcome.into(),
            timestamp: now_unix(),
            request_id: None,
            actor_id: None,
            resource: None,
            metadata: BTreeMap::new(),
        }
    }

    pub fn verify_integrity(&self) -> bool {
        self.integrity_sha256 == digest_for(self)
    }
}

pub struct AuditEventBuilder {
    action: String,
    outcome: String,
    timestamp: u64,
    request_id: Option<String>,
    actor_id: Option<String>,
    resource: Option<String>,
    metadata: BTreeMap<String, String>,
}

impl AuditEventBuilder {
    pub fn timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }
    pub fn actor_id(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> AuditEvent {
        let mut event = AuditEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: self.timestamp,
            request_id: self.request_id,
            actor_id: self.actor_id,
            action: self.action,
            resource: self.resource,
            outcome: self.outcome,
            metadata: redact_metadata(self.metadata),
            integrity_sha256: String::new(),
        };
        event.integrity_sha256 = digest_for(&event);
        event
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditQuery {
    pub request_id: Option<String>,
    pub actor_id: Option<String>,
    pub action: Option<String>,
    pub outcome: Option<String>,
    pub resource: Option<String>,
    pub since: Option<u64>,
    pub until: Option<u64>,
    pub limit: Option<usize>,
}

impl AuditQuery {
    pub fn action(mut self, value: impl Into<String>) -> Self {
        self.action = Some(value.into());
        self
    }
    pub fn actor_id(mut self, value: impl Into<String>) -> Self {
        self.actor_id = Some(value.into());
        self
    }
    pub fn outcome(mut self, value: impl Into<String>) -> Self {
        self.outcome = Some(value.into());
        self
    }
    pub fn limit(mut self, value: usize) -> Self {
        self.limit = Some(value);
        self
    }
}

#[derive(Clone)]
pub struct MemoryAuditSink {
    events: Arc<RwLock<VecDeque<AuditEvent>>>,
    capacity: usize,
}

impl MemoryAuditSink {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(capacity.min(1024)))),
            capacity: capacity.max(1),
        }
    }

    pub async fn record(&self, event: AuditEvent) {
        let mut events = self.events.write().await;
        if events.len() >= self.capacity {
            events.pop_front();
        }
        events.push_back(event);
    }

    pub async fn query(&self, query: &AuditQuery) -> Vec<AuditEvent> {
        let events = self.events.read().await;
        let mut result: Vec<_> = events
            .iter()
            .filter(|event| matches_query(event, query))
            .cloned()
            .collect();
        if let Some(limit) = query.limit {
            let keep_from = result.len().saturating_sub(limit);
            result = result.split_off(keep_from);
        }
        result
    }

    pub async fn len(&self) -> usize {
        self.events.read().await.len()
    }
    pub async fn is_empty(&self) -> bool {
        self.events.read().await.is_empty()
    }
}

fn matches_query(event: &AuditEvent, query: &AuditQuery) -> bool {
    query
        .request_id
        .as_deref()
        .map(|v| event.request_id.as_deref() == Some(v))
        .unwrap_or(true)
        && query
            .actor_id
            .as_deref()
            .map(|v| event.actor_id.as_deref() == Some(v))
            .unwrap_or(true)
        && query
            .action
            .as_deref()
            .map(|v| event.action == v)
            .unwrap_or(true)
        && query
            .outcome
            .as_deref()
            .map(|v| event.outcome == v)
            .unwrap_or(true)
        && query
            .resource
            .as_deref()
            .map(|v| event.resource.as_deref() == Some(v))
            .unwrap_or(true)
        && query.since.map(|v| event.timestamp >= v).unwrap_or(true)
        && query.until.map(|v| event.timestamp <= v).unwrap_or(true)
}

fn redact_metadata(metadata: BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .map(|(key, value)| {
            let lower = key.to_ascii_lowercase();
            let redacted = [
                "password",
                "token",
                "secret",
                "authorization",
                "cookie",
                "api_key",
            ]
            .iter()
            .any(|sensitive| lower.contains(sensitive));
            (
                key,
                if redacted {
                    "[REDACTED]".to_string()
                } else {
                    value
                },
            )
        })
        .collect()
}

fn digest_for(event: &AuditEvent) -> String {
    let canonical = format!(
        "{}|{}|{}|{}|{}|{}|{:?}",
        event.id,
        event.timestamp,
        event.request_id.as_deref().unwrap_or_default(),
        event.actor_id.as_deref().unwrap_or_default(),
        event.action,
        event.outcome,
        event.metadata
    );
    format!("sha256:{:x}", Sha256::digest(canonical.as_bytes()))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn redacts_sensitive_metadata_and_seals_integrity() {
        let event = AuditEvent::builder("login", "success")
            .actor_id("user-1")
            .metadata("password", "secret-value")
            .metadata("ip", "127.0.0.1")
            .build();
        assert_eq!(event.metadata["password"], "[REDACTED]");
        assert_eq!(event.metadata["ip"], "127.0.0.1");
        assert!(event.verify_integrity());
    }

    #[tokio::test]
    async fn sink_retains_capacity_and_filters_events() {
        let sink = MemoryAuditSink::new(2);
        sink.record(
            AuditEvent::builder("login", "success")
                .actor_id("user-1")
                .timestamp(1)
                .build(),
        )
        .await;
        sink.record(
            AuditEvent::builder("logout", "success")
                .actor_id("user-1")
                .timestamp(2)
                .build(),
        )
        .await;
        sink.record(
            AuditEvent::builder("delete", "denied")
                .actor_id("user-2")
                .timestamp(3)
                .build(),
        )
        .await;
        assert_eq!(sink.len().await, 2);
        let events = sink.query(&AuditQuery::default().actor_id("user-1")).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "logout");
    }
}
