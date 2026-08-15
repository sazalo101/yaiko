use yaiko_core::{
    App, Router, Server, Request, Response, StatusCode, 
    LoggingMiddleware, init_tracing, tracing,
};
use std::net::SocketAddr;
use std::fs;
use std::path::Path;
use handlebars::Handlebars;
use pulldown_cmark::{Parser, Options, html};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct PageContext {
    title: String,
    content: String,
    sidebar: Vec<SidebarItem>,
}

#[derive(Serialize, Clone)]
struct SidebarItem {
    title: String,
    url: String,
    active: bool,
}

struct AppState {
    hbs: Handlebars<'static>,
    sidebar: Vec<SidebarItem>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_tracing();
    
    // Setup Handlebars
    let mut hbs = Handlebars::new();
    hbs.register_template_string("layout", include_str!("../templates/layout.html"))?;
    hbs.register_template_string("page", include_str!("../templates/page.html"))?;
    
    // Define Sidebar
    let sidebar = vec![
        SidebarItem { title: "Introduction".into(), url: "/".into(), active: false },
        SidebarItem { title: "Yaiko for Beginners".into(), url: "/beginners-book".into(), active: false },
        SidebarItem { title: "The Yaiko Book".into(), url: "/book".into(), active: false },
        SidebarItem { title: "Getting Started".into(), url: "/getting-started".into(), active: false },
        SidebarItem { title: "Tutorial".into(), url: "/tutorial".into(), active: false },
        SidebarItem { title: "Configuration".into(), url: "/configuration".into(), active: false },
        SidebarItem { title: "Routing".into(), url: "/routing".into(), active: false },
        SidebarItem { title: "Database".into(), url: "/database".into(), active: false },
        SidebarItem { title: "Frontend".into(), url: "/frontend".into(), active: false },
        SidebarItem { title: "Security".into(), url: "/security".into(), active: false },
        SidebarItem { title: "Deployment".into(), url: "/deployment".into(), active: false },
        SidebarItem { title: "WebSockets".into(), url: "/websockets".into(), active: false },
        SidebarItem { title: "Background Jobs".into(), url: "/background-jobs".into(), active: false },
        SidebarItem { title: "File Uploads".into(), url: "/file-uploads".into(), active: false },
        SidebarItem { title: "CLI Tool".into(), url: "/cli".into(), active: false },
    ];
    
    let state = Arc::new(AppState {
        hbs,
        sidebar,
    });
    
    let router = Router::new()
        .static_files("/css", "./public/css")
        .get("/", {
            let s = state.clone();
            move |req| render_page(req, s.clone(), "index".to_string())
        })
        .get("/:page", {
            let s = state.clone();
            move |req: Request| {
                let page = req.param("page").map(|s| s.as_str()).unwrap_or("index").to_string();
                render_page(req, s.clone(), page)
            }
        })
        .use_middleware(LoggingMiddleware::new());
    
    let app = App::new().router(router);
    
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);
        
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    
    tracing::info!("Docs server running at http://{}", addr);
    Server::new(app, addr).run().await?;
    
    Ok(())
}

async fn render_page(_req: Request, state: Arc<AppState>, page_name: String) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let file_path = format!("./content/{}.md", page_name);
    let path = Path::new(&file_path);
    
    if !path.exists() {
        return Ok(Response::new().status(StatusCode::NOT_FOUND).text("Page not found"));
    }
    
    let markdown_input = fs::read_to_string(path)?;
    
    // Extract title (first h1)
    let title = markdown_input.lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim())
        .unwrap_or("Documentation")
        .to_string();
        
    // Render Markdown
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    
    let parser = Parser::new_ext(&markdown_input, options).map(|event| {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(link_type, dest, title)) => {
                if dest.ends_with(".md") {
                    let new_dest = dest.replace(".md", "");
                    let new_dest = if new_dest.starts_with("./") {
                        new_dest.replace("./", "/")
                    } else if !new_dest.starts_with("/") && !new_dest.starts_with("http") {
                        format!("/{}", new_dest)
                    } else {
                        new_dest
                    };
                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(link_type, new_dest.into(), title))
                } else {
                    pulldown_cmark::Event::Start(pulldown_cmark::Tag::Link(link_type, dest, title))
                }
            },
            pulldown_cmark::Event::End(pulldown_cmark::Tag::Link(link_type, dest, title)) => {
                 if dest.ends_with(".md") {
                    let new_dest = dest.replace(".md", "");
                    let new_dest = if new_dest.starts_with("./") {
                        new_dest.replace("./", "/")
                    } else if !new_dest.starts_with("/") && !new_dest.starts_with("http") {
                        format!("/{}", new_dest)
                    } else {
                        new_dest
                    };
                    pulldown_cmark::Event::End(pulldown_cmark::Tag::Link(link_type, new_dest.into(), title))
                } else {
                    pulldown_cmark::Event::End(pulldown_cmark::Tag::Link(link_type, dest, title))
                }
            }
            _ => event,
        }
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    
    // Prepare sidebar with active state
    let current_url = if page_name == "index" { "/".to_string() } else { format!("/{}", page_name) };
    let sidebar: Vec<SidebarItem> = state.sidebar.iter().map(|item| {
        let mut i = item.clone();
        if i.url == current_url {
            i.active = true;
        }
        i
    }).collect();
    
    let context = PageContext {
        title,
        content: html_output,
        sidebar,
    };
    
    let body = state.hbs.render("page", &context)?;
    
    Ok(Response::new().html(&body))
}
