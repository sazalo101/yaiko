//! Bounded in-process pub/sub channel facade.
use std::collections::{BTreeMap, VecDeque};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PubSubError {
    InvalidName,
    Capacity,
    NotFound,
    DuplicateSubscriber,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubSubMessage {
    pub sequence: u64,
    pub channel: String,
    pub payload: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubSubSubscription {
    pub id: String,
    pub channel: String,
    pub cursor: u64,
}
#[derive(Debug, Clone)]
pub struct PubSub {
    channels: BTreeMap<String, VecDeque<PubSubMessage>>,
    subscribers: BTreeMap<String, PubSubSubscription>,
    capacity: usize,
    next_sequence: u64,
}
impl PubSub {
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: BTreeMap::new(),
            subscribers: BTreeMap::new(),
            capacity,
            next_sequence: 0,
        }
    }
    pub fn create_channel(&mut self, name: impl Into<String>) -> Result<(), PubSubError> {
        let name = valid(name.into())?;
        self.channels.entry(name).or_default();
        Ok(())
    }
    pub fn subscribe(
        &mut self,
        id: impl Into<String>,
        channel: impl Into<String>,
    ) -> Result<PubSubSubscription, PubSubError> {
        let id = valid(id.into())?;
        let channel = valid(channel.into())?;
        if !self.channels.contains_key(&channel) {
            return Err(PubSubError::NotFound);
        }
        if self.subscribers.contains_key(&id) {
            return Err(PubSubError::DuplicateSubscriber);
        }
        let s = PubSubSubscription {
            id: id.clone(),
            channel: channel.clone(),
            cursor: self.next_sequence,
        };
        self.subscribers.insert(id, s.clone());
        Ok(s)
    }
    pub fn unsubscribe(&mut self, id: &str) -> Result<(), PubSubError> {
        self.subscribers
            .remove(id)
            .map(|_| ())
            .ok_or(PubSubError::NotFound)
    }
    pub fn publish(
        &mut self,
        channel: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<PubSubMessage, PubSubError> {
        let channel = valid(channel.into())?;
        let queue = self
            .channels
            .get_mut(&channel)
            .ok_or(PubSubError::NotFound)?;
        if self.capacity == 0 {
            return Err(PubSubError::Capacity);
        }
        self.next_sequence += 1;
        let msg = PubSubMessage {
            sequence: self.next_sequence,
            channel,
            payload: payload.into(),
        };
        if queue.len() >= self.capacity {
            queue.pop_front();
        }
        queue.push_back(msg.clone());
        Ok(msg)
    }
    pub fn poll(&mut self, id: &str) -> Result<Vec<PubSubMessage>, PubSubError> {
        let s = self.subscribers.get_mut(id).ok_or(PubSubError::NotFound)?;
        let queue = self.channels.get(&s.channel).ok_or(PubSubError::NotFound)?;
        let out = queue
            .iter()
            .filter(|m| m.sequence > s.cursor)
            .cloned()
            .collect::<Vec<_>>();
        if let Some(last) = out.last() {
            s.cursor = last.sequence
        }
        Ok(out)
    }
}
impl Default for PubSub {
    fn default() -> Self {
        Self::new(1024)
    }
}
fn valid(x: String) -> Result<String, PubSubError> {
    if x.is_empty() || x.len() > 128 || x.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(PubSubError::InvalidName)
    } else {
        Ok(x)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publishes_and_polls_in_order() {
        let mut p = PubSub::new(4);
        p.create_channel("media").unwrap();
        p.subscribe("editor", "media").unwrap();
        p.publish("media", b"one".to_vec()).unwrap();
        p.publish("media", b"two".to_vec()).unwrap();
        let msgs = p.poll("editor").unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].sequence, 1)
    }
    #[test]
    fn validates_channel_and_subscriber_lifecycle() {
        let mut p = PubSub::new(1);
        assert!(p.subscribe("x", "missing").is_err());
        p.create_channel("media").unwrap();
        p.subscribe("x", "media").unwrap();
        assert!(p.subscribe("x", "media").is_err());
        p.unsubscribe("x").unwrap();
        assert!(p.poll("x").is_err())
    }
}
