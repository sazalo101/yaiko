//! Bounded WebSocket channel and heartbeat management primitives.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    Client,
    Server,
    Timeout,
    Capacity,
    Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeartbeatState {
    Healthy,
    Suspect,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    InvalidId,
    Capacity,
    UnknownConnection,
    UnknownChannel,
    AlreadyMember,
    NotMember,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMessage {
    pub channel: String,
    pub payload: String,
}

#[derive(Debug, Clone)]
struct ConnectionState {
    last_seen: Instant,
    state: HeartbeatState,
    channels: BTreeSet<String>,
    close_reason: Option<CloseReason>,
}

#[derive(Clone)]
pub struct ChannelRegistry {
    inner: Arc<Mutex<RegistryState>>,
    max_connections: usize,
    max_channels_per_connection: usize,
}

#[derive(Default)]
struct RegistryState {
    connections: BTreeMap<String, ConnectionState>,
    channels: BTreeMap<String, BTreeSet<String>>,
}

impl ChannelRegistry {
    pub fn new(max_connections: usize, max_channels_per_connection: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RegistryState::default())),
            max_connections,
            max_channels_per_connection,
        }
    }
    pub fn register(&self, connection_id: impl Into<String>) -> Result<(), ChannelError> {
        let connection_id = connection_id.into();
        if connection_id.trim().is_empty() {
            return Err(ChannelError::InvalidId);
        }
        let mut state = self.inner.lock().expect("channel registry poisoned");
        if state.connections.len() >= self.max_connections {
            return Err(ChannelError::Capacity);
        }
        state.connections.insert(
            connection_id,
            ConnectionState {
                last_seen: Instant::now(),
                state: HeartbeatState::Healthy,
                channels: BTreeSet::new(),
                close_reason: None,
            },
        );
        Ok(())
    }
    pub fn join(
        &self,
        connection_id: &str,
        channel: impl Into<String>,
    ) -> Result<(), ChannelError> {
        let channel = channel.into();
        if channel.trim().is_empty() {
            return Err(ChannelError::InvalidId);
        }
        let mut state = self.inner.lock().expect("channel registry poisoned");
        let connection = state
            .connections
            .get_mut(connection_id)
            .ok_or(ChannelError::UnknownConnection)?;
        if connection.channels.contains(&channel) {
            return Err(ChannelError::AlreadyMember);
        }
        if connection.channels.len() >= self.max_channels_per_connection {
            return Err(ChannelError::Capacity);
        }
        connection.channels.insert(channel.clone());
        state
            .channels
            .entry(channel)
            .or_default()
            .insert(connection_id.into());
        Ok(())
    }
    pub fn leave(&self, connection_id: &str, channel: &str) -> Result<(), ChannelError> {
        let mut state = self.inner.lock().expect("channel registry poisoned");
        let connection = state
            .connections
            .get_mut(connection_id)
            .ok_or(ChannelError::UnknownConnection)?;
        if !connection.channels.remove(channel) {
            return Err(ChannelError::NotMember);
        }
        if let Some(members) = state.channels.get_mut(channel) {
            members.remove(connection_id);
            if members.is_empty() {
                state.channels.remove(channel);
            }
        }
        Ok(())
    }
    pub fn heartbeat(&self, connection_id: &str) -> Result<(), ChannelError> {
        let mut state = self.inner.lock().expect("channel registry poisoned");
        let connection = state
            .connections
            .get_mut(connection_id)
            .ok_or(ChannelError::UnknownConnection)?;
        connection.last_seen = Instant::now();
        connection.state = HeartbeatState::Healthy;
        Ok(())
    }
    pub fn expire(&self, timeout: Duration) -> Vec<String> {
        let mut state = self.inner.lock().expect("channel registry poisoned");
        let now = Instant::now();
        let expired = state
            .connections
            .iter_mut()
            .filter_map(|(id, connection)| {
                if now.duration_since(connection.last_seen) >= timeout {
                    connection.state = HeartbeatState::Expired;
                    connection.close_reason = Some(CloseReason::Timeout);
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect();
        expired
    }
    pub fn close(&self, connection_id: &str, reason: CloseReason) -> Result<(), ChannelError> {
        let mut state = self.inner.lock().expect("channel registry poisoned");
        let connection = state
            .connections
            .get_mut(connection_id)
            .ok_or(ChannelError::UnknownConnection)?;
        connection.close_reason = Some(reason);
        Ok(())
    }
    pub fn members(&self, channel: &str) -> Vec<String> {
        self.inner
            .lock()
            .expect("channel registry poisoned")
            .channels
            .get(channel)
            .map(|members| members.iter().cloned().collect())
            .unwrap_or_default()
    }
    pub fn state(&self, connection_id: &str) -> Option<HeartbeatState> {
        self.inner
            .lock()
            .expect("channel registry poisoned")
            .connections
            .get(connection_id)
            .map(|connection| connection.state)
    }
    pub fn close_reason(&self, connection_id: &str) -> Option<CloseReason> {
        self.inner
            .lock()
            .expect("channel registry poisoned")
            .connections
            .get(connection_id)
            .and_then(|connection| connection.close_reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_leaves_and_limits_channels() {
        let registry = ChannelRegistry::new(2, 1);
        registry.register("c1").unwrap();
        registry.join("c1", "room").unwrap();
        assert_eq!(registry.members("room"), vec!["c1"]);
        assert_eq!(registry.join("c1", "other"), Err(ChannelError::Capacity));
        registry.leave("c1", "room").unwrap();
        assert!(registry.members("room").is_empty());
    }

    #[test]
    fn enforces_connection_capacity_and_close_reasons() {
        let registry = ChannelRegistry::new(1, 2);
        registry.register("c1").unwrap();
        assert_eq!(registry.register("c2"), Err(ChannelError::Capacity));
        registry.close("c1", CloseReason::Server).unwrap();
        assert_eq!(registry.close_reason("c1"), Some(CloseReason::Server));
    }

    #[test]
    fn heartbeat_marks_connections_healthy_and_expiry_is_explicit() {
        let registry = ChannelRegistry::new(2, 2);
        registry.register("c1").unwrap();
        registry.heartbeat("c1").unwrap();
        assert_eq!(registry.state("c1"), Some(HeartbeatState::Healthy));
        assert!(registry
            .expire(Duration::ZERO)
            .contains(&String::from("c1")));
        assert_eq!(registry.close_reason("c1"), Some(CloseReason::Timeout));
    }
}
