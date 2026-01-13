use yaiko_core::{
    App, Router, Server, Request, Response, StatusCode, json, Body,
    LoggingMiddleware, SecurityHeaders, HealthCheck, init_tracing, tracing,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
    slug: String,
    title: String,
    date: String,
    excerpt: String,
    content: String,
}

/// Load posts from the `posts/` directory
fn load_posts() -> Vec<Post> {
    let mut posts = Vec::new();
    
    if let Ok(entries) = fs::read_dir("posts") {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Some(post) = parse_post(&content, &entry.file_name().to_string_lossy()) {
                    posts.push(post);
                }
            }
        }
    }
    
    // Sort by date (newest first)
    posts.sort_by(|a, b| b.date.cmp(&a.date));
    posts
}

/// Parse a post file (simple front matter format)
fn parse_post(content: &str, filename: &str) -> Option<Post> {
    let slug = filename.replace(".md", "");
    
    // Simple format: first line = title, second line = date, rest = content
    let lines: Vec<&str> = content.lines().collect();
    
    if lines.len() < 3 {
        return None;
    }
    
    let title = lines[0].trim_start_matches("# ").to_string();
    let date = lines[1].to_string();
    let content_lines: Vec<&str> = lines[3..].to_vec();
    let content = content_lines.join("\n");
    let excerpt = content_lines.iter().take(3).cloned().collect::<Vec<_>>().join(" ");
    
    Some(Post {
        slug,
        title,
        date,
        excerpt: format!("{}...", &excerpt[..excerpt.len().min(150)]),
        content,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    init_tracing();
    
    tracing::info!("Yaiko Blog Example starting...");
    
    let router = Router::new()
        // Health check endpoint
        .get("/health", HealthCheck::new())
        // Pages
        .get("/", home_handler)
        .get("/posts/:slug", post_handler)
        .get("/api/posts", api_posts)
        .get("/robots.txt", robots_handler)
        .get("/sitemap.xml", sitemap_handler)
        // Static files
        .static_files("/static", "./public")
        // Middleware
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new());
    
    let app = App::new().router(router);
    
    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let server = Server::new(app, addr);
    
    tracing::info!("Blog running at http://{}", addr);
    server.run().await?;
    
    Ok(())
}

async fn home_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let posts = load_posts();
    
    let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="A simple static blog built with Yaiko">
    <title>Yaiko Blog</title>
    <link rel="stylesheet" href="/static/css/blog.css">
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
</head>
<body>
    <div class="blog">
        <header class="header">
            <h1 class="logo">Yaiko Blog</h1>
            <p class="tagline">A static blog built with Rust + jQuery</p>
        </header>
        
        <main class="posts">
"#);

    for post in &posts {
        html.push_str(&format!(r#"
            <article class="post-card">
                <h2><a href="/posts/{}">{}</a></h2>
                <time>{}</time>
                <p>{}</p>
            </article>
"#, post.slug, post.title, post.date, post.excerpt));
    }

    html.push_str(r#"
        </main>
        
        <footer class="footer">
            <p>Built with Yaiko</p>
        </footer>
    </div>
</body>
</html>"#);

    Ok(Response::new().html(&html))
}

async fn post_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let slug = req.param("slug").map(|s| s.to_string()).unwrap_or_default();
    let posts = load_posts();
    
    let post = posts.iter().find(|p| p.slug == slug);
    
    match post {
        Some(post) => {
            let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="{}">
    <title>{} - Yaiko Blog</title>
    <link rel="stylesheet" href="/static/css/blog.css">
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;700&display=swap" rel="stylesheet">
</head>
<body>
    <div class="blog">
        <header class="header">
            <a href="/" class="back">← Back to posts</a>
        </header>
        
        <article class="post">
            <h1>{}</h1>
            <time>{}</time>
            <div class="content">
                {}
            </div>
        </article>
        
        <footer class="footer">
            <p>Built with Yaiko</p>
        </footer>
    </div>
</body>
</html>"#, post.excerpt, post.title, post.title, post.date, post.content.replace("\n", "<br>"));
            
            Ok(Response::new().html(&html))
        }
        None => {
            Ok(Response::new()
                .status(StatusCode::NOT_FOUND)
                .html("<h1>Post not found</h1>"))
        }
    }
}

async fn api_posts(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let posts = load_posts();
    Ok(Response::new().json(&json!({ "posts": posts }))?)
}

async fn robots_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new()
        .text("User-agent: *\nAllow: /\nSitemap: /sitemap.xml\n"))
}

async fn sitemap_handler(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let posts = load_posts();
    let host = "http://localhost:3000";
    
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url>
    <loc>"#);
    xml.push_str(host);
    xml.push_str(r#"/</loc>
    <changefreq>weekly</changefreq>
    <priority>1.0</priority>
  </url>
"#);
    
    for post in &posts {
        xml.push_str(&format!(r#"  <url>
    <loc>{}/posts/{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.8</priority>
  </url>
"#, host, post.slug, post.date));
    }
    
    xml.push_str("</urlset>\n");
    
    Ok(Response::new()
        .header("Content-Type", "application/xml")
        .body(Body::from(xml))
        .status(StatusCode::OK))
}
