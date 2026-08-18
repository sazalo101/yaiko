//! Deterministic RSS 2.0 feed generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedError {
    InvalidValue,
    Capacity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedItem {
    pub title: String,
    pub link: String,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RssFeed {
    pub title: String,
    pub link: String,
    pub description: String,
    pub items: Vec<FeedItem>,
    max_items: usize,
}
impl RssFeed {
    pub fn new(
        title: impl Into<String>,
        link: impl Into<String>,
        description: impl Into<String>,
        max_items: usize,
    ) -> Result<Self, FeedError> {
        Ok(Self {
            title: v(title.into())?,
            link: url(link.into())?,
            description: v(description.into())?,
            items: Vec::new(),
            max_items: max_items.max(1),
        })
    }
    pub fn item(
        mut self,
        title: impl Into<String>,
        link: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, FeedError> {
        if self.items.len() >= self.max_items {
            return Err(FeedError::Capacity);
        }
        self.items.push(FeedItem {
            title: v(title.into())?,
            link: url(link.into())?,
            description: v(description.into())?,
        });
        Ok(self)
    }
    pub fn render(&self) -> String {
        let mut s=format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>{}</title><link>{}</link><description>{}</description>",e(&self.title),e(&self.link),e(&self.description));
        for i in &self.items {
            s.push_str(&format!(
                "<item><title>{}</title><link>{}</link><description>{}</description></item>",
                e(&i.title),
                e(&i.link),
                e(&i.description)
            ))
        }
        s.push_str("</channel></rss>");
        s
    }
}
fn v(x: String) -> Result<String, FeedError> {
    if x.is_empty() || x.len() > 4096 || x.chars().any(|c| c.is_control()) {
        Err(FeedError::InvalidValue)
    } else {
        Ok(x)
    }
}
fn url(x: String) -> Result<String, FeedError> {
    if !x.starts_with("https://") || x.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(FeedError::InvalidValue)
    } else {
        Ok(x)
    }
}
fn e(x: &str) -> String {
    x.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_escaped_feed() {
        let f = RssFeed::new("News & Updates", "https://example.com", "Latest", 2)
            .unwrap()
            .item("One <item>", "https://example.com/1", "A & B")
            .unwrap();
        assert!(f.render().contains("News &amp; Updates"));
        assert!(f.render().contains("A &amp; B"))
    }
    #[test]
    fn validates_urls_and_capacity() {
        assert_eq!(
            RssFeed::new("x", "http://example.com", "d", 1),
            Err(FeedError::InvalidValue)
        );
        let f = RssFeed::new("x", "https://example.com", "d", 1)
            .unwrap()
            .item("a", "https://example.com/a", "d")
            .unwrap();
        assert_eq!(
            f.clone().item("b", "https://example.com/b", "d"),
            Err(FeedError::Capacity)
        )
    }
}
