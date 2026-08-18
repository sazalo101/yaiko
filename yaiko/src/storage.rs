//! Storage abstractions for local files and future object-storage adapters.

use async_trait::async_trait;
use std::path::{Component, Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub type StorageResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Backend-neutral byte storage contract.
#[async_trait]
pub trait Storage: Send + Sync {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()>;
    async fn get(&self, key: &str) -> StorageResult<Option<Vec<u8>>>;
    async fn delete(&self, key: &str) -> StorageResult<()>;
    async fn exists(&self, key: &str) -> StorageResult<bool>;
}

/// Local filesystem storage with traversal-safe keys.
#[derive(Debug, Clone)]
pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_for(&self, key: &str) -> StorageResult<PathBuf> {
        let path = Path::new(key);
        if path.is_absolute()
            || path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
        {
            return Err("storage key must be a relative, traversal-safe path".into());
        }
        Ok(self.root.join(path))
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
        let path = self.path_for(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::File::create(path).await?;
        file.write_all(data).await?;
        file.flush().await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> StorageResult<Option<Vec<u8>>> {
        let path = self.path_for(key)?;
        match fs::read(path).await {
            Ok(data) => Ok(Some(data)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    async fn delete(&self, key: &str) -> StorageResult<()> {
        let path = self.path_for(key)?;
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn exists(&self, key: &str) -> StorageResult<bool> {
        Ok(fs::metadata(self.path_for(key)?).await.is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_storage_round_trips_bytes_and_deletes() {
        let directory = tempfile::tempdir().unwrap();
        let storage = LocalStorage::new(directory.path());

        storage.put("clips/sample.bin", b"hello").await.unwrap();
        assert!(storage.exists("clips/sample.bin").await.unwrap());
        assert_eq!(
            storage.get("clips/sample.bin").await.unwrap(),
            Some(b"hello".to_vec())
        );

        storage.delete("clips/sample.bin").await.unwrap();
        assert!(!storage.exists("clips/sample.bin").await.unwrap());
    }

    #[tokio::test]
    async fn local_storage_rejects_traversal_keys() {
        let directory = tempfile::tempdir().unwrap();
        let storage = LocalStorage::new(directory.path());
        assert!(storage.put("../escape.txt", b"nope").await.is_err());
    }
}
