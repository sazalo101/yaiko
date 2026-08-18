//! Privacy and data-protection helpers for logs, exports, and API diagnostics.

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyPolicy {
    sensitive_fields: BTreeSet<String>,
    replacement: String,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyPolicy {
    pub fn new() -> Self {
        Self {
            sensitive_fields: [
                "password",
                "secret",
                "token",
                "api_key",
                "apikey",
                "authorization",
                "cookie",
                "client_secret",
                "refresh_token",
                "access_token",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            replacement: "[REDACTED]".to_string(),
        }
    }

    pub fn sensitive_field(mut self, field: impl Into<String>) -> Self {
        self.sensitive_fields
            .insert(field.into().to_ascii_lowercase());
        self
    }

    pub fn replacement(mut self, replacement: impl Into<String>) -> Self {
        self.replacement = replacement.into();
        self
    }

    pub fn redact_map(&self, values: &BTreeMap<String, String>) -> BTreeMap<String, String> {
        values
            .iter()
            .map(|(key, value)| {
                (
                    key.clone(),
                    if self.is_sensitive(key) {
                        self.replacement.clone()
                    } else {
                        value.clone()
                    },
                )
            })
            .collect()
    }

    pub fn redact_json(&self, value: &Value) -> Value {
        match value {
            Value::Object(object) => {
                let mut output = Map::new();
                for (key, child) in object {
                    output.insert(
                        key.clone(),
                        if self.is_sensitive(key) {
                            Value::String(self.replacement.clone())
                        } else {
                            self.redact_json(child)
                        },
                    );
                }
                Value::Object(output)
            }
            Value::Array(values) => {
                Value::Array(values.iter().map(|value| self.redact_json(value)).collect())
            }
            other => other.clone(),
        }
    }

    fn is_sensitive(&self, field: &str) -> bool {
        let normalized = field.to_ascii_lowercase();
        self.sensitive_fields
            .iter()
            .any(|sensitive| normalized == *sensitive || normalized.contains(sensitive))
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

impl fmt::Display for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn recursively_redacts_sensitive_json_fields() {
        let policy = PrivacyPolicy::new().replacement("<hidden>");
        let input = json!({"user": {"email": "a@example.com", "password": "secret"}, "items": [{"access_token": "abc"}]});
        assert_eq!(policy.redact_json(&input)["user"]["email"], "a@example.com");
        assert_eq!(policy.redact_json(&input)["user"]["password"], "<hidden>");
        assert_eq!(
            policy.redact_json(&input)["items"][0]["access_token"],
            "<hidden>"
        );
    }

    #[test]
    fn secret_never_leaks_through_debug_display_or_serialization() {
        let secret = SecretString::new("super-secret");
        assert_eq!(secret.expose(), "super-secret");
        assert_eq!(format!("{secret}"), "[REDACTED]");
        assert!(!format!("{secret:?}").contains("super-secret"));
        assert_eq!(serde_json::to_string(&secret).unwrap(), "\"[REDACTED]\"");
    }

    #[test]
    fn custom_fields_and_maps_are_redacted() {
        let policy = PrivacyPolicy::new().sensitive_field("phone");
        let values = BTreeMap::from([
            ("phone".to_string(), "555".to_string()),
            ("city".to_string(), "Lagos".to_string()),
        ]);
        let redacted = policy.redact_map(&values);
        assert_eq!(redacted["phone"], "[REDACTED]");
        assert_eq!(redacted["city"], "Lagos");
    }
}
