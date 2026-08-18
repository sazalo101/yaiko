//! Provider-neutral notification and email-delivery primitives.

use handlebars::Handlebars;
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationEnvelope {
    pub to: String,
    pub from: Option<String>,
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationTemplate {
    pub subject: String,
    pub text: String,
    pub html: Option<String>,
}

impl NotificationTemplate {
    pub fn render<T: Serialize>(
        &self,
        data: &T,
    ) -> Result<(String, String, Option<String>), TemplateError> {
        let registry = Handlebars::new();
        let data = serde_json::to_value(data)
            .map_err(|error| TemplateError::Serialization(error.to_string()))?;
        let subject = registry
            .render_template(&self.subject, &data)
            .map_err(|error| TemplateError::Render(error.to_string()))?;
        let text = registry
            .render_template(&self.text, &data)
            .map_err(|error| TemplateError::Render(error.to_string()))?;
        let html = self
            .html
            .as_ref()
            .map(|template| {
                registry
                    .render_template(template, &data)
                    .map_err(|error| TemplateError::Render(error.to_string()))
            })
            .transpose()?;
        Ok((subject, text, html))
    }

    pub fn envelope<T: Serialize>(
        &self,
        to: impl Into<String>,
        data: &T,
    ) -> Result<NotificationEnvelope, TemplateError> {
        let (subject, text, html) = self.render(data)?;
        Ok(NotificationEnvelope {
            to: to.into(),
            from: None,
            subject,
            text,
            html,
            idempotency_key: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateError {
    Serialization(String),
    Render(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}

impl RetryPolicy {
    pub fn next_delay(&self, attempt: u32) -> Option<Duration> {
        if attempt >= self.max_attempts {
            return None;
        }
        let multiplier = 2u32.saturating_pow(attempt);
        Some(
            self.initial_delay
                .saturating_mul(multiplier)
                .min(self.max_delay),
        )
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_secs(5),
            max_delay: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryResult {
    Delivered {
        provider_id: String,
    },
    RetryableFailure {
        reason: String,
        retry_after: Duration,
    },
    PermanentFailure {
        reason: String,
    },
}

impl DeliveryResult {
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Delivered { .. })
    }
    pub fn retry_after(&self) -> Option<Duration> {
        if let Self::RetryableFailure { retry_after, .. } = self {
            Some(*retry_after)
        } else {
            None
        }
    }
}

impl From<Value> for NotificationEnvelope {
    fn from(value: Value) -> Self {
        let object = value.as_object().cloned().unwrap_or_default();
        Self {
            to: object
                .get("to")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            from: object.get("from").and_then(Value::as_str).map(String::from),
            subject: object
                .get("subject")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            text: object
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            html: object.get("html").and_then(Value::as_str).map(String::from),
            idempotency_key: object
                .get("idempotency_key")
                .and_then(Value::as_str)
                .map(String::from),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_subject_text_and_html_templates() {
        let template = NotificationTemplate {
            subject: "Hello {{name}}".into(),
            text: "Welcome, {{name}}".into(),
            html: Some("<strong>{{name}}</strong>".into()),
        };
        let envelope = template
            .envelope("user@example.com", &json!({"name": "Ada"}))
            .unwrap();
        assert_eq!(envelope.subject, "Hello Ada");
        assert_eq!(envelope.text, "Welcome, Ada");
        assert_eq!(envelope.html.as_deref(), Some("<strong>Ada</strong>"));
    }

    #[test]
    fn retry_policy_exponentially_backoffs_and_caps() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_secs(2),
            max_delay: Duration::from_secs(5),
        };
        assert_eq!(policy.next_delay(0), Some(Duration::from_secs(2)));
        assert_eq!(policy.next_delay(1), Some(Duration::from_secs(4)));
        assert_eq!(policy.next_delay(2), Some(Duration::from_secs(5)));
        assert_eq!(policy.next_delay(3), None);
    }

    #[test]
    fn delivery_results_normalize_success_and_failures() {
        let success = DeliveryResult::Delivered {
            provider_id: "msg-1".into(),
        };
        let retry = DeliveryResult::RetryableFailure {
            reason: "timeout".into(),
            retry_after: Duration::from_secs(10),
        };
        assert!(success.is_success());
        assert!(!retry.is_success());
        assert_eq!(retry.retry_after(), Some(Duration::from_secs(10)));
    }
}
