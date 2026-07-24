use yaiko_core::{
    App, Router, Server, Request, Response, StatusCode, json,
    LoggingMiddleware, SecurityHeaders, init_tracing,
    parse_multipart,
};
use chrono::{DateTime, Utc, Duration};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileRequest {
    code: String,
    title: String,
    description: String,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    max_files: Option<usize>,
    files: Vec<UploadedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UploadedFile {
    id: String,
    original_filename: String,
    stored_filename: String,
    content_type: String,
    size: usize,
    uploaded_at: DateTime<Utc>,
    uploader_name: Option<String>,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateRequestInput {
    title: String,
    description: Option<String>,
    max_files: Option<usize>,
    expires_in_days: Option<i64>,
}

type RequestStore = Arc<RwLock<HashMap<String, FileRequest>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv::dotenv().ok();
    init_tracing();

    let store: RequestStore = Arc::new(RwLock::new(HashMap::new()));

    let router = Router::new()
        .get("/", home_page)
        .get("/r/:code", request_page)
        .get("/api/requests", {
            let store = store.clone();
            move |req| list_requests(req, store.clone())
        })
        .post("/api/requests", {
            let store = store.clone();
            move |req| create_request(req, store.clone())
        })
        .get("/api/requests/:code", {
            let store = store.clone();
            move |req| get_request(req, store.clone())
        })
        .post("/api/requests/:code/upload", {
            let store = store.clone();
            move |req| upload_files(req, store.clone())
        })
        .get("/download/:code/:file_id", {
            let store = store.clone();
            move |req| download_file(req, store.clone())
        })
        .static_files("/static", "./public")
        .use_middleware(LoggingMiddleware::new())
        .use_middleware(SecurityHeaders::new());

    let app = App::new().router(router);

    let addr: SocketAddr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
        std::env::var("PORT").unwrap_or_else(|_| "3000".to_string())
    ).parse()?;

    let server = Server::new(app, addr);
    server.run().await?;
    Ok(())
}

async fn home_page(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().html(include_str!("../templates/index.html")))
}

async fn request_page(_req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Response::new().html(include_str!("../templates/request.html")))
}

async fn list_requests(_req: Request, store: RequestStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let store = store.read().unwrap();
    let mut items: Vec<_> = store.values().cloned().collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let summary: Vec<_> = items.into_iter().map(|req| {
        let total_size: usize = req.files.iter().map(|f| f.size).sum();
        json!({
            "code": req.code,
            "title": req.title,
            "description": req.description,
            "created_at": req.created_at,
            "expires_at": req.expires_at,
            "max_files": req.max_files,
            "files_count": req.files.len(),
            "total_size": total_size,
        })
    }).collect();

    Ok(Response::new().json(&json!({ "requests": summary }))?)
}

async fn create_request(mut req: Request, store: RequestStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let value = req.json().await?;
    let input: CreateRequestInput = serde_json::from_value(value)?;

    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({ "error": "Title is required" }))?);
    }

    let code = generate_code(8);
    let expires_at = input.expires_in_days
        .filter(|days| *days > 0)
        .map(|days| Utc::now() + Duration::days(days));

    let request = FileRequest {
        code: code.clone(),
        title,
        description: input.description.unwrap_or_default(),
        created_at: Utc::now(),
        expires_at,
        max_files: input.max_files.filter(|v| *v > 0),
        files: Vec::new(),
    };

    store.write().unwrap().insert(code.clone(), request.clone());

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "request": request }))?)
}

async fn get_request(req: Request, store: RequestStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let code = req.param("code").cloned().unwrap_or_default();
    let store = store.read().unwrap();

    match store.get(&code) {
        Some(request) => Ok(Response::new().json(&json!({ "request": request }))?),
        None => Ok(Response::new()
            .status(StatusCode::NOT_FOUND)
            .json(&json!({ "error": "Request not found" }))?),
    }
}

async fn upload_files(req: Request, store: RequestStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let code = req.param("code").cloned().unwrap_or_default();

    let content_type = match req.header("content-type") {
        Some(ct) => ct.to_string(),
        None => {
            return Ok(Response::new()
                .status(StatusCode::BAD_REQUEST)
                .json(&json!({ "error": "Missing content-type" }))?);
        }
    };

    let parsed = parse_multipart(req.body, &content_type).await?;
    let uploader_name = parsed.fields.get("uploader_name").cloned();
    let note = parsed.fields.get("note").cloned();

    let (expires_at, max_files, existing_count) = {
        let store_guard = store.read().unwrap();
        let request = match store_guard.get(&code) {
            Some(req) => req,
            None => {
                return Ok(Response::new()
                    .status(StatusCode::NOT_FOUND)
                    .json(&json!({ "error": "Request not found" }))?);
            }
        };
        (request.expires_at, request.max_files, request.files.len())
    };

    if let Some(expires_at) = expires_at {
        if Utc::now() > expires_at {
            return Ok(Response::new()
                .status(StatusCode::GONE)
                .json(&json!({ "error": "This link has expired" }))?);
        }
    }

    if let Some(max_files) = max_files {
        if existing_count + parsed.files.len() > max_files {
            return Ok(Response::new()
                .status(StatusCode::BAD_REQUEST)
                .json(&json!({ "error": "File limit exceeded" }))?);
        }
    }

    let mut uploaded = Vec::new();
    let upload_dir = format!("./uploads/{}", code);

    for mut file in parsed.files {
        let file_id = Uuid::new_v4().to_string();
        let original_filename = file.filename.clone();
        let safe_name = sanitize_filename(&original_filename);
        let stored_filename = format!("{}_{}", file_id, safe_name);
        file.filename = stored_filename.clone();
        file.save_to(&upload_dir).await?;

        let record = UploadedFile {
            id: file_id,
            original_filename,
            stored_filename,
            content_type: file.content_type,
            size: file.size,
            uploaded_at: Utc::now(),
            uploader_name: uploader_name.clone(),
            note: note.clone(),
        };

        uploaded.push(record);
    }

    if !uploaded.is_empty() {
        let mut store_guard = store.write().unwrap();
        if let Some(request) = store_guard.get_mut(&code) {
            request.files.extend(uploaded.clone());
        }
    }

    Ok(Response::new().json(&json!({ "uploaded": uploaded }))?)
}

async fn download_file(req: Request, store: RequestStore) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    let code = req.param("code").cloned().unwrap_or_default();
    let file_id = req.param("file_id").cloned().unwrap_or_default();

    let file = {
        let store_guard = store.read().unwrap();
        let request = match store_guard.get(&code) {
            Some(req) => req,
            None => {
                return Ok(Response::new()
                    .status(StatusCode::NOT_FOUND)
                    .text("Request not found"));
            }
        };

        match request.files.iter().find(|f| f.id == file_id) {
            Some(file) => file.clone(),
            None => {
                return Ok(Response::new()
                    .status(StatusCode::NOT_FOUND)
                    .text("File not found"));
            }
        }
    };

    let path = format!("./uploads/{}/{}", code, file.stored_filename);
    let reader = tokio::fs::File::open(&path).await?;
    let download_name = sanitize_filename(&file.original_filename);

    Ok(Response::new()
        .header("Content-Type", &file.content_type)
        .header("Content-Disposition", &format!("attachment; filename=\"{}\"", download_name))
        .stream(reader))
}

fn generate_code(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .filter(|c| c.is_ascii_alphanumeric())
        .take(len)
        .map(|c| c.to_ascii_lowercase() as char)
        .collect()
}

fn sanitize_filename(name: &str) -> String {
    let mut cleaned = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            cleaned.push(ch);
        } else {
            cleaned.push('_');
        }
    }
    if cleaned.is_empty() {
        "file".to_string()
    } else {
        cleaned
    }
}
