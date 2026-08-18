//! Reusable form schemas backed by structured validation reports.
use crate::schema_validation::{FieldError, SchemaRule, SchemaValidator, ValidationReport};
use serde_json::Value;
#[derive(Debug, Clone, Default)]
pub struct FormSchema {
    validator: SchemaValidator,
    fields: Vec<String>,
}
impl FormSchema {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn field(
        mut self,
        name: impl Into<String>,
        rules: impl IntoIterator<Item = SchemaRule>,
    ) -> Self {
        let name = name.into();
        self.fields.push(name.clone());
        for rule in rules {
            self.validator = self.validator.rule(name.clone(), rule)
        }
        self
    }
    pub fn required(self, name: impl Into<String>) -> Self {
        self.field(name, [SchemaRule::Required])
    }
    pub fn text(self, name: impl Into<String>, min: usize, max: usize) -> Self {
        self.field(
            name,
            [SchemaRule::StringMin(min), SchemaRule::StringMax(max)],
        )
    }
    pub fn email(self, name: impl Into<String>) -> Self {
        self.field(name, [SchemaRule::Email])
    }
    pub fn validate(&self, input: &Value) -> ValidationReport {
        self.validator.validate(input)
    }
    pub fn fields(&self) -> &[String] {
        &self.fields
    }
    pub fn invalid_fields(&self, input: &Value) -> Vec<String> {
        let mut fields = self
            .validate(input)
            .errors
            .into_iter()
            .map(|e: FieldError| e.field)
            .collect::<Vec<_>>();
        fields.sort();
        fields.dedup();
        fields
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn validates_form_fields_and_reports_invalid_names() {
        let form = FormSchema::new()
            .required("name")
            .text("name", 2, 40)
            .email("email");
        let report = form.validate(&json!({"name":"A","email":"bad"}));
        assert!(!report.valid);
        assert_eq!(
            form.invalid_fields(&json!({"name":"A","email":"bad"})),
            vec!["email", "name"]
        );
        assert_eq!(form.fields(), ["name", "name", "email"])
    }
    #[test]
    fn accepts_valid_submission() {
        let form = FormSchema::new().required("name").email("email");
        assert!(
            form.validate(&json!({"name":"Ada","email":"ada@example.com"}))
                .valid
        )
    }
}
