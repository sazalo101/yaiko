//! Extensible audit-log delivery sinks.

use crate::audit::{AuditEvent, MemoryAuditSink};
use async_trait::async_trait;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditSinkError {
    Io,
    Serialization,
    Capacity,
}

#[derive(Debug, Clone, Default)]
pub struct AuditFilter {
    actions: Option<HashSet<String>>,
    outcomes: Option<HashSet<String>>,
}

impl AuditFilter {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.actions = Some(actions.into_iter().map(Into::into).collect());
        self
    }
    pub fn outcomes(mut self, outcomes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.outcomes = Some(outcomes.into_iter().map(Into::into).collect());
        self
    }
    pub fn matches(&self, event: &AuditEvent) -> bool {
        self.actions
            .as_ref()
            .map(|set| set.contains(&event.action))
            .unwrap_or(true)
            && self
                .outcomes
                .as_ref()
                .map(|set| set.contains(&event.outcome))
                .unwrap_or(true)
    }
}

#[async_trait]
pub trait AuditSink: Clone + Send + Sync + 'static {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditSinkError>;
}

#[async_trait]
impl AuditSink for MemoryAuditSink {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditSinkError> {
        MemoryAuditSink::record(self, event).await;
        Ok(())
    }
}

#[derive(Clone)]
pub struct FilteredAuditSink<S> {
    inner: S,
    filter: AuditFilter,
}

impl<S> FilteredAuditSink<S> {
    pub fn new(inner: S, filter: AuditFilter) -> Self {
        Self { inner, filter }
    }
}

#[async_trait]
impl<S: AuditSink> AuditSink for FilteredAuditSink<S> {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditSinkError> {
        if self.filter.matches(&event) {
            self.inner.record(event).await?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct JsonlAuditSink {
    path: Arc<PathBuf>,
    write_lock: Arc<Mutex<()>>,
}

impl JsonlAuditSink {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: Arc::new(path.into()),
            write_lock: Arc::new(Mutex::new(())),
        }
    }
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[async_trait]
impl AuditSink for JsonlAuditSink {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditSinkError> {
        let line = serde_json::to_vec(&event).map_err(|_| AuditSinkError::Serialization)?;
        let _guard = self.write_lock.lock().await;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.path.as_ref())
            .await
            .map_err(|_| AuditSinkError::Io)?;
        file.write_all(&line)
            .await
            .map_err(|_| AuditSinkError::Io)?;
        file.write_all(b"\n")
            .await
            .map_err(|_| AuditSinkError::Io)?;
        file.flush().await.map_err(|_| AuditSinkError::Io)
    }
}

#[derive(Clone)]
pub struct BufferedAuditSink<S> {
    inner: S,
    queue: Arc<Mutex<VecDeque<AuditEvent>>>,
    capacity: usize,
}

impl<S: AuditSink> BufferedAuditSink<S> {
    pub fn new(inner: S, capacity: usize) -> Self {
        Self {
            inner,
            queue: Arc::new(Mutex::new(VecDeque::new())),
            capacity: capacity.max(1),
        }
    }
    pub async fn pending(&self) -> usize {
        self.queue.lock().await.len()
    }
    pub async fn flush(&self) -> Result<usize, AuditSinkError> {
        let events = {
            let mut queue = self.queue.lock().await;
            queue.drain(..).collect::<Vec<_>>()
        };
        let mut delivered = 0;
        for event in events {
            self.inner.record(event).await?;
            delivered += 1;
        }
        Ok(delivered)
    }
}

#[async_trait]
impl<S: AuditSink> AuditSink for BufferedAuditSink<S> {
    async fn record(&self, event: AuditEvent) -> Result<(), AuditSinkError> {
        let mut queue = self.queue.lock().await;
        if queue.len() >= self.capacity {
            return Err(AuditSinkError::Capacity);
        }
        queue.push_back(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditQuery;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn event(action: &str, outcome: &str) -> AuditEvent {
        AuditEvent::builder(action, outcome).build()
    }
    #[tokio::test]
    async fn filters_actions_and_outcomes() {
        let memory = MemoryAuditSink::new(8);
        let sink = FilteredAuditSink::new(
            memory.clone(),
            AuditFilter::new().actions(["login"]).outcomes(["success"]),
        );
        sink.record(event("login", "success")).await.unwrap();
        sink.record(event("login", "denied")).await.unwrap();
        sink.record(event("delete", "success")).await.unwrap();
        assert_eq!(memory.query(&AuditQuery::default()).await.len(), 1);
    }
    #[tokio::test]
    async fn writes_integrity_preserving_jsonl() {
        let path = std::env::temp_dir().join(format!(
            "yaiko-audit-{}.jsonl",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sink = JsonlAuditSink::new(&path);
        let original = event("upload", "success");
        sink.record(original.clone()).await.unwrap();
        let raw = tokio::fs::read_to_string(&path).await.unwrap();
        let decoded: AuditEvent = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
        assert_eq!(decoded.id, original.id);
        assert!(decoded.verify_integrity());
        let _ = tokio::fs::remove_file(path).await;
    }
    #[tokio::test]
    async fn bounded_buffer_requires_explicit_flush() {
        let memory = MemoryAuditSink::new(8);
        let sink = BufferedAuditSink::new(memory.clone(), 1);
        sink.record(event("one", "ok")).await.unwrap();
        assert_eq!(sink.pending().await, 1);
        assert_eq!(
            sink.record(event("two", "ok")).await,
            Err(AuditSinkError::Capacity)
        );
        assert_eq!(sink.flush().await.unwrap(), 1);
        assert!(!memory.is_empty().await);
    }
}
