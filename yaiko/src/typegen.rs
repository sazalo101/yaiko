//! Deterministic Rust and TypeScript declaration generation.
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    Integer,
    Boolean,
    Bytes,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypegenError {
    InvalidName,
    InvalidField,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSchema {
    pub name: String,
    pub fields: BTreeMap<String, FieldType>,
}
#[derive(Debug, Clone, Default)]
pub struct TypeGenerator {
    schemas: BTreeMap<String, TypeSchema>,
    capacity: usize,
}
impl TypeGenerator {
    pub fn new(capacity: usize) -> Self {
        Self {
            schemas: BTreeMap::new(),
            capacity,
        }
    }
    pub fn schema(
        mut self,
        name: impl Into<String>,
        fields: impl IntoIterator<Item = (String, FieldType)>,
    ) -> Result<Self, TypegenError> {
        let name = name.into();
        if name.is_empty()
            || name.len() > 128
            || name.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_')
        {
            return Err(TypegenError::InvalidName);
        }
        if !self.schemas.contains_key(&name) && self.schemas.len() >= self.capacity {
            return Err(TypegenError::Capacity);
        }
        let fields: BTreeMap<_, _> = fields.into_iter().collect();
        if fields
            .keys()
            .any(|k| k.is_empty() || k.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_'))
        {
            return Err(TypegenError::InvalidField);
        }
        self.schemas
            .insert(name.clone(), TypeSchema { name, fields });
        Ok(self)
    }
    pub fn rust(&self) -> String {
        self.schemas
            .values()
            .map(|s| {
                let fields = s
                    .fields
                    .iter()
                    .map(|(n, t)| format!("    pub {n}: {},", rust_type(*t)))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("pub struct {} {{\n{}\n}}", s.name, fields)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
    pub fn typescript(&self) -> String {
        self.schemas
            .values()
            .map(|s| {
                let fields = s
                    .fields
                    .iter()
                    .map(|(n, t)| format!("  {n}: {};", ts_type(*t)))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("export interface {} {{\n{}\n}}", s.name, fields)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}
fn rust_type(t: FieldType) -> &'static str {
    match t {
        FieldType::String => "String",
        FieldType::Integer => "i64",
        FieldType::Boolean => "bool",
        FieldType::Bytes => "Vec<u8>",
    }
}
fn ts_type(t: FieldType) -> &'static str {
    match t {
        FieldType::String => "string",
        FieldType::Integer => "number",
        FieldType::Boolean => "boolean",
        FieldType::Bytes => "Uint8Array",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_stable_rust_and_typescript() {
        let g = TypeGenerator::new(2)
            .schema(
                "Media",
                [
                    ("z".into(), FieldType::Boolean),
                    ("id".into(), FieldType::String),
                ],
            )
            .unwrap();
        assert!(g.rust().contains("pub id: String"));
        assert!(g.typescript().contains("z: boolean"))
    }
    #[test]
    fn validates_names_fields_and_capacity() {
        assert!(TypeGenerator::new(1)
            .schema("bad-name", Vec::new())
            .is_err());
        let g = TypeGenerator::new(1).schema("A", [("bad-name".into(), FieldType::String)]);
        assert!(g.is_err())
    }
}
