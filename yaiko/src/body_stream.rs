//! Bounded request-body streaming and backpressure helpers.

use hyper::{body::HttpBody, Body};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyStreamError {
    PayloadLimit,
    Timeout,
    Cancelled,
    Transport,
}

#[derive(Clone, Default)]
pub struct BodyCancellation {
    cancelled: Arc<AtomicBool>,
}

impl BodyCancellation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyReadReport {
    pub bytes: Vec<u8>,
    pub chunks: usize,
}

pub async fn read_bounded(
    mut body: Body,
    max_bytes: usize,
    chunk_timeout: Duration,
    cancellation: &BodyCancellation,
) -> Result<BodyReadReport, BodyStreamError> {
    let mut bytes = Vec::new();
    let mut chunks = 0;
    while let Some(next) = tokio::time::timeout(chunk_timeout, body.data())
        .await
        .map_err(|_| BodyStreamError::Timeout)?
    {
        if cancellation.is_cancelled() {
            return Err(BodyStreamError::Cancelled);
        }
        let chunk = next.map_err(|_| BodyStreamError::Transport)?;
        if bytes.len().saturating_add(chunk.len()) > max_bytes {
            return Err(BodyStreamError::PayloadLimit);
        }
        bytes.extend_from_slice(&chunk);
        chunks += 1;
    }
    Ok(BodyReadReport { bytes, chunks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Bytes;

    #[tokio::test]
    async fn reads_chunks_with_a_hard_payload_limit() {
        let cancellation = BodyCancellation::new();
        let report = read_bounded(
            Body::from(Bytes::from_static(b"hello")),
            8,
            Duration::from_secs(1),
            &cancellation,
        )
        .await
        .unwrap();
        assert_eq!(report.bytes, b"hello");
        assert_eq!(report.chunks, 1);
        assert_eq!(
            read_bounded(
                Body::from(Bytes::from_static(b"too large")),
                3,
                Duration::from_secs(1),
                &cancellation
            )
            .await,
            Err(BodyStreamError::PayloadLimit)
        );
    }

    #[tokio::test]
    async fn cancellation_is_observed_before_next_chunk() {
        let cancellation = BodyCancellation::new();
        cancellation.cancel();
        assert_eq!(
            read_bounded(
                Body::from("body"),
                100,
                Duration::from_secs(1),
                &cancellation
            )
            .await,
            Err(BodyStreamError::Cancelled)
        );
    }
}
