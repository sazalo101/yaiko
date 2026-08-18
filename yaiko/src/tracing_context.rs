//! Lightweight distributed tracing context and span primitives.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub sampled: bool,
}

impl TraceContext {
    pub fn new(sampled: bool) -> Self {
        Self {
            trace_id: Uuid::new_v4().simple().to_string(),
            span_id: new_span_id(),
            parent_span_id: None,
            sampled,
        }
    }

    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: new_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            sampled: self.sampled,
        }
    }

    pub fn to_headers(&self) -> BTreeMap<String, String> {
        BTreeMap::from([(
            "traceparent".to_string(),
            format!(
                "00-{}-{}-{:02x}",
                self.trace_id,
                self.span_id,
                if self.sampled { 1 } else { 0 }
            ),
        )])
    }

    pub fn from_headers(headers: &BTreeMap<String, String>) -> Option<Self> {
        let value = headers.get("traceparent")?;
        let parts = value.split('-').collect::<Vec<_>>();
        if parts.len() != 4
            || parts[0] != "00"
            || parts[1].len() != 32
            || parts[2].len() != 16
            || !parts[3].eq("01") && !parts[3].eq("00")
        {
            return None;
        }
        Some(Self {
            trace_id: parts[1].to_string(),
            span_id: parts[2].to_string(),
            parent_span_id: None,
            sampled: parts[3] == "01",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanEvent {
    pub name: String,
    pub attributes: BTreeMap<String, String>,
    pub timestamp_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub name: String,
    pub context: TraceContext,
    pub attributes: BTreeMap<String, String>,
    pub events: Vec<SpanEvent>,
    pub ended: bool,
}

impl Span {
    pub fn new(name: impl Into<String>, context: TraceContext) -> Self {
        Self {
            name: name.into(),
            context,
            attributes: BTreeMap::new(),
            events: Vec::new(),
            ended: false,
        }
    }
    pub fn attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
    pub fn event(&mut self, name: impl Into<String>, attributes: BTreeMap<String, String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            attributes,
            timestamp_ms: now_ms(),
        });
    }
    pub fn end(&mut self) {
        self.ended = true;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sampler {
    pub probability: f64,
}

impl Sampler {
    pub fn new(probability: f64) -> Self {
        Self {
            probability: probability.clamp(0.0, 1.0),
        }
    }
    pub fn should_sample(&self, entropy: u64) -> bool {
        (entropy as f64 / u64::MAX as f64) < self.probability
    }
}

fn new_span_id() -> String {
    Uuid::new_v4().simple().to_string()[..16].to_string()
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propagates_and_parses_traceparent_context() {
        let context = TraceContext::new(true);
        let parsed = TraceContext::from_headers(&context.to_headers()).unwrap();
        assert_eq!(parsed.trace_id, context.trace_id);
        assert_eq!(parsed.span_id, context.span_id);
        assert!(parsed.sampled);
    }

    #[test]
    fn child_span_preserves_trace_and_links_parent() {
        let parent = TraceContext::new(true);
        let child = parent.child();
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(
            child.parent_span_id.as_deref(),
            Some(parent.span_id.as_str())
        );
        assert_ne!(child.span_id, parent.span_id);
    }

    #[test]
    fn span_records_attributes_events_and_end_state() {
        let mut span =
            Span::new("upload", TraceContext::new(true)).attribute("component", "storage");
        span.event("checksum_verified", BTreeMap::new());
        span.end();
        assert_eq!(span.attributes["component"], "storage");
        assert_eq!(span.events[0].name, "checksum_verified");
        assert!(span.ended);
    }

    #[test]
    fn sampler_clamps_probability_and_is_deterministic_for_entropy() {
        assert!(Sampler::new(1.5).should_sample(0));
        assert!(!Sampler::new(-1.0).should_sample(u64::MAX));
        assert!(Sampler::new(0.5).should_sample(0));
    }
}
