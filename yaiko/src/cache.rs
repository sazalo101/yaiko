//! Cache abstractions and built-in Redis/in-memory backends.

use async_trait::async_trait;
use redis::{AsyncCommands, Client};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub type CacheResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Backend-neutral byte cache contract.
#[async_trait]
pub trait CacheStore: Send + Sync {
    async fn get_bytes(&self, key: &str) -> CacheResult<Option<Vec<u8>>>;
    async fn set_bytes(&self, key: &str, value: &[u8], ttl: Duration) -> CacheResult<()>;
    async fn delete(&self, key: &str) -> CacheResult<()>;
}

#[derive(Clone)]
struct MemoryEntry {
    value: Vec<u8>,
    expires_at: Instant,
}

/// Process-local cache backend for development and single-instance deployments.
#[derive(Default)]
pub struct MemoryCache {
    entries: RwLock<HashMap<String, MemoryEntry>>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self::default()
    }

    async fn purge_expired(&self) {
        let now = Instant::now();
        self.entries
            .write()
            .await
            .retain(|_, entry| entry.expires_at > now);
    }
}

#[async_trait]
impl CacheStore for MemoryCache {
    async fn get_bytes(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
        self.purge_expired().await;
        Ok(self
            .entries
            .read()
            .await
            .get(key)
            .map(|entry| entry.value.clone()))
    }

    async fn set_bytes(&self, key: &str, value: &[u8], ttl: Duration) -> CacheResult<()> {
        self.entries.write().await.insert(
            key.to_string(),
            MemoryEntry {
                value: value.to_vec(),
                expires_at: Instant::now() + ttl,
            },
        );
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        self.entries.write().await.remove(key);
        Ok(())
    }
}

/// Redis JSON cache backend retained for existing Yaiko applications.
#[derive(Clone)]
pub struct Cache {
    client: Client,
}

impl Cache {
    pub async fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = Client::open(redis_url)?;
        Ok(Cache { client })
    }

    pub async fn get<T: for<'de> serde::Deserialize<'de>>(
        &self,
        key: &str,
    ) -> CacheResult<Option<T>> {
        let mut conn = self.client.get_async_connection().await?;
        let value: Option<String> = conn.get(key).await?;

        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    pub async fn set<T: serde::Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        let mut conn = self.client.get_async_connection().await?;
        let serialized = serde_json::to_string(value)?;
        let _: () = conn.set_ex(key, serialized, ttl.as_secs() as usize).await?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> CacheResult<()> {
        let mut conn = self.client.get_async_connection().await?;
        let _: () = conn.del(key).await?;
        Ok(())
    }
}

#[async_trait]
impl CacheStore for Cache {
    async fn get_bytes(&self, key: &str) -> CacheResult<Option<Vec<u8>>> {
        let mut conn = self.client.get_async_connection().await?;
        let value: Option<Vec<u8>> = conn.get(key).await?;
        Ok(value)
    }

    async fn set_bytes(&self, key: &str, value: &[u8], ttl: Duration) -> CacheResult<()> {
        let mut conn = self.client.get_async_connection().await?;
        let _: () = conn.set_ex(key, value, ttl.as_secs() as usize).await?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        self.delete(key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_cache_round_trips_and_expires_values() {
        let cache = MemoryCache::new();
        cache
            .set_bytes("greeting", b"hello", Duration::from_millis(20))
            .await
            .unwrap();
        assert_eq!(
            cache.get_bytes("greeting").await.unwrap(),
            Some(b"hello".to_vec())
        );
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(cache.get_bytes("greeting").await.unwrap(), None);
    }
}
