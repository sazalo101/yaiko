# File Uploads

Yaiko makes it easy to handle multipart file uploads.

## Usage

### 1. Import File Upload Types

```rust
use yaiko_core::{FileUpload, parse_multipart};
```

### 2. Handle Upload Request

Use `parse_multipart` to extract files from the request body:

```rust
async fn upload_handler(req: Request) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
    // Get Content-Type header (required for boundary)
    let content_type = req.headers.get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Parse files
    let files = parse_multipart(req.body, content_type).await?;
    
    for file in files {
        println!("Uploaded: {} ({} bytes)", file.filename, file.size);
        
        // Save to disk
        file.save_to("./uploads").await?;
    }
    
    Ok(Response::new().text("Upload successful"))
}
```

## FileUpload Struct

The `FileUpload` struct contains:

```rust
pub struct FileUpload {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub data: Vec<u8>,
}
```

## Methods

- `save_to(directory: &str)`: Saves the file to the specified directory. It automatically creates the directory if it doesn't exist.

## Frontend Example

```html
<form action="/upload" method="POST" enctype="multipart/form-data">
    <input type="file" name="file">
    <button type="submit">Upload</button>
</form>
```
