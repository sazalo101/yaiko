use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ImageRecord {
    pub id: String,
    pub filename: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub width: i64,
    pub height: i64,
    pub view_count: i64,
    pub delete_token: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub url: String,
    pub viewer_url: String,
    pub delete_url: String,
    pub delete_token: String,
    pub filename: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ImageDetailResponse {
    pub id: String,
    pub url: String,
    pub raw_url: String,
    pub viewer_url: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub width: i64,
    pub height: i64,
    pub view_count: i64,
    pub created_at: String,
}
