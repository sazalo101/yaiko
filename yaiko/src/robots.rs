//! Deterministic robots.txt generation.
use std::fmt::Write;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RobotsError {
    InvalidValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotsTxt {
    user_agent: String,
    disallow: Vec<String>,
    allow: Vec<String>,
    sitemaps: Vec<String>,
}
impl RobotsTxt {
    pub fn new(user_agent: impl Into<String>) -> Result<Self, RobotsError> {
        let user_agent = valid(user_agent.into())?;
        Ok(Self {
            user_agent,
            disallow: Vec::new(),
            allow: Vec::new(),
            sitemaps: Vec::new(),
        })
    }
    pub fn disallow(mut self, path: impl Into<String>) -> Result<Self, RobotsError> {
        self.disallow.push(path.into());
        self.validate_path(self.disallow.last().unwrap())?;
        Ok(self)
    }
    pub fn allow(mut self, path: impl Into<String>) -> Result<Self, RobotsError> {
        self.allow.push(path.into());
        self.validate_path(self.allow.last().unwrap())?;
        Ok(self)
    }
    pub fn sitemap(mut self, url: impl Into<String>) -> Result<Self, RobotsError> {
        let url = url.into();
        if !url.starts_with("https://") || url.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            return Err(RobotsError::InvalidValue);
        }
        self.sitemaps.push(url);
        Ok(self)
    }
    pub fn render(&self) -> String {
        let mut out = String::new();
        writeln!(out, "User-agent: {}", self.user_agent).unwrap();
        for p in &self.disallow {
            writeln!(out, "Disallow: {p}").unwrap()
        }
        for p in &self.allow {
            writeln!(out, "Allow: {p}").unwrap()
        }
        for s in &self.sitemaps {
            writeln!(out, "Sitemap: {s}").unwrap()
        }
        out
    }
    fn validate_path(&self, path: &str) -> Result<(), RobotsError> {
        if path.is_empty()
            || !path.starts_with('/')
            || path.chars().any(|c| c.is_control() || c.is_whitespace())
        {
            Err(RobotsError::InvalidValue)
        } else {
            Ok(())
        }
    }
}
fn valid(v: String) -> Result<String, RobotsError> {
    if v.is_empty() || v.len() > 128 || v.chars().any(|c| c.is_control() || c.is_whitespace()) {
        Err(RobotsError::InvalidValue)
    } else {
        Ok(v)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renders_directives_deterministically() {
        let r = RobotsTxt::new("*")
            .unwrap()
            .disallow("/admin")
            .unwrap()
            .allow("/public")
            .unwrap()
            .sitemap("https://example.com/sitemap.xml")
            .unwrap();
        assert_eq!(r.render(),"User-agent: *\nDisallow: /admin\nAllow: /public\nSitemap: https://example.com/sitemap.xml\n")
    }
    #[test]
    fn rejects_invalid_values() {
        assert_eq!(RobotsTxt::new("bad agent"), Err(RobotsError::InvalidValue));
        assert_eq!(
            RobotsTxt::new("*").unwrap().disallow("admin"),
            Err(RobotsError::InvalidValue)
        );
        assert_eq!(
            RobotsTxt::new("*").unwrap().sitemap("http://example.com/x"),
            Err(RobotsError::InvalidValue)
        )
    }
}
