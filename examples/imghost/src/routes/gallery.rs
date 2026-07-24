use yaiko_core::{Request, Response, BoxError, json};
use sqlx::{SqlitePool, Row};

pub async fn recent(_req: Request, pool: SqlitePool) -> Result<Response, BoxError> {

    let rows = sqlx::query(
        "SELECT id, filename, original_name, mime_type, size_bytes, view_count, created_at FROM images ORDER BY created_at DESC LIMIT 24"
    )
    .fetch_all(&pool)
    .await?;

    let images: Vec<serde_json::Value> = rows.iter().map(|r| {
        let id: String = r.get("id");
        let filename: String = r.get("filename");
        json!({
            "id": id,
            "thumbnail": format!("/uploads/{}", filename),
            "viewer_url": format!("/i/{}", id),
            "original_name": r.get::<String, _>("original_name"),
            "mime_type": r.get::<String, _>("mime_type"),
            "size_bytes": r.get::<i64, _>("size_bytes"),
            "view_count": r.get::<i64, _>("view_count"),
            "created_at": r.get::<String, _>("created_at"),
        })
    }).collect();


    Ok(Response::new().json(&json!({ "images": images }))?)
}
