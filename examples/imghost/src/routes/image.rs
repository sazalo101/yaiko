use yaiko_core::{Request, Response, BoxError, StatusCode, json};
use sqlx::{SqlitePool, Row};

use crate::models::image::ImageDetailResponse;

pub async fn get_image(req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let id = req.param("id").cloned().unwrap_or_default();

    let row = sqlx::query(
        "SELECT id, filename, original_name, mime_type, size_bytes, width, height, view_count, created_at FROM images WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await?;

    match row {
        Some(r) => {
            let filename: String = r.get("filename");
            let view_count: i64 = r.get("view_count");

            let detail = ImageDetailResponse {
                id: r.get("id"),
                url: format!("/uploads/{}", filename),
                raw_url: format!("/uploads/{}", filename),
                viewer_url: format!("/i/{}", id),
                original_name: r.get("original_name"),
                mime_type: r.get("mime_type"),
                size_bytes: r.get("size_bytes"),
                width: r.get("width"),
                height: r.get("height"),
                view_count: view_count + 1,
                created_at: r.get("created_at"),
            };

            // Fire-and-forget view count increment — does NOT block the response.
            // Spawned as a background task so concurrent reads never queue on the DB write lock.
            tokio::spawn({
                let pool = pool.clone();
                let id = id.clone();
                async move {
                    let _ = sqlx::query("UPDATE images SET view_count = view_count + 1 WHERE id = ?")
                        .bind(&id)
                        .execute(&pool)
                        .await;
                }
            });

            Ok(Response::new().json(&detail)?)
        }
        None => {
            Ok(Response::new()
                .status(StatusCode::NOT_FOUND)
                .json(&json!({"error": "Image not found"}))?)
        }
    }
}

pub async fn delete_image(mut req: Request, pool: SqlitePool) -> Result<Response, BoxError> {
    let id = req.param("id").cloned().unwrap_or_default();

    // Get delete token from request body or query param
    let token = {
        if let Ok(body) = req.json().await {
            body.get("delete_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    }.or_else(|| req.query_param("token").cloned());

    let token = match token {
        Some(t) => t,
        None => {
            return Ok(Response::new()
                .status(StatusCode::BAD_REQUEST)
                .json(&json!({"error": "delete_token is required"}))?);
        }
    };

    let row = sqlx::query("SELECT delete_token, filename FROM images WHERE id = ?")
        .bind(&id)
        .fetch_optional(&pool)
        .await?;

    match row {
        Some(r) => {
            let stored_token: String = r.get("delete_token");
            let filename: String = r.get("filename");

            if stored_token != token {
                return Ok(Response::new()
                    .status(StatusCode::FORBIDDEN)
                    .json(&json!({"error": "Invalid delete token"}))?);
            }

            // Delete from database
            sqlx::query("DELETE FROM images WHERE id = ?")
                .bind(&id)
                .execute(&pool)
                .await?;

            // Delete file from disk
            let path = std::path::Path::new("public/uploads").join(&filename);
            if path.exists() {
                tokio::fs::remove_file(&path).await.ok();
            }

            Ok(Response::new().json(&json!({"status": "deleted", "id": id}))?)
        }
        None => {
            Ok(Response::new()
                .status(StatusCode::NOT_FOUND)
                .json(&json!({"error": "Image not found"}))?)
        }
    }
}
