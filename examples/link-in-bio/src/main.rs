use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yaiko_core::{
    json, App, Request, Response, Router, Server, Settings, StatusCode, tracing,
    LoggingMiddleware, SecurityHeaders,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkItem {
    id: String,
    title: String,
    url: String,
    description: String,
    featured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkInBioProfile {
    slug: String,
    display_name: String,
    headline: String,
    bio: String,
    avatar_emoji: String,
    accent_color: String,
    updated_at: DateTime<Utc>,
    links: Vec<LinkItem>,
}

#[derive(Debug, Deserialize)]
struct UpdateProfileInput {
    display_name: String,
    headline: String,
    bio: String,
    avatar_emoji: String,
    accent_color: String,
}

#[derive(Debug, Deserialize)]
struct CreateLinkInput {
    title: String,
    url: String,
    description: Option<String>,
    featured: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateLinkInput {
    title: String,
    url: String,
    description: Option<String>,
    featured: Option<bool>,
}

type ProfileStore = Arc<RwLock<LinkInBioProfile>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenvy::dotenv().ok();
    let settings = Settings::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let store = Arc::new(RwLock::new(sample_profile()));
    let store_for_profile = store.clone();
    let store_for_update = store.clone();
    let store_for_create_link = store.clone();
    let store_for_update_link = store.clone();
    let store_for_delete_link = store.clone();

    let router = Router::new()
        .get("/", |_req: Request| async move {
            Ok(Response::new().html(include_str!("../templates/index.html")))
        })
        .get("/api/profile", move |_req: Request| {
            let store = store_for_profile.clone();
            async move { get_profile(store) }
        })
        .put("/api/profile", move |req: Request| {
            let store = store_for_update.clone();
            async move { update_profile(req, store).await }
        })
        .post("/api/links", move |req: Request| {
            let store = store_for_create_link.clone();
            async move { create_link(req, store).await }
        })
        .put("/api/links/:id", move |req: Request| {
            let store = store_for_update_link.clone();
            async move { update_link(req, store).await }
        })
        .delete("/api/links/:id", move |req: Request| {
            let store = store_for_delete_link.clone();
            async move { delete_link(req, store).await }
        })
        .get("/robots.txt", |_req: Request| async move {
            Ok(Response::new()
                .text("User-agent: *\nAllow: /\nSitemap: /sitemap.xml\n")
                .status(StatusCode::OK))
        })
        .get("/sitemap.xml", |_req: Request| async move {
            let host = std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
            let xml = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n  <url><loc>{}/</loc><changefreq>weekly</changefreq><priority>1.0</priority></url>\n</urlset>",
                host
            );
            Ok(Response::new()
                .body(xml.into())
                .header("Content-Type", "application/xml")
                .status(StatusCode::OK))
        })
        .static_files("/static", "./public")
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new());

    let app = App::new().router(router);
    let addr: SocketAddr = settings.server_addr().parse()?;
    let server = Server::new(app, addr);

    tracing::info!("Link in bio example running on http://{}", addr);
    server.run().await?;
    Ok(())
}

fn get_profile(store: ProfileStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let profile = store.read().unwrap().clone();
    Ok(Response::new().json(&json!({ "profile": profile }))?)
}

async fn update_profile(
    mut req: Request,
    store: ProfileStore,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let input: UpdateProfileInput = serde_json::from_value(req.json().await?)?;

    if input.display_name.trim().is_empty() || input.headline.trim().is_empty() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({ "error": "Display name and headline are required" }))?);
    }

    let mut profile = store.write().unwrap();
    profile.display_name = input.display_name.trim().to_string();
    profile.headline = input.headline.trim().to_string();
    profile.bio = input.bio.trim().to_string();
    profile.avatar_emoji = if input.avatar_emoji.trim().is_empty() {
        "✨".to_string()
    } else {
        input.avatar_emoji.trim().to_string()
    };
    profile.accent_color = normalize_color(&input.accent_color);
    profile.updated_at = Utc::now();

    Ok(Response::new().json(&json!({ "profile": profile.clone() }))?)
}

async fn create_link(
    mut req: Request,
    store: ProfileStore,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let input: CreateLinkInput = serde_json::from_value(req.json().await?)?;

    let title = input.title.trim().to_string();
    let url = input.url.trim().to_string();
    if title.is_empty() || url.is_empty() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({ "error": "Title and URL are required" }))?);
    }

    let mut profile = store.write().unwrap();
    let link = LinkItem {
        id: Uuid::new_v4().to_string(),
        title,
        url,
        description: input.description.unwrap_or_default().trim().to_string(),
        featured: input.featured.unwrap_or(false),
    };
    profile.links.push(link.clone());
    profile.updated_at = Utc::now();

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "link": link, "profile": profile.clone() }))?)
}

async fn update_link(
    mut req: Request,
    store: ProfileStore,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let link_id = req.param("id").cloned().unwrap_or_default();
    let input: UpdateLinkInput = serde_json::from_value(req.json().await?)?;

    let mut profile = store.write().unwrap();
    let maybe_link = profile.links.iter_mut().find(|link| link.id == link_id);

    match maybe_link {
        Some(link) => {
            if input.title.trim().is_empty() || input.url.trim().is_empty() {
                return Ok(Response::new()
                    .status(StatusCode::BAD_REQUEST)
                    .json(&json!({ "error": "Title and URL are required" }))?);
            }

            link.title = input.title.trim().to_string();
            link.url = input.url.trim().to_string();
            link.description = input.description.unwrap_or_default().trim().to_string();
            link.featured = input.featured.unwrap_or(false);
            let updated_link = link.clone();
            profile.updated_at = Utc::now();

            Ok(Response::new().json(&json!({ "link": updated_link, "profile": profile.clone() }))?)
        }
        None => Ok(Response::new()
            .status(StatusCode::NOT_FOUND)
            .json(&json!({ "error": "Link not found" }))?),
    }
}

async fn delete_link(
    req: Request,
    store: ProfileStore,
) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let link_id = req.param("id").cloned().unwrap_or_default();
    let mut profile = store.write().unwrap();
    let before = profile.links.len();
    profile.links.retain(|link| link.id != link_id);

    if profile.links.len() == before {
        return Ok(Response::new()
            .status(StatusCode::NOT_FOUND)
            .json(&json!({ "error": "Link not found" }))?);
    }

    profile.updated_at = Utc::now();
    Ok(Response::new().json(&json!({ "profile": profile.clone() }))?)
}

fn normalize_color(color: &str) -> String {
    let trimmed = color.trim();
    if trimmed.starts_with('#') && trimmed.len() == 7 {
        trimmed.to_string()
    } else {
        "#ff6b35".to_string()
    }
}

fn sample_profile() -> LinkInBioProfile {
    LinkInBioProfile {
        slug: "marin-sloane".to_string(),
        display_name: "Marin Sloane".to_string(),
        headline: "Designing launches, writing sharp product notes, and making the internet feel alive."
            .to_string(),
        bio: "Part indie maker, part creative director. This demo shows how a Yaiko app can power a polished creator profile with editable links and instant preview."
            .to_string(),
        avatar_emoji: "☀️".to_string(),
        accent_color: "#ff6b35".to_string(),
        updated_at: Utc::now(),
        links: vec![
            LinkItem {
                id: Uuid::new_v4().to_string(),
                title: "Launch Week Journal".to_string(),
                url: "https://example.com/journal".to_string(),
                description: "Behind-the-scenes notes from product launch week.".to_string(),
                featured: true,
            },
            LinkItem {
                id: Uuid::new_v4().to_string(),
                title: "Book a Strategy Sprint".to_string(),
                url: "https://example.com/strategy".to_string(),
                description: "Two focused hours on positioning, messaging, and landing page fixes.".to_string(),
                featured: false,
            },
            LinkItem {
                id: Uuid::new_v4().to_string(),
                title: "Photo Sets + Moodboards".to_string(),
                url: "https://example.com/moodboards".to_string(),
                description: "Visual references, art direction, and campaign fragments.".to_string(),
                featured: false,
            },
        ],
    }
}
