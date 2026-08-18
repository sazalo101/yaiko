//! Bounded media MIME sniffing and extension consistency checks.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Mp4,
    Webm,
    Matroska,
    Mp3,
    Wav,
}

impl MediaKind {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Mp4 => "video/mp4",
            Self::Webm => "video/webm",
            Self::Matroska => "video/x-matroska",
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentTypeError {
    Empty,
    TooLarge,
    Unknown,
    ExtensionMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaContentType {
    pub kind: MediaKind,
    pub mime: &'static str,
    pub extension: &'static str,
}

pub fn sniff_media(
    data: &[u8],
    filename: &str,
    max_probe_bytes: usize,
) -> Result<MediaContentType, ContentTypeError> {
    if data.is_empty() || filename.is_empty() {
        return Err(ContentTypeError::Empty);
    }
    if max_probe_bytes == 0 || data.len() > max_probe_bytes || max_probe_bytes > 1_048_576 {
        return Err(ContentTypeError::TooLarge);
    }
    let kind = detect(data).ok_or(ContentTypeError::Unknown)?;
    let extension = Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(
        (kind, extension.as_str()),
        (MediaKind::Mp4, "mp4")
            | (MediaKind::Webm, "webm")
            | (MediaKind::Matroska, "mkv")
            | (MediaKind::Mp3, "mp3")
            | (MediaKind::Wav, "wav")
    ) {
        return Err(ContentTypeError::ExtensionMismatch);
    }
    Ok(MediaContentType {
        kind,
        mime: kind.mime(),
        extension: match kind {
            MediaKind::Mp4 => "mp4",
            MediaKind::Webm => "webm",
            MediaKind::Matroska => "mkv",
            MediaKind::Mp3 => "mp3",
            MediaKind::Wav => "wav",
        },
    })
}

fn detect(data: &[u8]) -> Option<MediaKind> {
    if data.len() >= 12 && &data[4..8] == b"ftyp" {
        return Some(MediaKind::Mp4);
    }
    if data.len() >= 4 && &data[..4] == b"\x1a\x45\xdf\xa3" {
        return Some(MediaKind::Matroska);
    }
    if data.len() >= 4 && &data[..4] == b"RIFF" && data.len() >= 12 && &data[8..12] == b"WAVE" {
        return Some(MediaKind::Wav);
    }
    if data.len() >= 3 && &data[..3] == b"ID3"
        || data.len() >= 2 && data[0] == 0xff && data[1] & 0xe0 == 0xe0
    {
        return Some(MediaKind::Mp3);
    }
    if data.len() >= 4 && &data[..4] == b"\x1a\x45\xdf\xa3" {
        return Some(MediaKind::Webm);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_supported_signatures_and_normalizes_mime() {
        let mut mp4 = vec![0; 12];
        mp4[4..8].copy_from_slice(b"ftyp");
        assert_eq!(
            sniff_media(&mp4, "video.MP4", 1024).unwrap().mime,
            "video/mp4"
        );
        assert_eq!(
            sniff_media(b"ID3data", "song.mp3", 1024).unwrap().kind,
            MediaKind::Mp3
        );
        assert_eq!(
            sniff_media(b"\x1a\x45\xdf\xa3data", "video.mkv", 1024)
                .unwrap()
                .kind,
            MediaKind::Matroska
        );
    }
    #[test]
    fn rejects_unknown_empty_oversized_and_mismatched_inputs() {
        assert_eq!(
            sniff_media(&[], "video.mp4", 1024),
            Err(ContentTypeError::Empty)
        );
        assert_eq!(
            sniff_media(b"unknown", "video.mp4", 1024),
            Err(ContentTypeError::Unknown)
        );
        assert_eq!(
            sniff_media(b"ID3data", "video.mp4", 1024),
            Err(ContentTypeError::ExtensionMismatch)
        );
        assert_eq!(
            sniff_media(b"ID3data", "song.mp3", 0),
            Err(ContentTypeError::TooLarge)
        );
    }
}
