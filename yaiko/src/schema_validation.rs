//! Typed JSON schema validation with nested paths and structured field errors.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldError {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub valid: bool,
    pub errors: Vec<FieldError>,
}

impl ValidationReport {
    pub fn ok() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }
    pub fn from_errors(errors: Vec<FieldError>) -> Self {
        Self {
            valid: errors.is_empty(),
            errors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaRule {
    Required,
    StringMin(usize),
    StringMax(usize),
    Email,
    Integer,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaValidator {
    rules: BTreeMap<String, Vec<SchemaRule>>,
}

impl SchemaValidator {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn rule(mut self, field: impl Into<String>, rule: SchemaRule) -> Self {
        self.rules.entry(field.into()).or_default().push(rule);
        self
    }
    pub fn required(self, field: impl Into<String>) -> Self {
        self.rule(field, SchemaRule::Required)
    }
    pub fn string_min(self, field: impl Into<String>, length: usize) -> Self {
        self.rule(field, SchemaRule::StringMin(length))
    }
    pub fn string_max(self, field: impl Into<String>, length: usize) -> Self {
        self.rule(field, SchemaRule::StringMax(length))
    }
    pub fn email(self, field: impl Into<String>) -> Self {
        self.rule(field, SchemaRule::Email)
    }
    pub fn integer(self, field: impl Into<String>) -> Self {
        self.rule(field, SchemaRule::Integer)
    }

    pub fn validate(&self, input: &Value) -> ValidationReport {
        let mut errors = Vec::new();
        for (field, rules) in &self.rules {
            let value = field_value(input, field);
            for rule in rules {
                if let Some(error) = validate_rule(field, value, rule) {
                    errors.push(error);
                }
            }
        }
        ValidationReport::from_errors(errors)
    }
}

fn field_value<'a>(input: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = input;
    for part in path.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

fn validate_rule(field: &str, value: Option<&Value>, rule: &SchemaRule) -> Option<FieldError> {
    let missing = value.is_none() || value.is_some_and(Value::is_null);
    let error = |code: &str, message: String| FieldError {
        field: field.to_string(),
        code: code.to_string(),
        message,
    };
    match rule {
        SchemaRule::Required if missing => Some(error("required", "Field is required".into())),
        SchemaRule::StringMin(min)
            if value
                .and_then(Value::as_str)
                .is_some_and(|text| text.chars().count() < *min) =>
        {
            Some(error(
                "min_length",
                format!("Must be at least {min} characters"),
            ))
        }
        SchemaRule::StringMax(max)
            if value
                .and_then(Value::as_str)
                .is_some_and(|text| text.chars().count() > *max) =>
        {
            Some(error(
                "max_length",
                format!("Must be no more than {max} characters"),
            ))
        }
        SchemaRule::Email
            if value
                .and_then(Value::as_str)
                .is_some_and(|text| !text.contains('@') || !text.contains('.')) =>
        {
            Some(error("email", "Must be a valid email address".into()))
        }
        SchemaRule::Integer if value.is_some_and(|value| !value.is_i64() && !value.is_u64()) => {
            Some(error("integer", "Must be an integer".into()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_nested_fields_and_collects_structured_errors() {
        let validator = SchemaValidator::new()
            .required("user.name")
            .email("user.email")
            .integer("age");
        let report = validator.validate(&json!({"user": {"email": "invalid"}, "age": "old"}));
        assert!(!report.valid);
        assert!(report
            .errors
            .iter()
            .any(|error| error.field == "user.name" && error.code == "required"));
        assert!(report
            .errors
            .iter()
            .any(|error| error.field == "user.email" && error.code == "email"));
        assert!(report
            .errors
            .iter()
            .any(|error| error.field == "age" && error.code == "integer"));
    }

    #[test]
    fn accepts_valid_values_and_enforces_string_bounds() {
        let validator = SchemaValidator::new()
            .required("name")
            .string_min("name", 2)
            .string_max("name", 5);
        assert!(validator.validate(&json!({"name": "Ada"})).valid);
        assert_eq!(
            validator.validate(&json!({"name": "A"})).errors[0].code,
            "min_length"
        );
    }

    #[test]
    fn serializes_validation_report_for_http_responses() {
        let report = SchemaValidator::new()
            .required("email")
            .validate(&json!({}));
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["valid"], false);
        assert_eq!(json["errors"][0]["field"], "email");
    }
}
