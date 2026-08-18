//! Validated favicon and application-icon metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconError {
    InvalidValue,
    InvalidSize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Icon {
    pub path: String,
    pub sizes: Vec<u16>,
    pub mime: String,
    pub svg: Option<String>,
}
impl Icon {
    pub fn new(path: impl Into<String>, mime: impl Into<String>) -> Result<Self, IconError> {
        let path = path.into();
        let mime = mime.into();
        if !path.starts_with('/') || path.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(IconError::InvalidValue);
        }
        if mime.is_empty() || mime.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(IconError::InvalidValue);
        }
        Ok(Self {
            path,
            sizes: Vec::new(),
            mime,
            svg: None,
        })
    }
    pub fn size(mut self, size: u16) -> Result<Self, IconError> {
        if size == 0 || size > 4096 {
            return Err(IconError::InvalidSize);
        }
        if !self.sizes.contains(&size) {
            self.sizes.push(size);
            self.sizes.sort_unstable()
        }
        Ok(self)
    }
    pub fn svg(mut self, content: impl Into<String>) -> Result<Self, IconError> {
        let content = content.into();
        if !content.contains("<svg")
            || content.contains("<script")
            || content.chars().any(|c| c.is_control())
        {
            return Err(IconError::InvalidValue);
        }
        self.svg = Some(content);
        Ok(self)
    }
    pub fn render_link(&self) -> String {
        format!(
            "<link rel=\"icon\" type=\"{}\" href=\"{}\" sizes=\"{}\">",
            esc(&self.mime),
            esc(&self.path),
            self.sizes
                .iter()
                .map(|s| format!("{}x{}", s, s))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
    pub fn render_svg(&self) -> Option<&str> {
        self.svg.as_deref()
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
    fn renders_sorted_favicon_sizes_and_svg() {
        let i = Icon::new("/favicon.ico", "image/x-icon")
            .unwrap()
            .size(64)
            .unwrap()
            .size(32)
            .unwrap()
            .svg("<svg viewBox=\"0 0 1 1\"></svg>")
            .unwrap();
        assert_eq!(i.sizes, vec![32, 64]);
        assert!(i.render_link().contains("32x32 64x64"));
        assert!(i.render_svg().is_some())
    }
    #[test]
    fn rejects_unsafe_icon_data() {
        assert_eq!(
            Icon::new("favicon.ico", "image/x-icon"),
            Err(IconError::InvalidValue)
        );
        assert!(Icon::new("/x.svg", "image/svg+xml")
            .unwrap()
            .svg("<svg><script>alert(1)</script></svg>")
            .is_err())
    }
}
