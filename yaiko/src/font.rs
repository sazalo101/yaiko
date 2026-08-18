//! Safe font loading metadata for Google Fonts and local assets.
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontStyle {
    Normal,
    Italic,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontError {
    InvalidFamily,
    InvalidSource,
    InvalidWeight,
    InvalidSubset,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontSource {
    Google,
    Local(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Font {
    pub family: String,
    pub source: FontSource,
    pub weights: BTreeSet<u16>,
    pub styles: BTreeSet<FontStyle>,
    pub subsets: BTreeSet<String>,
}
impl Font {
    pub fn google(family: impl Into<String>) -> Result<Self, FontError> {
        Self::new(family.into(), FontSource::Google)
    }
    pub fn local(family: impl Into<String>, path: impl Into<String>) -> Result<Self, FontError> {
        let path = path.into();
        if !path.starts_with('/') || path.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(FontError::InvalidSource);
        }
        Self::new(family.into(), FontSource::Local(path))
    }
    fn new(family: String, source: FontSource) -> Result<Self, FontError> {
        if family.is_empty() || family.len() > 128 || family.chars().any(|c| c.is_control()) {
            return Err(FontError::InvalidFamily);
        }
        Ok(Self {
            family,
            source,
            weights: BTreeSet::new(),
            styles: BTreeSet::new(),
            subsets: BTreeSet::new(),
        })
    }
    pub fn weight(mut self, w: u16) -> Result<Self, FontError> {
        if !(100..=900).contains(&w) || !w.is_multiple_of(100) {
            return Err(FontError::InvalidWeight);
        }
        self.weights.insert(w);
        Ok(self)
    }
    pub fn style(mut self, s: FontStyle) -> Self {
        self.styles.insert(s);
        self
    }
    pub fn subset(mut self, s: impl Into<String>) -> Result<Self, FontError> {
        let s = s.into();
        if s.is_empty() || s.len() > 32 || s.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(FontError::InvalidSubset);
        }
        self.subsets.insert(s);
        Ok(self)
    }
    pub fn render(&self) -> String {
        match &self.source{FontSource::Local(path)=>format!("<link rel=\"preload\" as=\"font\" href=\"{}\" crossorigin><style>@font-face{{font-family:'{}';src:url('{}') format('woff2');}}</style>",esc(path),esc(&self.family),esc(path)),FontSource::Google=>{let weights=self.weights.iter().map(|w|w.to_string()).collect::<Vec<_>>().join(";");let subsets=self.subsets.iter().cloned().collect::<Vec<_>>().join(",");format!("<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\"><link rel=\"stylesheet\" href=\"https://fonts.googleapis.com/css2?family={}&wght@{}&subset={}\">",esc(&self.family.replace(' ', "+")),weights,esc(&subsets))}}
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
    fn renders_google_and_local_fonts() {
        let g = Font::google("Inter")
            .unwrap()
            .weight(400)
            .unwrap()
            .weight(700)
            .unwrap()
            .subset("latin")
            .unwrap();
        assert!(g.render().contains("fonts.googleapis.com"));
        let l = Font::local("Brand", "/fonts/brand.woff2").unwrap();
        assert!(l.render().contains("font-face"))
    }
    #[test]
    fn validates_font_values() {
        assert_eq!(Font::google(""), Err(FontError::InvalidFamily));
        assert_eq!(
            Font::google("Inter").unwrap().weight(450),
            Err(FontError::InvalidWeight)
        );
        assert_eq!(
            Font::local("Brand", "fonts/a.woff2"),
            Err(FontError::InvalidSource)
        )
    }
}
