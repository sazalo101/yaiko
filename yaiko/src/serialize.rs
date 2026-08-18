//! Deterministic JSON serialization facade.
use serde_json::Value;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerializeError {
    TooLarge,
    InvalidValue,
    Serialization,
}
#[derive(Debug, Clone)]
pub struct Serializer {
    max_bytes: usize,
}
impl Serializer {
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }
    pub fn render(&self, value: &Value) -> Result<Vec<u8>, SerializeError> {
        if contains_invalid(value) {
            return Err(SerializeError::InvalidValue);
        }
        let bytes = serde_json::to_vec(value).map_err(|_| SerializeError::Serialization)?;
        if bytes.len() > self.max_bytes {
            Err(SerializeError::TooLarge)
        } else {
            Ok(bytes)
        }
    }
    pub fn render_string(&self, value: &Value) -> Result<String, SerializeError> {
        String::from_utf8(self.render(value)?).map_err(|_| SerializeError::Serialization)
    }
}
fn contains_invalid(value: &Value) -> bool {
    match value {
        Value::Number(n) => n.as_f64().is_some_and(|x| !x.is_finite()),
        Value::Array(a) => a.iter().any(contains_invalid),
        Value::Object(o) => o.values().any(contains_invalid),
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn renders_stable_json_and_escapes_values() {
        let s = Serializer::new(128);
        assert_eq!(
            s.render_string(&json!({"z":1,"a":"<&"})).unwrap(),
            r#"{"a":"<&","z":1}"#
        )
    }
    #[test]
    fn enforces_output_bounds() {
        assert_eq!(
            Serializer::new(1).render(&json!({"x":1})),
            Err(SerializeError::TooLarge)
        );
    }
}
