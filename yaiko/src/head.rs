//! Deterministic and safe HTML head aggregation.
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeadError {
    InvalidValue,
}
#[derive(Debug, Clone, Default)]
pub struct Head {
    title: Option<String>,
    meta: BTreeMap<String, String>,
    links: BTreeMap<String, String>,
    scripts: BTreeMap<String, String>,
    styles: BTreeMap<String, String>,
}
impl Head {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn title(mut self, x: impl Into<String>) -> Result<Self, HeadError> {
        self.title = Some(valid(x.into())?);
        Ok(self)
    }
    pub fn meta(
        mut self,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<Self, HeadError> {
        self.meta
            .insert(valid(name.into())?, valid(content.into())?);
        Ok(self)
    }
    pub fn link(
        mut self,
        rel: impl Into<String>,
        href: impl Into<String>,
    ) -> Result<Self, HeadError> {
        self.links.insert(valid(rel.into())?, url(href.into())?);
        Ok(self)
    }
    pub fn script(mut self, src: impl Into<String>, defer: bool) -> Result<Self, HeadError> {
        self.scripts.insert(
            url(src.into())?,
            if defer { "defer".into() } else { "".into() },
        );
        Ok(self)
    }
    pub fn style(
        mut self,
        key: impl Into<String>,
        css: impl Into<String>,
    ) -> Result<Self, HeadError> {
        self.styles.insert(valid(key.into())?, valid(css.into())?);
        Ok(self)
    }
    pub fn render(&self) -> String {
        let mut o = String::new();
        if let Some(t) = &self.title {
            o.push_str(&format!("<title>{}</title>", esc(t)))
        }
        for (n, c) in &self.meta {
            o.push_str(&format!(
                "<meta name=\"{}\" content=\"{}\">",
                esc(n),
                esc(c)
            ))
        }
        for (r, h) in &self.links {
            o.push_str(&format!("<link rel=\"{}\" href=\"{}\">", esc(r), esc(h)))
        }
        for (s, d) in &self.scripts {
            o.push_str(&format!(
                "<script src=\"{}\"{}></script>",
                esc(s),
                if d.is_empty() {
                    String::new()
                } else {
                    " defer".into()
                }
            ))
        }
        for css in self.styles.values() {
            o.push_str(&format!("<style>{}</style>", css))
        }
        o
    }
}
fn valid(x: String) -> Result<String, HeadError> {
    if x.is_empty() || x.len() > 4096 || x.chars().any(|c| c.is_control()) {
        Err(HeadError::InvalidValue)
    } else {
        Ok(x)
    }
}
fn url(x: String) -> Result<String, HeadError> {
    if !x.starts_with("/") && !x.starts_with("https://") {
        Err(HeadError::InvalidValue)
    } else {
        valid(x)
    }
}
fn esc(x: &str) -> String {
    x.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_deterministically_and_deduplicates() {
        let h = Head::new()
            .title("Page")
            .unwrap()
            .meta("description", "A & B")
            .unwrap()
            .meta("description", "Final & Ready")
            .unwrap()
            .link("canonical", "https://example.com")
            .unwrap()
            .script("/app.js", true)
            .unwrap()
            .style("main", "body{color:red}")
            .unwrap();
        let o = h.render();
        assert!(o.contains("Final"));
        assert!(o.contains("defer"));
        assert!(o.contains("&amp;"))
    }
    #[test]
    fn rejects_unsafe_values() {
        assert!(matches!(
            Head::new().link("canonical", "javascript:bad"),
            Err(HeadError::InvalidValue)
        ));
        assert!(matches!(
            Head::new().title("bad\n"),
            Err(HeadError::InvalidValue)
        ));
    }
}
