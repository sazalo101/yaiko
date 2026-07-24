use yaiko_core::{Request, Response, BoxError, StatusCode, json, parse_multipart};
use sqlx::SqlitePool;
use uuid::Uuid;
use rand::Rng;
use std::path::Path;

use crate::models::image::UploadResponse;

const ALLOWED_TYPES: &[&str] = &[
    "image/jpeg", "image/png", "image/gif", "image/webp", "image/svg+xml", "image/bmp",
];
const MAX_SIZE: usize = 10 * 1024 * 1024; // 10 MB

fn generate_short_id() -> String {
    let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789".chars().collect();
    let mut rng = rand::thread_rng();
    (0..8).map(|_| chars[rng.gen_range(0..chars.len())]).collect()
}

fn generate_delete_token() -> String {
    Uuid::new_v4().to_string().replace("-", "")
}

fn file_extension(mime: &str) -> &str {
    match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        _ => "bin",
    }
}

async fn validate_nsfw_jigsawstack(image_url: &str, api_key: &str) -> Result<bool, BoxError> {
    let client = reqwest::Client::new();
    let res = client.post("https://api.jigsawstack.com/v1/validate/nsfw")
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "url": image_url }))
        .send()
        .await?;

    if res.status().is_success() {
        let body: serde_json::Value = res.json().await?;
        let is_nsfw = body.get("nsfw").and_then(|v| v.as_bool()).unwrap_or(false);
        let is_nudity = body.get("nudity").and_then(|v| v.as_bool()).unwrap_or(false);
        let nsfw_score = body.get("nsfw_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let nudity_score = body.get("nudity_score").and_then(|v| v.as_f64()).unwrap_or(0.0);

        if is_nsfw || is_nudity || nsfw_score > 0.4 || nudity_score > 0.4 {
            tracing::warn!("JigsawStack NSFW detected! image_url={}, nsfw_score={}, nudity_score={}", image_url, nsfw_score, nudity_score);
            return Ok(true);
        }
    }
    Ok(false)
}

pub async fn upload(mut req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let content_type = req.header("content-type").unwrap_or_default().to_string();

    if !content_type.starts_with("multipart/form-data") {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({"error": "Expected multipart/form-data"}))?);
    }

    let body = std::mem::take(&mut req.body);
    let parsed = parse_multipart(body, &content_type).await?;

    if parsed.files.is_empty() {
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({"error": "No files uploaded"}))?);
    }

    let site_url = std::env::var("SITE_URL").unwrap_or_else(|_| "http://localhost:3000".into());
    let host_base = req.header("host").map(|h| {
        let proto = req.header("x-forwarded-proto").unwrap_or("http");
        format!("{}://{}", proto, h)
    }).unwrap_or(site_url.clone());

    let jigsaw_key = std::env::var("JIGSAWSTACK_API_KEY").ok();
    let mut results: Vec<UploadResponse> = Vec::new();
    let mut nsfw_rejected = false;

    for file in &parsed.files {
        if !ALLOWED_TYPES.contains(&file.content_type.as_str()) {
            continue;
        }
        if file.size > MAX_SIZE {
            continue;
        }

        let id = generate_short_id();
        let ext = file_extension(&file.content_type);
        let disk_filename = format!("{}.{}", id, ext);
        let delete_token = generate_delete_token();
        let now = chrono::Utc::now().to_rfc3339();

        // Save to disk
        let upload_dir = Path::new("public/uploads");
        tokio::fs::create_dir_all(upload_dir).await?;
        let dest = upload_dir.join(&disk_filename);
        tokio::fs::write(&dest, &file.data).await?;

        // Public URL for NSFW check
        let image_public_url = format!("{}/uploads/{}", site_url, disk_filename);

        // JigsawStack NSFW Validation Check
        if let Some(key) = &jigsaw_key {
            match validate_nsfw_jigsawstack(&image_public_url, key).await {
                Ok(true) => {
                    // NSFW / Nudity flagged! Delete file immediately
                    tokio::fs::remove_file(&dest).await.ok();
                    nsfw_rejected = true;
                    continue;
                }
                Err(e) => {
                    tracing::error!("JigsawStack NSFW check error: {:?}", e);
                }
                _ => {}
            }
        }

        // Insert metadata into database
        sqlx::query(
            "INSERT INTO images (id, filename, original_name, mime_type, size_bytes, width, height, view_count, delete_token, created_at) VALUES (?, ?, ?, ?, ?, 0, 0, 0, ?, ?)"
        )
        .bind(&id)
        .bind(&disk_filename)
        .bind(&file.filename)
        .bind(&file.content_type)
        .bind(file.size as i64)
        .bind(&delete_token)
        .bind(&now)
        .execute(&pool)
        .await?;

        results.push(UploadResponse {
            id: id.clone(),
            url: format!("{}/uploads/{}", host_base, disk_filename),
            viewer_url: format!("{}/i/{}", host_base, id),
            delete_url: format!("{}/api/images/{}", host_base, id),
            delete_token: delete_token.clone(),
            filename: disk_filename,
            original_name: file.filename.clone(),
            mime_type: file.content_type.clone(),
            size_bytes: file.size as i64,
            created_at: now,
        });
    }

    if results.is_empty() {
        if nsfw_rejected {
            return Ok(Response::new()
                .status(StatusCode::BAD_REQUEST)
                .json(&json!({"error": "Upload rejected: NSFW or explicit/nude content detected by JigsawStack."}))?);
        }
        return Ok(Response::new()
            .status(StatusCode::BAD_REQUEST)
            .json(&json!({"error": "No valid images found. Allowed: JPEG, PNG, GIF, WebP, SVG, BMP. Max 10MB."}))?);
    }

    Ok(Response::new()
        .status(StatusCode::CREATED)
        .json(&json!({ "images": results }))?)
}
