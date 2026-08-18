//! Lightweight typed metrics registry with bounded label cardinality.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram { count: u64, sum: f64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricError {
    InvalidName,
    InvalidLabel,
    UnknownMetric,
    WrongKind,
    CardinalityLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    Counter,
    Gauge,
    Histogram,
}

#[derive(Debug, Clone)]
struct Definition {
    kind: MetricKind,
    labels: BTreeSet<String>,
    max_series: usize,
}

#[derive(Debug, Clone, Default)]
pub struct MetricsSnapshot {
    pub values: BTreeMap<String, MetricValue>,
}

#[derive(Clone, Default)]
pub struct MetricsRegistry {
    inner: Arc<Mutex<RegistryState>>,
}

#[derive(Default)]
struct RegistryState {
    definitions: BTreeMap<String, Definition>,
    values: BTreeMap<String, MetricValue>,
    series: BTreeMap<String, BTreeSet<String>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn define(
        &self,
        name: impl Into<String>,
        kind: MetricKind,
        labels: &[&str],
        max_series: usize,
    ) -> Result<(), MetricError> {
        let name = name.into();
        if !valid_name(&name) {
            return Err(MetricError::InvalidName);
        }
        let labels = labels
            .iter()
            .map(|label| (*label).to_string())
            .collect::<BTreeSet<_>>();
        if labels.iter().any(|label| !valid_name(label)) {
            return Err(MetricError::InvalidLabel);
        }
        self.inner
            .lock()
            .expect("metrics registry poisoned")
            .definitions
            .insert(
                name,
                Definition {
                    kind,
                    labels,
                    max_series,
                },
            );
        Ok(())
    }
    pub fn increment(
        &self,
        name: &str,
        labels: &BTreeMap<String, String>,
        amount: u64,
    ) -> Result<(), MetricError> {
        self.update(
            name,
            labels,
            MetricValue::Counter(amount),
            |current, value| {
                if let MetricValue::Counter(existing) = current {
                    if let MetricValue::Counter(incoming) = value {
                        *existing = existing.saturating_add(incoming);
                        return Ok(());
                    }
                }
                Err(MetricError::WrongKind)
            },
        )
    }
    pub fn set_gauge(
        &self,
        name: &str,
        labels: &BTreeMap<String, String>,
        value: f64,
    ) -> Result<(), MetricError> {
        self.update(name, labels, MetricValue::Gauge(value), |current, value| {
            if let (MetricValue::Gauge(existing), MetricValue::Gauge(incoming)) = (current, value) {
                *existing = incoming;
                Ok(())
            } else {
                Err(MetricError::WrongKind)
            }
        })
    }
    pub fn observe(
        &self,
        name: &str,
        labels: &BTreeMap<String, String>,
        value: f64,
    ) -> Result<(), MetricError> {
        self.update(
            name,
            labels,
            MetricValue::Histogram {
                count: 1,
                sum: value,
            },
            |current, value| {
                if let (
                    MetricValue::Histogram { count, sum },
                    MetricValue::Histogram {
                        count: incoming_count,
                        sum: incoming_sum,
                    },
                ) = (current, value)
                {
                    *count = count.saturating_add(incoming_count);
                    *sum += incoming_sum;
                    Ok(())
                } else {
                    Err(MetricError::WrongKind)
                }
            },
        )
    }
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            values: self
                .inner
                .lock()
                .expect("metrics registry poisoned")
                .values
                .clone(),
        }
    }
    fn update<F>(
        &self,
        name: &str,
        labels: &BTreeMap<String, String>,
        incoming: MetricValue,
        merge: F,
    ) -> Result<(), MetricError>
    where
        F: FnOnce(&mut MetricValue, MetricValue) -> Result<(), MetricError>,
    {
        let mut state = self.inner.lock().expect("metrics registry poisoned");
        let definition = state
            .definitions
            .get(name)
            .ok_or(MetricError::UnknownMetric)?
            .clone();
        let incoming_kind = match &incoming {
            MetricValue::Counter(_) => MetricKind::Counter,
            MetricValue::Gauge(_) => MetricKind::Gauge,
            MetricValue::Histogram { .. } => MetricKind::Histogram,
        };
        if definition.kind != incoming_kind {
            return Err(MetricError::WrongKind);
        }
        if definition.labels != labels.keys().cloned().collect() {
            return Err(MetricError::InvalidLabel);
        }
        let series_key = labels
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let series = state.series.entry(name.to_string()).or_default();
        if !series.contains(&series_key) && series.len() >= definition.max_series {
            return Err(MetricError::CardinalityLimit);
        }
        series.insert(series_key.clone());
        let key = format!("{name}{{{series_key}}}");
        if let Some(current) = state.values.get_mut(&key) {
            merge(current, incoming)
        } else {
            state.values.insert(key, incoming);
            Ok(())
        }
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    fn labels(value: &str) -> BTreeMap<String, String> {
        BTreeMap::from([(String::from("route"), value.into())])
    }

    #[test]
    fn updates_typed_metrics_and_exports_snapshot() {
        let registry = MetricsRegistry::new();
        registry
            .define("requests", MetricKind::Counter, &["route"], 2)
            .unwrap();
        registry
            .define("temperature", MetricKind::Gauge, &[], 1)
            .unwrap();
        registry
            .define("latency", MetricKind::Histogram, &[], 1)
            .unwrap();
        registry.increment("requests", &labels("/"), 2).unwrap();
        registry
            .set_gauge("temperature", &BTreeMap::new(), 21.5)
            .unwrap();
        registry.observe("latency", &BTreeMap::new(), 0.5).unwrap();
        assert_eq!(registry.snapshot().values.len(), 3);
    }

    #[test]
    fn enforces_labels_and_cardinality() {
        let registry = MetricsRegistry::new();
        registry
            .define("requests", MetricKind::Counter, &["route"], 1)
            .unwrap();
        assert!(registry.increment("requests", &labels("/"), 1).is_ok());
        assert_eq!(
            registry.increment("requests", &labels("/other"), 1),
            Err(MetricError::CardinalityLimit)
        );
        assert_eq!(
            registry.increment("requests", &BTreeMap::new(), 1),
            Err(MetricError::InvalidLabel)
        );
    }
}
