//! Storage abstractions for local files and future object-storage adapters.

use async_trait::async_trait;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;

type HmacSha256 = Hmac<Sha256>;
pub type StorageResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Backend-neutral byte storage contract.
#[async_trait]
pub trait Storage: Send + Sync {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()>;
    async fn get(&self, key: &str) -> StorageResult<Option<Vec<u8>>>;
    async fn delete(&self, key: &str) -> StorageResult<()>;
    async fn exists(&self, key: &str) -> StorageResult<bool>;
}

/// File metadata exposed by storage backends.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageMetadata {
    pub key: String,
    pub size: u64,
    pub checksum_sha256: String,
    pub content_type: String,
    pub modified_unix: Option<u64>,
}

/// HMAC-signed, expiring access token for a storage key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedAccess {
    pub key: String,
    pub expires_at: u64,
    pub signature: String,
}

impl SignedAccess {
    pub fn create(secret: &[u8], key: &str, expires_at: u64) -> StorageResult<Self> {
        let signature = sign(secret, key, expires_at)?;
        Ok(Self {
            key: key.to_string(),
            expires_at,
            signature,
        })
    }

    pub fn verify(&self, secret: &[u8], now_unix: u64) -> bool {
        self.expires_at >= now_unix
            && sign(secret, &self.key, self.expires_at)
                .map(|expected| constant_time_eq(expected.as_bytes(), self.signature.as_bytes()))
                .unwrap_or(false)
    }
}

fn sign(secret: &[u8], key: &str, expires_at: u64) -> StorageResult<String> {
    let mut mac = HmacSha256::new_from_slice(secret).map_err(|_| "invalid signing secret")?;
    mac.update(format!("{}\n{}", key, expires_at).as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
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

    pub async fn metadata(&self, key: &str) -> StorageResult<Option<StorageMetadata>> {
        let path = self.path_for(key)?;
        let metadata = match fs::metadata(&path).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let data = fs::read(&path).await?;
        let checksum_sha256 = format!("sha256:{:x}", Sha256::digest(&data));
        let modified_unix = metadata
            .modified()
            .ok()
            .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
            .map(|value| value.as_secs());
        Ok(Some(StorageMetadata {
            key: key.to_string(),
            size: metadata.len(),
            checksum_sha256,
            content_type: mime_guess::from_path(&path)
                .first_or_octet_stream()
                .essence_str()
                .to_string(),
            modified_unix,
        }))
    }

    pub fn signed_access(
        &self,
        secret: &[u8],
        key: &str,
        expires_at: u64,
    ) -> StorageResult<SignedAccess> {
        let _ = self.path_for(key)?;
        SignedAccess::create(secret, key, expires_at)
    }

    pub fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
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
    use crate::Storage;

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
    async fn metadata_and_signed_access_are_verifiable() {
        let directory = tempfile::tempdir().unwrap();
        let storage = LocalStorage::new(directory.path());
        storage.put("clips/sample.txt", b"hello").await.unwrap();

        let metadata = storage.metadata("clips/sample.txt").await.unwrap().unwrap();
        assert_eq!(metadata.size, 5);
        assert_eq!(metadata.content_type, "text/plain");
        assert!(metadata.checksum_sha256.starts_with("sha256:"));

        let access = storage
            .signed_access(b"secret", "clips/sample.txt", LocalStorage::now_unix() + 60)
            .unwrap();
        assert!(access.verify(b"secret", LocalStorage::now_unix()));
        assert!(!access.verify(b"wrong", LocalStorage::now_unix()));
    }

    #[tokio::test]
    async fn local_storage_rejects_traversal_keys() {
        let directory = tempfile::tempdir().unwrap();
        let storage = LocalStorage::new(directory.path());
        assert!(storage.put("../escape.txt", b"nope").await.is_err());
    }
}
