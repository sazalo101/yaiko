//! Safe byte-range and conditional media delivery primitives.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64,
}

impl ByteRange {
    pub fn parse(header: &str, length: u64) -> Result<Self, RangeError> {
        if length == 0 || !header.starts_with("bytes=") || header.contains(',') {
            return Err(RangeError::Invalid);
        }
        let value = &header[6..];
        let (start, end) = value.split_once('-').ok_or(RangeError::Invalid)?;
        let (start, end) = if start.is_empty() {
            let suffix = end.parse::<u64>().map_err(|_| RangeError::Invalid)?;
            if suffix == 0 {
                return Err(RangeError::Unsatisfiable);
            }
            (length.saturating_sub(suffix), length - 1)
        } else {
            let start = start.parse::<u64>().map_err(|_| RangeError::Invalid)?;
            let end = if end.is_empty() {
                length - 1
            } else {
                end.parse::<u64>()
                    .map_err(|_| RangeError::Invalid)?
                    .min(length - 1)
            };
            (start, end)
        };
        if start >= length || start > end {
            return Err(RangeError::Unsatisfiable);
        }
        Ok(Self { start, end })
    }

    pub fn length(&self) -> u64 {
        self.end - self.start + 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeError {
    Invalid,
    Unsatisfiable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaAsset {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub etag: String,
}

impl MediaAsset {
    pub fn new(bytes: Vec<u8>, content_type: impl Into<String>) -> Self {
        let etag = format!("\"sha256:{:x}\"", Sha256::digest(&bytes));
        Self {
            bytes,
            content_type: content_type.into(),
            etag,
        }
    }

    pub fn checksum_sha256(&self) -> String {
        format!("sha256:{:x}", Sha256::digest(&self.bytes))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

pub struct MediaDelivery;

impl MediaDelivery {
    pub fn respond(
        asset: &MediaAsset,
        range: Option<&str>,
        if_none_match: Option<&str>,
        download_name: Option<&str>,
    ) -> MediaResponse {
        let mut headers = BTreeMap::from([
            ("Accept-Ranges".to_string(), "bytes".to_string()),
            ("Content-Type".to_string(), asset.content_type.clone()),
            ("ETag".to_string(), asset.etag.clone()),
        ]);
        if let Some(name) = download_name {
            headers.insert(
                "Content-Disposition".to_string(),
                format!("attachment; filename=\"{}\"", safe_filename(name)),
            );
        }
        if if_none_match
            .map(|value| value.trim() == asset.etag)
            .unwrap_or(false)
        {
            return MediaResponse {
                status: 304,
                headers,
                body: Vec::new(),
            };
        }
        if let Some(range_header) = range {
            match ByteRange::parse(range_header, asset.bytes.len() as u64) {
                Ok(range) => {
                    headers.insert("Content-Length".to_string(), range.length().to_string());
                    headers.insert(
                        "Content-Range".to_string(),
                        format!("bytes {}-{}/{}", range.start, range.end, asset.bytes.len()),
                    );
                    return MediaResponse {
                        status: 206,
                        headers,
                        body: asset.bytes[range.start as usize..=range.end as usize].to_vec(),
                    };
                }
                Err(RangeError::Unsatisfiable) => {
                    headers.insert(
                        "Content-Range".to_string(),
                        format!("bytes */{}", asset.bytes.len()),
                    );
                    return MediaResponse {
                        status: 416,
                        headers,
                        body: Vec::new(),
                    };
                }
                Err(RangeError::Invalid) => {}
            }
        }
        headers.insert("Content-Length".to_string(), asset.bytes.len().to_string());
        MediaResponse {
            status: 200,
            headers,
            body: asset.bytes.clone(),
        }
    }
}

fn safe_filename(name: &str) -> String {
    let filename = name.rsplit(['/', '\\']).next().unwrap_or("download.bin");
    let sanitized: String = filename
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "download.bin".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_suffix_and_open_ranges() {
        assert_eq!(
            ByteRange::parse("bytes=2-5", 10).unwrap(),
            ByteRange { start: 2, end: 5 }
        );
        assert_eq!(
            ByteRange::parse("bytes=-3", 10).unwrap(),
            ByteRange { start: 7, end: 9 }
        );
        assert_eq!(
            ByteRange::parse("bytes=7-", 10).unwrap(),
            ByteRange { start: 7, end: 9 }
        );
        assert_eq!(
            ByteRange::parse("bytes=10-", 10),
            Err(RangeError::Unsatisfiable)
        );
    }

    #[test]
    fn delivers_ranges_and_conditional_not_modified_responses() {
        let asset = MediaAsset::new(b"0123456789".to_vec(), "video/mp4");
        let partial =
            MediaDelivery::respond(&asset, Some("bytes=2-5"), None, Some("../clip final.mp4"));
        assert_eq!(partial.status, 206);
        assert_eq!(partial.body, b"2345");
        assert_eq!(partial.headers["Content-Range"], "bytes 2-5/10");
        assert_eq!(
            partial.headers["Content-Disposition"],
            "attachment; filename=\"clip_final.mp4\""
        );
        let not_modified = MediaDelivery::respond(&asset, None, Some(&asset.etag), None);
        assert_eq!(not_modified.status, 304);
        assert!(not_modified.body.is_empty());
    }

    #[test]
    fn rejects_unsatisfiable_ranges_and_preserves_checksum() {
        let asset = MediaAsset::new(b"abc".to_vec(), "text/plain");
        let response = MediaDelivery::respond(&asset, Some("bytes=9-10"), None, None);
        assert_eq!(response.status, 416);
        assert_eq!(response.headers["Content-Range"], "bytes */3");
        assert!(asset.checksum_sha256().starts_with("sha256:"));
    }
}
