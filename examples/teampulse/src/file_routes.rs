use yaiko_core::{Request, Response, BoxError, json, StatusCode};
use yaiko_core::file_upload::parse_multipart;
use uuid::Uuid;
use std::fs;

pub async fn upload_file(mut req: Request) -> Result<Response, BoxError> {
    // Requires authenticated session
    let user_id = req.session.as_ref().and_then(|s| s.get::<String>("user_id"));
    if user_id.is_none() {
        return Ok(Response::new().status(StatusCode::UNAUTHORIZED).json(&json!({"error": "Unauthorized"}))?);
    }

    let content_type = req.header("content-type").unwrap_or("").to_string();
    let body = std::mem::replace(&mut req.body, yaiko_core::Body::empty());

    let form = match parse_multipart(body, &content_type).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Multipart parse error: {}", e);
            return Ok(Response::new().status(StatusCode::BAD_REQUEST).json(&json!({"error": "Invalid multipart data"}))?);
        }
    };

    if let Some(file) = form.files.first() {
        let ext = file.filename.split('.').last().unwrap_or("");
        let ext_str = if ext.is_empty() { String::new() } else { format!(".{}", ext) };
        
        let new_filename = format!("{}{}", Uuid::new_v4(), ext_str);
        
        // Ensure upload directory exists
        let _ = fs::create_dir_all("public/uploads");
        
        let filepath = format!("public/uploads/{}", new_filename);
        fs::write(&filepath, &file.data)?;

        let url = format!("/uploads/{}", new_filename);
        return Ok(Response::new().json(&json!({"url": url, "filename": file.filename}))?);
    }

    Ok(Response::new().status(StatusCode::BAD_REQUEST).json(&json!({"error": "No file uploaded"}))?)
}
