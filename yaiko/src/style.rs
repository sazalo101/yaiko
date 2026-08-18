//! Validated scoped CSS generation.
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleError {
    InvalidSelector,
    InvalidProperty,
    InvalidValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyleSheet {
    scope: String,
    rules: BTreeMap<String, BTreeMap<String, String>>,
}
impl StyleSheet {
    pub fn new(scope: impl Into<String>) -> Result<Self, StyleError> {
        let scope = valid(scope.into(), StyleError::InvalidSelector)?;
        Ok(Self {
            scope,
            rules: BTreeMap::new(),
        })
    }
    pub fn rule(
        mut self,
        selector: impl Into<String>,
        property: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, StyleError> {
        let selector = valid(selector.into(), StyleError::InvalidSelector)?;
        if selector.chars().any(|c| c == ';' || c == '{') {
            return Err(StyleError::InvalidSelector);
        }
        let property = valid(property.into(), StyleError::InvalidProperty)?;
        if property.chars().any(|c| c == ';' || c == '{' || c == '}') {
            return Err(StyleError::InvalidProperty);
        }
        let value = valid(value.into(), StyleError::InvalidValue)?;
        if value.chars().any(|c| c == ';' || c == '{' || c == '}') {
            return Err(StyleError::InvalidValue);
        }
        self.rules
            .entry(selector)
            .or_default()
            .insert(property, value);
        Ok(self)
    }
    pub fn render(&self) -> String {
        let mut out = format!("<style data-scope=\"{}\">", esc(&self.scope));
        for (selector, props) in &self.rules {
            out.push_str(&format!(
                "{}[data-scope=\"{}\"]{{",
                selector,
                esc(&self.scope)
            ));
            for (property, value) in props {
                out.push_str(&format!("{}:{};", property, value))
            }
            out.push('}')
        }
        out.push_str("</style>");
        out
    }
}
fn valid(x: String, e: StyleError) -> Result<String, StyleError> {
    if x.is_empty() || x.len() > 256 || x.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(e)
    } else {
        Ok(x)
    }
}
fn esc(x: &str) -> String {
    x.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_scoped_sorted_rules() {
        let s = StyleSheet::new("card")
            .unwrap()
            .rule(".title", "color", "red")
            .unwrap()
            .rule(".title", "font-size", "1rem")
            .unwrap();
        let o = s.render();
        assert!(o.contains("data-scope=\"card\""));
        assert!(o.contains("font-size:1rem;"))
    }
    #[test]
    fn rejects_css_injection() {
        assert!(matches!(
            StyleSheet::new("x")
                .unwrap()
                .rule(".x", "color", "red;body{}"),
            Err(StyleError::InvalidValue)
        ));
        assert!(StyleSheet::new("bad scope").is_err())
    }
}
