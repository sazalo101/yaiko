//! Structured observability for scheduled task outcomes.

use crate::{MetricKind, MetricsRegistry};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutcome {
    Scheduled,
    Claimed,
    Completed,
    Retried,
    Misfired,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskObservation {
    pub task_id: String,
    pub task_name: String,
    pub outcome: TaskOutcome,
    pub attempt: u32,
    pub correlation_id: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Clone)]
pub struct TaskObserver {
    events: Arc<Mutex<Vec<TaskObservation>>>,
    capacity: usize,
    metrics: MetricsRegistry,
}

impl TaskObserver {
    pub fn new(capacity: usize) -> Self {
        let metrics = MetricsRegistry::new();
        let _ = metrics.define(
            "task_outcomes",
            MetricKind::Counter,
            &["task", "outcome"],
            capacity.max(1),
        );
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            capacity,
            metrics,
        }
    }
    pub fn record(&self, observation: TaskObservation) {
        let outcome = match observation.outcome {
            TaskOutcome::Scheduled => "scheduled",
            TaskOutcome::Claimed => "claimed",
            TaskOutcome::Completed => "completed",
            TaskOutcome::Retried => "retried",
            TaskOutcome::Misfired => "misfired",
            TaskOutcome::Cancelled => "cancelled",
            TaskOutcome::Failed => "failed",
        };
        let labels = BTreeMap::from([
            (String::from("task"), observation.task_name.clone()),
            (String::from("outcome"), outcome.to_string()),
        ]);
        let _ = self.metrics.increment("task_outcomes", &labels, 1);
        let mut events = self.events.lock().expect("task observer poisoned");
        if self.capacity == 0 {
            return;
        }
        if events.len() >= self.capacity {
            events.remove(0);
        }
        events.push(observation);
    }
    pub fn events(&self) -> Vec<TaskObservation> {
        self.events.lock().expect("task observer poisoned").clone()
    }
    pub fn metrics(&self) -> crate::MetricsSnapshot {
        self.metrics.snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(outcome: TaskOutcome) -> TaskObservation {
        TaskObservation {
            task_id: "t1".into(),
            task_name: "render".into(),
            outcome,
            attempt: 1,
            correlation_id: Some("req-1".into()),
            error_code: None,
        }
    }

    #[test]
    fn records_bounded_task_history_and_metrics() {
        let observer = TaskObserver::new(2);
        observer.record(observation(TaskOutcome::Scheduled));
        observer.record(observation(TaskOutcome::Completed));
        observer.record(observation(TaskOutcome::Misfired));
        assert_eq!(observer.events().len(), 2);
        assert_eq!(observer.events()[0].outcome, TaskOutcome::Completed);
        assert!(!observer.metrics().values.is_empty());
    }

    #[test]
    fn preserves_retry_and_correlation_metadata() {
        let observer = TaskObserver::new(3);
        let mut retry = observation(TaskOutcome::Retried);
        retry.attempt = 3;
        retry.error_code = Some("timeout".into());
        observer.record(retry);
        assert_eq!(observer.events()[0].attempt, 3);
        assert_eq!(
            observer.events()[0].correlation_id.as_deref(),
            Some("req-1")
        );
        assert_eq!(observer.events()[0].error_code.as_deref(), Some("timeout"));
    }
}
