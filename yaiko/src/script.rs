//! Safe script loading strategy metadata.
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptStrategy {
    Defer,
    Lazy,
    Worker,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptError {
    InvalidSource,
    InvalidIntegrity,
}
#[derive(Debug, Clone, Default)]
pub struct ScriptRegistry {
    scripts: BTreeMap<String, (ScriptStrategy, Option<String>)>,
}
impl ScriptRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(
        mut self,
        src: impl Into<String>,
        strategy: ScriptStrategy,
        integrity: Option<String>,
    ) -> Result<Self, ScriptError> {
        let src = source(src.into())?;
        if let Some(i) = &integrity {
            if i.len() < 16 || i.chars().any(|c| c.is_control() || c.is_whitespace()) {
                return Err(ScriptError::InvalidIntegrity);
            }
        }
        self.scripts.insert(src, (strategy, integrity));
        Ok(self)
    }
    pub fn render(&self) -> String {
        let mut o = String::new();
        for (src, (strategy, integrity)) in &self.scripts {
            let mode = match strategy {
                ScriptStrategy::Defer => " defer",
                ScriptStrategy::Lazy => " data-loading=\"lazy\"",
                ScriptStrategy::Worker => " type=\"text/worker\"",
            };
            let hash = integrity
                .as_ref()
                .map(|i| format!(" integrity=\"{}\" crossorigin=\"anonymous\"", esc(i)))
                .unwrap_or_default();
            o.push_str(&format!(
                "<script src=\"{}\"{}{}></script>",
                esc(src),
                mode,
                hash
            ))
        }
        o
    }
}
fn source(x: String) -> Result<String, ScriptError> {
    if x.is_empty()
        || x.chars().any(|c| c.is_control() || c.is_whitespace())
        || (!x.starts_with('/') && !x.starts_with("https://"))
    {
        Err(ScriptError::InvalidSource)
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
    fn renders_strategies_and_deduplicates() {
        let r = ScriptRegistry::new()
            .add("/app.js", ScriptStrategy::Defer, None)
            .unwrap()
            .add("/lazy.js", ScriptStrategy::Lazy, None)
            .unwrap()
            .add(
                "https://cdn.example/x.js",
                ScriptStrategy::Worker,
                Some("sha256-abcdef0123456789".into()),
            )
            .unwrap();
        let o = r.render();
        assert!(o.contains(" defer"));
        assert!(o.contains("data-loading=\"lazy\""));
        assert!(o.contains("text/worker"));
        assert!(o.contains("integrity"))
    }
    #[test]
    fn rejects_unsafe_sources_and_integrity() {
        assert!(matches!(
            ScriptRegistry::new().add("javascript:bad", ScriptStrategy::Defer, None),
            Err(ScriptError::InvalidSource)
        ));
        assert!(matches!(
            ScriptRegistry::new().add("/x.js", ScriptStrategy::Defer, Some("bad".into())),
            Err(ScriptError::InvalidIntegrity)
        ));
    }
}
