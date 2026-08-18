//! In-memory resumable media-upload sessions with strict chunk validation.

use crate::media_processing::MediaPath;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadSessionError {
    UnsafePath,
    InvalidChunkCount,
    InvalidChunk,
    OutOfOrder,
    DuplicateChunk,
    ChecksumMismatch,
    Expired,
    MissingSession,
    Capacity,
    AlreadyCompleted,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedUpload {
    pub path: MediaPath,
    pub bytes: Vec<u8>,
    pub checksum_sha256: String,
}

#[derive(Debug, Clone)]
struct Session {
    path: MediaPath,
    total_chunks: u32,
    next_chunk: u32,
    chunks: BTreeMap<u32, Vec<u8>>,
    expires_at: SystemTime,
    completed: bool,
}

#[derive(Debug, Clone)]
pub struct ResumableUploadStore {
    inner: Arc<Mutex<HashMap<String, Session>>>,
    max_sessions: usize,
    max_chunk_bytes: usize,
}

impl ResumableUploadStore {
    pub fn new(max_sessions: usize, max_chunk_bytes: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_sessions: max_sessions.max(1),
            max_chunk_bytes: max_chunk_bytes.max(1),
        }
    }
    pub fn start(
        &self,
        id: impl Into<String>,
        path: impl Into<PathBuf>,
        total_chunks: u32,
        now: SystemTime,
        ttl: Duration,
    ) -> Result<(), UploadSessionError> {
        let id = id.into();
        let path = MediaPath::new(path).map_err(|_| UploadSessionError::UnsafePath)?;
        if id.is_empty()
            || id.len() > 128
            || total_chunks == 0
            || ttl.is_zero()
            || ttl > Duration::from_secs(86_400)
        {
            return Err(UploadSessionError::InvalidChunkCount);
        }
        let mut guard = self.inner.lock().unwrap();
        if guard.len() >= self.max_sessions || guard.contains_key(&id) {
            return Err(UploadSessionError::Capacity);
        }
        guard.insert(
            id,
            Session {
                path,
                total_chunks,
                next_chunk: 0,
                chunks: BTreeMap::new(),
                expires_at: now + ttl,
                completed: false,
            },
        );
        Ok(())
    }
    pub fn accept(
        &self,
        id: &str,
        index: u32,
        data: Vec<u8>,
        checksum_sha256: &str,
        now: SystemTime,
    ) -> Result<u32, UploadSessionError> {
        let mut guard = self.inner.lock().unwrap();
        let session = guard
            .get_mut(id)
            .ok_or(UploadSessionError::MissingSession)?;
        if now >= session.expires_at {
            return Err(UploadSessionError::Expired);
        }
        if session.completed {
            return Err(UploadSessionError::AlreadyCompleted);
        }
        if index >= session.total_chunks || data.is_empty() || data.len() > self.max_chunk_bytes {
            return Err(UploadSessionError::InvalidChunk);
        }
        if index != session.next_chunk {
            return Err(UploadSessionError::OutOfOrder);
        }
        if session.chunks.contains_key(&index) {
            return Err(UploadSessionError::DuplicateChunk);
        }
        let actual = format!("sha256:{:x}", Sha256::digest(&data));
        if actual != checksum_sha256 {
            return Err(UploadSessionError::ChecksumMismatch);
        }
        session.chunks.insert(index, data);
        session.next_chunk += 1;
        Ok(session.next_chunk)
    }
    pub fn complete(
        &self,
        id: &str,
        now: SystemTime,
    ) -> Result<CompletedUpload, UploadSessionError> {
        let mut guard = self.inner.lock().unwrap();
        let session = guard
            .get_mut(id)
            .ok_or(UploadSessionError::MissingSession)?;
        if now >= session.expires_at {
            return Err(UploadSessionError::Expired);
        }
        if session.completed {
            return Err(UploadSessionError::AlreadyCompleted);
        }
        if session.next_chunk != session.total_chunks {
            return Err(UploadSessionError::Incomplete);
        }
        let bytes = session
            .chunks
            .values()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let checksum_sha256 = format!("sha256:{:x}", Sha256::digest(&bytes));
        session.completed = true;
        Ok(CompletedUpload {
            path: session.path.clone(),
            bytes,
            checksum_sha256,
        })
    }
    pub fn cancel(&self, id: &str) -> Result<(), UploadSessionError> {
        self.inner
            .lock()
            .unwrap()
            .remove(id)
            .map(|_| ())
            .ok_or(UploadSessionError::MissingSession)
    }
    pub fn active_sessions(&self) -> usize {
        self.inner.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(10_000)
    }
    fn sum(data: &[u8]) -> String {
        format!("sha256:{:x}", Sha256::digest(data))
    }
    #[test]
    fn accepts_ordered_chunks_and_completes() {
        let store = ResumableUploadStore::new(4, 10);
        store
            .start(
                "upload-1",
                "uploads/video.mp4",
                2,
                now(),
                Duration::from_secs(60),
            )
            .unwrap();
        store
            .accept("upload-1", 0, b"abc".to_vec(), &sum(b"abc"), now())
            .unwrap();
        store
            .accept("upload-1", 1, b"def".to_vec(), &sum(b"def"), now())
            .unwrap();
        let completed = store.complete("upload-1", now()).unwrap();
        assert_eq!(completed.bytes, b"abcdef");
        assert_eq!(completed.path, MediaPath::new("uploads/video.mp4").unwrap());
    }
    #[test]
    fn enforces_order_checksum_duplicates_and_completion() {
        let store = ResumableUploadStore::new(4, 10);
        store
            .start("upload-1", "video.mp4", 2, now(), Duration::from_secs(60))
            .unwrap();
        assert_eq!(
            store.accept("upload-1", 1, b"b".to_vec(), &sum(b"b"), now()),
            Err(UploadSessionError::OutOfOrder)
        );
        assert_eq!(
            store.accept("upload-1", 0, b"a".to_vec(), "sha256:bad", now()),
            Err(UploadSessionError::ChecksumMismatch)
        );
        store
            .accept("upload-1", 0, b"a".to_vec(), &sum(b"a"), now())
            .unwrap();
        assert_eq!(
            store.accept("upload-1", 0, b"a".to_vec(), &sum(b"a"), now()),
            Err(UploadSessionError::OutOfOrder)
        );
        assert_eq!(
            store.complete("upload-1", now()),
            Err(UploadSessionError::Incomplete)
        );
    }
    #[test]
    fn enforces_expiry_capacity_and_cancellation() {
        let store = ResumableUploadStore::new(1, 2);
        store
            .start("upload-1", "video.mp4", 1, now(), Duration::from_secs(1))
            .unwrap();
        assert_eq!(
            store.start("upload-2", "video.mp4", 1, now(), Duration::from_secs(1)),
            Err(UploadSessionError::Capacity)
        );
        assert_eq!(
            store.accept(
                "upload-1",
                0,
                b"a".to_vec(),
                &sum(b"a"),
                now() + Duration::from_secs(2)
            ),
            Err(UploadSessionError::Expired)
        );
        store.cancel("upload-1").unwrap();
        assert_eq!(store.active_sessions(), 0);
    }
}
