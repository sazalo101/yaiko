//! Validated SEO and social metadata rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataError {
    InvalidValue,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub canonical: Option<String>,
    pub image: Option<String>,
    pub json_ld: Option<String>,
}
impl Metadata {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn title(mut self, v: impl Into<String>) -> Result<Self, MetadataError> {
        self.title = Some(valid(v.into())?);
        Ok(self)
    }
    pub fn description(mut self, v: impl Into<String>) -> Result<Self, MetadataError> {
        self.description = Some(valid(v.into())?);
        Ok(self)
    }
    pub fn canonical(mut self, v: impl Into<String>) -> Result<Self, MetadataError> {
        self.canonical = Some(url(v.into())?);
        Ok(self)
    }
    pub fn image(mut self, v: impl Into<String>) -> Result<Self, MetadataError> {
        self.image = Some(url(v.into())?);
        Ok(self)
    }
    pub fn json_ld(mut self, v: impl Into<String>) -> Result<Self, MetadataError> {
        let v = valid(v.into())?;
        if !v.trim_start().starts_with('{') {
            return Err(MetadataError::InvalidValue);
        }
        self.json_ld = Some(v);
        Ok(self)
    }
    pub fn render(&self) -> String {
        let mut s = String::new();
        if let Some(v) = &self.title {
            s.push_str(&format!(
                "<title>{}</title><meta property=\"og:title\" content=\"{}\">",
                e(v),
                e(v)
            ))
        }
        if let Some(v) = &self.description {
            s.push_str(&format!("<meta name=\"description\" content=\"{}\"><meta property=\"og:description\" content=\"{}\">",e(v),e(v)))
        }
        if let Some(v) = &self.canonical {
            s.push_str(&format!("<link rel=\"canonical\" href=\"{}\">", e(v)))
        }
        if let Some(v) = &self.image {
            s.push_str(&format!("<meta property=\"og:image\" content=\"{}\"><meta name=\"twitter:card\" content=\"summary_large_image\">",e(v)))
        }
        if let Some(v) = &self.json_ld {
            s.push_str(&format!(
                "<script type=\"application/ld+json\">{}</script>",
                v
            ))
        }
        s
    }
}
fn valid(v: String) -> Result<String, MetadataError> {
    if v.is_empty() || v.len() > 4096 || v.chars().any(|c| c.is_control()) {
        Err(MetadataError::InvalidValue)
    } else {
        Ok(v)
    }
}
fn url(v: String) -> Result<String, MetadataError> {
    if !v.starts_with("https://") || v.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(MetadataError::InvalidValue)
    } else {
        Ok(v)
    }
}
fn e(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_seo_social_and_jsonld() {
        let m = Metadata::new()
            .title("A & B")
            .unwrap()
            .description("Desc")
            .unwrap()
            .canonical("https://example.com")
            .unwrap()
            .image("https://example.com/a.png")
            .unwrap()
            .json_ld("{\"@type\":\"VideoObject\"}")
            .unwrap();
        let o = m.render();
        assert!(o.contains("og:title"));
        assert!(o.contains("&amp;"));
        assert!(o.contains("application/ld+json"))
    }
    #[test]
    fn rejects_invalid_metadata() {
        assert_eq!(
            Metadata::new().canonical("http://example.com"),
            Err(MetadataError::InvalidValue)
        );
        assert_eq!(
            Metadata::new().json_ld("[]"),
            Err(MetadataError::InvalidValue)
        )
    }
}
