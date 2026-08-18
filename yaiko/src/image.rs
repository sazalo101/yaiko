//! Image optimization metadata and deterministic responsive HTML rendering.
use std::collections::BTreeSet;
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImageFormat {
    Webp,
    Avif,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loading {
    Lazy,
    Eager,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    InvalidSource,
    InvalidDimensions,
    InvalidWidth,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub src: String,
    pub alt: String,
    pub width: u32,
    pub height: u32,
    pub formats: BTreeSet<ImageFormat>,
    pub responsive: BTreeSet<u32>,
    pub loading: Loading,
}
impl Image {
    pub fn new(
        src: impl Into<String>,
        alt: impl Into<String>,
        width: u32,
        height: u32,
    ) -> Result<Self, ImageError> {
        let src = src.into();
        let alt = alt.into();
        if !src.starts_with('/') && !src.starts_with("https://") {
            return Err(ImageError::InvalidSource);
        }
        if width == 0 || height == 0 || width > 8192 || height > 8192 {
            return Err(ImageError::InvalidDimensions);
        }
        Ok(Self {
            src,
            alt,
            width,
            height,
            formats: BTreeSet::new(),
            responsive: BTreeSet::new(),
            loading: Loading::Lazy,
        })
    }
    pub fn format(mut self, f: ImageFormat) -> Self {
        self.formats.insert(f);
        self
    }
    pub fn responsive(mut self, w: u32) -> Result<Self, ImageError> {
        if w == 0 || w > 8192 {
            return Err(ImageError::InvalidWidth);
        }
        self.responsive.insert(w);
        Ok(self)
    }
    pub fn loading(mut self, l: Loading) -> Self {
        self.loading = l;
        self
    }
    pub fn render(&self) -> String {
        let mut s = format!(
            "<img src=\"{}\" alt=\"{}\" width=\"{}\" height=\"{}\" loading=\"{}\"",
            esc(&self.src),
            esc(&self.alt),
            self.width,
            self.height,
            match self.loading {
                Loading::Lazy => "lazy",
                Loading::Eager => "eager",
            }
        );
        if !self.responsive.is_empty() {
            let widths = self
                .responsive
                .iter()
                .map(|w| format!("{}w", w))
                .collect::<Vec<_>>()
                .join(",");
            s.push_str(&format!(" srcset=\"{}\" sizes=\"100vw\"", widths))
        }
        s.push_str(" />");
        s
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
    fn renders_optimized_responsive_image() {
        let i = Image::new("/hero.jpg", "Hero", 1200, 800)
            .unwrap()
            .format(ImageFormat::Webp)
            .format(ImageFormat::Avif)
            .responsive(640)
            .unwrap()
            .responsive(1200)
            .unwrap();
        let o = i.render();
        assert!(o.contains("loading=\"lazy\""));
        assert!(o.contains("640w,1200w"))
    }
    #[test]
    fn validates_sources_dimensions_and_widths() {
        assert_eq!(
            Image::new("http://bad", "x", 1, 1),
            Err(ImageError::InvalidSource)
        );
        assert_eq!(
            Image::new("/x", "x", 0, 1),
            Err(ImageError::InvalidDimensions)
        );
        assert_eq!(
            Image::new("/x", "x", 1, 1).unwrap().responsive(9000),
            Err(ImageError::InvalidWidth)
        )
    }
}
