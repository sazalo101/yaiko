//! Input validation system for Yaiko applications.
//!
//! Provides a declarative way to validate user input and structured data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// An error that occurred during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// The result of a validation operation
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// A rule for validating a value
pub trait Rule: Send + Sync {
    fn validate(&self, field: &str, value: &str) -> Option<ValidationError>;
}

/// Minimum length rule
pub struct MinLength(pub usize);

impl Rule for MinLength {
    fn validate(&self, field: &str, value: &str) -> Option<ValidationError> {
        if value.chars().count() < self.0 {
            Some(ValidationError {
                field: field.to_string(),
                message: format!("Must be at least {} characters long", self.0),
            })
        } else {
            None
        }
    }
}

/// Maximum length rule
pub struct MaxLength(pub usize);

impl Rule for MaxLength {
    fn validate(&self, field: &str, value: &str) -> Option<ValidationError> {
        if value.chars().count() > self.0 {
            Some(ValidationError {
                field: field.to_string(),
                message: format!("Must be no more than {} characters long", self.0),
            })
        } else {
            None
        }
    }
}

/// Email format rule
pub struct Email;

impl Rule for Email {
    fn validate(&self, field: &str, value: &str) -> Option<ValidationError> {
        if !value.contains('@') || !value.contains('.') {
            Some(ValidationError {
                field: field.to_string(),
                message: "Must be a valid email address".to_string(),
            })
        } else {
            None
        }
    }
}

/// Required field rule
pub struct Required;

impl Rule for Required {
    fn validate(&self, field: &str, value: &str) -> Option<ValidationError> {
        if value.trim().is_empty() {
            Some(ValidationError {
                field: field.to_string(),
                message: "Field is required".to_string(),
            })
        } else {
            None
        }
    }
}

/// A collection of rules to validate a single field
pub struct Validator {
    rules: HashMap<String, Vec<Box<dyn Rule>>>,
}

impl Validator {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Add a rule for a specific field
    pub fn add_rule<R: Rule + 'static>(mut self, field: &str, rule: R) -> Self {
        self.rules
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(rule));
        self
    }

    /// Validate a map of field values
    pub fn validate(&self, data: &HashMap<String, String>) -> ValidationResult {
        let mut errors = Vec::new();

        for (field, rules) in &self.rules {
            let value = data.get(field).map(|s| s.as_str()).unwrap_or("");
            for rule in rules {
                if let Some(err) = rule.validate(field, value) {
                    errors.push(err);
                    // Usually we want to gather all errors, but sometimes breaking early on first error per field is preferred.
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}