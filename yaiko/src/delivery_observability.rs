//! Structured observability for outbound delivery outcomes.

use crate::{MetricKind, MetricsRegistry};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Succeeded,
    Retried,
    Failed,
    DeadLettered,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryObservation {
    pub delivery_id: String,
    pub destination_kind: String,
    pub outcome: DeliveryOutcome,
    pub attempt: u32,
    pub error_code: Option<String>,
}

#[derive(Clone)]
pub struct DeliveryObserver {
    events: Arc<Mutex<Vec<DeliveryObservation>>>,
    capacity: usize,
    metrics: MetricsRegistry,
}

impl DeliveryObserver {
    pub fn new(capacity: usize) -> Self {
        let metrics = MetricsRegistry::new();
        let _ = metrics.define(
            "delivery_outcomes",
            MetricKind::Counter,
            &["kind", "outcome"],
            capacity.max(1),
        );
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            capacity,
            metrics,
        }
    }
    pub fn record(&self, observation: DeliveryObservation) {
        let outcome = match observation.outcome {
            DeliveryOutcome::Succeeded => "succeeded",
            DeliveryOutcome::Retried => "retried",
            DeliveryOutcome::Failed => "failed",
            DeliveryOutcome::DeadLettered => "dead_lettered",
        };
        let labels = BTreeMap::from([
            (String::from("kind"), observation.destination_kind.clone()),
            (String::from("outcome"), outcome.to_string()),
        ]);
        let _ = self.metrics.increment("delivery_outcomes", &labels, 1);
        let mut events = self.events.lock().expect("delivery observer poisoned");
        if self.capacity == 0 {
            return;
        }
        if events.len() >= self.capacity {
            events.remove(0);
        }
        events.push(observation);
    }
    pub fn events(&self) -> Vec<DeliveryObservation> {
        self.events
            .lock()
            .expect("delivery observer poisoned")
            .clone()
    }
    pub fn metrics(&self) -> crate::MetricsSnapshot {
        self.metrics.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(outcome: DeliveryOutcome) -> DeliveryObservation {
        DeliveryObservation {
            delivery_id: "d1".into(),
            destination_kind: "webhook".into(),
            outcome,
            attempt: 1,
            error_code: None,
        }
    }

    #[test]
    fn records_outcomes_and_bounded_events() {
        let observer = DeliveryObserver::new(2);
        observer.record(observation(DeliveryOutcome::Succeeded));
        observer.record(observation(DeliveryOutcome::Retried));
        observer.record(observation(DeliveryOutcome::DeadLettered));
        assert_eq!(observer.events().len(), 2);
        assert_eq!(observer.events()[0].outcome, DeliveryOutcome::Retried);
        assert!(!observer.metrics().values.is_empty());
    }

    #[test]
    fn preserves_failure_metadata() {
        let observer = DeliveryObserver::new(4);
        let mut failed = observation(DeliveryOutcome::Failed);
        failed.attempt = 3;
        failed.error_code = Some("timeout".into());
        observer.record(failed);
        assert_eq!(observer.events()[0].error_code.as_deref(), Some("timeout"));
        assert_eq!(observer.events()[0].attempt, 3);
    }
}
