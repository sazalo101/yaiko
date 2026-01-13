//! SEO module for Yaiko applications
//!
//! Provides automatic generation of robots.txt and sitemap.xml

use crate::{Request, Response, StatusCode};

/// Configuration for robots.txt generation
#[derive(Debug, Clone)]
pub struct RobotsTxt {
    pub user_agent: String,
    pub allow: Vec<String>,
    pub disallow: Vec<String>,
    pub sitemap_url: Option<String>,
}

impl Default for RobotsTxt {
    fn default() -> Self {
        Self {
            user_agent: "*".to_string(),
            allow: vec!["/".to_string()],
            disallow: vec![],
            sitemap_url: Some("/sitemap.xml".to_string()),
        }
    }
}

impl RobotsTxt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disallow(mut self, path: &str) -> Self {
        self.disallow.push(path.to_string());
        self
    }

    pub fn sitemap(mut self, url: &str) -> Self {
        self.sitemap_url = Some(url.to_string());
        self
    }

    pub fn render(&self) -> String {
        let mut content = format!("User-agent: {}\n", self.user_agent);
        
        for path in &self.allow {
            content.push_str(&format!("Allow: {}\n", path));
        }
        
        for path in &self.disallow {
            content.push_str(&format!("Disallow: {}\n", path));
        }
        
        if let Some(ref sitemap) = self.sitemap_url {
            content.push_str(&format!("Sitemap: {}\n", sitemap));
        }
        
        content
    }

    pub fn handler(&self) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Clone {
        let content = self.render();
        move |_req| {
            let content = content.clone();
            Box::pin(async move {
                Ok(Response::new()
                    .text(&content)
                    .status(StatusCode::OK))
            })
        }
    }
}

/// URL entry for sitemap
#[derive(Debug, Clone)]
pub struct SitemapUrl {
    pub loc: String,
    pub lastmod: Option<String>,
    pub changefreq: Option<String>,
    pub priority: Option<f32>,
}

impl SitemapUrl {
    pub fn new(loc: &str) -> Self {
        Self {
            loc: loc.to_string(),
            lastmod: None,
            changefreq: Some("weekly".to_string()),
            priority: Some(0.5),
        }
    }

    pub fn priority(mut self, p: f32) -> Self {
        self.priority = Some(p);
        self
    }

    pub fn changefreq(mut self, freq: &str) -> Self {
        self.changefreq = Some(freq.to_string());
        self
    }
}

/// Sitemap builder
#[derive(Debug, Clone, Default)]
pub struct Sitemap {
    pub base_url: String,
    pub urls: Vec<SitemapUrl>,
}

impl Sitemap {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            urls: vec![],
        }
    }

    pub fn add(mut self, url: SitemapUrl) -> Self {
        self.urls.push(url);
        self
    }

    pub fn add_path(mut self, path: &str) -> Self {
        self.urls.push(SitemapUrl::new(path));
        self
    }

    pub fn render(&self) -> String {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#);

        for url in &self.urls {
            xml.push_str("  <url>\n");
            xml.push_str(&format!("    <loc>{}{}</loc>\n", self.base_url, url.loc));
            
            if let Some(ref lastmod) = url.lastmod {
                xml.push_str(&format!("    <lastmod>{}</lastmod>\n", lastmod));
            }
            if let Some(ref freq) = url.changefreq {
                xml.push_str(&format!("    <changefreq>{}</changefreq>\n", freq));
            }
            if let Some(priority) = url.priority {
                xml.push_str(&format!("    <priority>{:.1}</priority>\n", priority));
            }
            
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>\n");
        xml
    }

    pub fn handler(&self) -> impl Fn(Request) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, Box<dyn std::error::Error + Send + Sync>>> + Send>> + Clone {
        let content = self.render();
        move |_req| {
            let content = content.clone();
            Box::pin(async move {
                Ok(Response::new()
                    .header("Content-Type", "application/xml")
                    .body(hyper::Body::from(content))
                    .status(StatusCode::OK))
            })
        }
    }
}
