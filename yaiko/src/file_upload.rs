use multer::Multipart;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct FileUpload {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub data: Vec<u8>,
}

impl FileUpload {
    pub async fn save_to(&self, directory: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let path = PathBuf::from(directory).join(&self.filename);
        
        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        let mut file = fs::File::create(&path).await?;
        file.write_all(&self.data).await?;
        
        Ok(path)
    }
}

pub async fn parse_multipart(
    body: hyper::Body,
    content_type: &str,
) -> Result<Vec<FileUpload>, Box<dyn std::error::Error + Send + Sync>> {
    let boundary = content_type
        .split("boundary=")
        .nth(1)
        .ok_or("Missing boundary in multipart")?;

    let mut multipart = Multipart::new(body, boundary);
    let mut uploads = Vec::new();

    while let Some(mut field) = multipart.next_field().await? {
        let filename = field.file_name().map(|f| f.to_string());
        if let Some(filename) = filename {
            let content_type = field.content_type()
                .map(|ct| ct.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            let mut data = Vec::new();
            while let Some(chunk) = field.chunk().await? {
                data.extend_from_slice(&chunk);
            }

            uploads.push(FileUpload {
                filename: filename.to_string(),
                content_type,
                size: data.len(),
                data,
            });
        }
    }

    Ok(uploads)
}
