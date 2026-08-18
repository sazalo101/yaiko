//! Safe client-navigation link metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationMode {
    Client,
    FullReload,
    NewTab,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prefetch {
    Disabled,
    Intent,
    Viewport,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    InvalidTarget,
    ExternalTarget,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub href: String,
    pub mode: NavigationMode,
    pub prefetch: Prefetch,
}
impl Link {
    pub fn new(href: impl Into<String>) -> Result<Self, LinkError> {
        let href = href.into();
        if href.is_empty() || href.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(LinkError::InvalidTarget);
        }
        if href.starts_with("http://") || href.starts_with("https://") || href.starts_with("//") {
            return Err(LinkError::ExternalTarget);
        }
        if !href.starts_with('/') && !href.starts_with('#') {
            return Err(LinkError::InvalidTarget);
        }
        Ok(Self {
            href,
            mode: NavigationMode::Client,
            prefetch: Prefetch::Disabled,
        })
    }
    pub fn mode(mut self, mode: NavigationMode) -> Self {
        self.mode = mode;
        self
    }
    pub fn prefetch(mut self, prefetch: Prefetch) -> Self {
        self.prefetch = prefetch;
        self
    }
    pub fn attributes(&self) -> String {
        let mut s = format!("href=\"{}\"", escape(&self.href));
        match self.mode {
            NavigationMode::Client => s.push_str(" data-navigation=\"client\""),
            NavigationMode::FullReload => s.push_str(" data-navigation=\"reload\""),
            NavigationMode::NewTab => s.push_str(" target=\"_blank\" rel=\"noopener noreferrer\""),
        }
        match self.prefetch {
            Prefetch::Disabled => {}
            Prefetch::Intent => s.push_str(" data-prefetch=\"intent\""),
            Prefetch::Viewport => s.push_str(" data-prefetch=\"viewport\""),
        }
        s
    }
}
fn escape(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_navigation_and_prefetch_attributes() {
        let l = Link::new("/editor?id=1")
            .unwrap()
            .mode(NavigationMode::Client)
            .prefetch(Prefetch::Viewport);
        assert!(l.attributes().contains("data-navigation=\"client\""));
        assert!(l.attributes().contains("data-prefetch=\"viewport\""))
    }
    #[test]
    fn rejects_external_and_unsafe_targets() {
        assert_eq!(
            Link::new("https://example.com"),
            Err(LinkError::ExternalTarget)
        );
        assert_eq!(Link::new("editor"), Err(LinkError::InvalidTarget));
    }
}
