//! Bounded media metadata parsing and validation.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaMetadataError {
    TooLarge,
    InvalidJson,
    MissingFormat,
    InvalidDuration,
    DurationExceeded,
    MissingVideo,
    MissingAudio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaMetadata {
    pub duration_ms: u64,
    pub format_name: String,
    pub video_streams: u32,
    pub audio_streams: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaMetadataRequirements {
    pub max_duration_ms: u64,
    pub require_video: bool,
    pub require_audio: bool,
}

impl Default for MediaMetadataRequirements {
    fn default() -> Self {
        Self {
            max_duration_ms: 86_400_000,
            require_video: true,
            require_audio: false,
        }
    }
}

pub fn parse_and_validate(
    input: &[u8],
    requirements: MediaMetadataRequirements,
) -> Result<MediaMetadata, MediaMetadataError> {
    if input.len() > 64 * 1024 {
        return Err(MediaMetadataError::TooLarge);
    }
    let root: Value = serde_json::from_slice(input).map_err(|_| MediaMetadataError::InvalidJson)?;
    let format = root
        .get("format")
        .ok_or(MediaMetadataError::MissingFormat)?;
    let format_name = format
        .get("format_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(MediaMetadataError::MissingFormat)?
        .to_string();
    let duration_text = format
        .get("duration")
        .and_then(|value| value.as_str().or_else(|| value.as_f64().map(|_| "")))
        .ok_or(MediaMetadataError::InvalidDuration)?;
    let duration_secs = if duration_text.is_empty() {
        format
            .get("duration")
            .and_then(Value::as_f64)
            .ok_or(MediaMetadataError::InvalidDuration)?
    } else {
        duration_text
            .parse::<f64>()
            .map_err(|_| MediaMetadataError::InvalidDuration)?
    };
    if !duration_secs.is_finite() || duration_secs < 0.0 {
        return Err(MediaMetadataError::InvalidDuration);
    }
    let duration_ms = (duration_secs * 1000.0).round() as u64;
    if duration_ms > requirements.max_duration_ms {
        return Err(MediaMetadataError::DurationExceeded);
    }
    let streams = root
        .get("streams")
        .and_then(Value::as_array)
        .ok_or(MediaMetadataError::InvalidJson)?;
    let video_streams = streams
        .iter()
        .filter(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("video"))
        .count() as u32;
    let audio_streams = streams
        .iter()
        .filter(|stream| stream.get("codec_type").and_then(Value::as_str) == Some("audio"))
        .count() as u32;
    if requirements.require_video && video_streams == 0 {
        return Err(MediaMetadataError::MissingVideo);
    }
    if requirements.require_audio && audio_streams == 0 {
        return Err(MediaMetadataError::MissingAudio);
    }
    Ok(MediaMetadata {
        duration_ms,
        format_name,
        video_streams,
        audio_streams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> &'static [u8] {
        br#"{"format":{"duration":"12.345","format_name":"mov,mp4"},"streams":[{"codec_type":"video"},{"codec_type":"audio"}]}"#
    }
    #[test]
    fn parses_string_duration_and_streams() {
        let metadata = parse_and_validate(
            valid(),
            MediaMetadataRequirements {
                max_duration_ms: 20_000,
                require_video: true,
                require_audio: true,
            },
        )
        .unwrap();
        assert_eq!(metadata.duration_ms, 12_345);
        assert_eq!(metadata.video_streams, 1);
        assert_eq!(metadata.audio_streams, 1);
    }
    #[test]
    fn accepts_numeric_duration_and_rejects_limits() {
        let input = br#"{"format":{"duration":3.5,"format_name":"webm"},"streams":[{"codec_type":"video"}]}"#;
        assert_eq!(
            parse_and_validate(
                input,
                MediaMetadataRequirements {
                    max_duration_ms: 3_000,
                    ..Default::default()
                }
            ),
            Err(MediaMetadataError::DurationExceeded)
        );
        assert_eq!(
            parse_and_validate(
                input,
                MediaMetadataRequirements {
                    max_duration_ms: 4_000,
                    ..Default::default()
                }
            )
            .unwrap()
            .duration_ms,
            3_500
        );
    }
    #[test]
    fn rejects_malformed_and_missing_streams() {
        assert_eq!(
            parse_and_validate(b"{}", Default::default()),
            Err(MediaMetadataError::MissingFormat)
        );
        let no_video = br#"{"format":{"duration":"1","format_name":"mp4"},"streams":[{"codec_type":"audio"}]}"#;
        assert_eq!(
            parse_and_validate(no_video, Default::default()),
            Err(MediaMetadataError::MissingVideo)
        );
        let no_audio = br#"{"format":{"duration":"1","format_name":"mp4"},"streams":[{"codec_type":"video"}]}"#;
        assert_eq!(
            parse_and_validate(
                no_audio,
                MediaMetadataRequirements {
                    require_audio: true,
                    ..Default::default()
                }
            ),
            Err(MediaMetadataError::MissingAudio)
        );
    }
    #[test]
    fn rejects_large_and_non_finite_duration() {
        assert_eq!(
            parse_and_validate(&vec![b' '; 65 * 1024], Default::default()),
            Err(MediaMetadataError::TooLarge)
        );
        let invalid = br#"{"format":{"duration":"NaN","format_name":"mp4"},"streams":[{"codec_type":"video"}]}"#;
        assert_eq!(
            parse_and_validate(invalid, Default::default()),
            Err(MediaMetadataError::InvalidDuration)
        );
    }
}
