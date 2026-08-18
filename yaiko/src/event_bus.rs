//! Typed in-process event bus with bounded deterministic history.
use std::collections::VecDeque;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventBusError {
    InvalidScope,
    InvalidTopic,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub scope: String,
    pub topic: String,
    pub payload: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscription {
    pub scope: String,
    pub topic: Option<String>,
    pub cursor: u64,
}
#[derive(Debug, Clone)]
pub struct EventBus {
    history: VecDeque<Event>,
    capacity: usize,
    next_sequence: u64,
}
impl EventBus {
    pub fn new(capacity: usize) -> Self {
        Self {
            history: VecDeque::new(),
            capacity,
            next_sequence: 0,
        }
    }
    pub fn subscribe(
        &self,
        scope: impl Into<String>,
        topic: Option<String>,
    ) -> Result<Subscription, EventBusError> {
        let scope = valid(scope.into(), EventBusError::InvalidScope)?;
        if let Some(t) = &topic {
            valid(t.clone(), EventBusError::InvalidTopic)?;
        }
        Ok(Subscription {
            scope,
            topic,
            cursor: self.next_sequence,
        })
    }
    pub fn publish(
        &mut self,
        scope: impl Into<String>,
        topic: impl Into<String>,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Event, EventBusError> {
        let scope = valid(scope.into(), EventBusError::InvalidScope)?;
        let topic = valid(topic.into(), EventBusError::InvalidTopic)?;
        if self.capacity == 0 {
            return Err(EventBusError::Capacity);
        }
        self.next_sequence += 1;
        let event = Event {
            sequence: self.next_sequence,
            scope,
            topic,
            payload: payload.into(),
        };
        if self.history.len() >= self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(event.clone());
        Ok(event)
    }
    pub fn poll(&self, sub: &mut Subscription) -> Vec<Event> {
        let out = self
            .history
            .iter()
            .filter(|e| {
                e.sequence > sub.cursor
                    && e.scope == sub.scope
                    && sub.topic.as_ref().is_none_or(|t| t == &e.topic)
            })
            .cloned()
            .collect::<Vec<_>>();
        if let Some(last) = out.last() {
            sub.cursor = last.sequence
        }
        out
    }
    pub fn snapshot(&self) -> Vec<Event> {
        self.history.iter().cloned().collect()
    }
}
impl Default for EventBus {
    fn default() -> Self {
        Self::new(1024)
    }
}
fn valid(x: String, e: EventBusError) -> Result<String, EventBusError> {
    if x.is_empty() || x.len() > 128 || x.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(e)
    } else {
        Ok(x)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publishes_filters_and_orders_events() {
        let mut b = EventBus::new(4);
        let mut s = b.subscribe("project", Some("media".into())).unwrap();
        b.publish("project", "other", b"x".to_vec()).unwrap();
        b.publish("project", "media", b"y".to_vec()).unwrap();
        b.publish("other", "media", b"z".to_vec()).unwrap();
        let e = b.poll(&mut s);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].sequence, 2)
    }
    #[test]
    fn bounds_history_and_validates_inputs() {
        let mut b = EventBus::new(1);
        b.publish("x", "a", Vec::new()).unwrap();
        b.publish("x", "b", Vec::new()).unwrap();
        assert_eq!(b.snapshot()[0].topic, "b");
        assert!(b.subscribe("bad scope", None).is_err());
    }
}
