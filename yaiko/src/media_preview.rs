//! Safe thumbnail and preview extraction specifications.

use crate::media_processing::MediaPath;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewError {
    UnsafePath,
    UnsupportedFormat,
    InvalidSeek,
    InvalidDimensions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewFormat {
    Jpeg,
    Png,
    Webp,
}

impl PreviewFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
    fn codec(self) -> &'static str {
        match self {
            Self::Jpeg => "mjpeg",
            Self::Png => "png",
            Self::Webp => "libwebp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailSpec {
    pub input: MediaPath,
    pub output: MediaPath,
    pub seek: Duration,
    pub width: u16,
    pub height: u16,
    pub format: PreviewFormat,
}

impl ThumbnailSpec {
    pub fn new(
        input: impl Into<PathBuf>,
        output: impl Into<PathBuf>,
        seek: Duration,
        width: u16,
        height: u16,
        format: PreviewFormat,
    ) -> Result<Self, PreviewError> {
        let input = MediaPath::new(input).map_err(|_| PreviewError::UnsafePath)?;
        let output = MediaPath::new(output).map_err(|_| PreviewError::UnsafePath)?;
        if !matches!(
            input.as_path().extension().and_then(|v| v.to_str()),
            Some("mp4" | "mov" | "webm" | "mkv")
        ) {
            return Err(PreviewError::UnsupportedFormat);
        }
        if output.as_path().extension().and_then(|v| v.to_str()) != Some(format.extension()) {
            return Err(PreviewError::UnsupportedFormat);
        }
        if seek > Duration::from_secs(86_400) {
            return Err(PreviewError::InvalidSeek);
        }
        if !(16..=4096).contains(&width) || !(16..=4096).contains(&height) {
            return Err(PreviewError::InvalidDimensions);
        }
        Ok(Self {
            input,
            output,
            seek,
            width,
            height,
            format,
        })
    }
    pub fn command_line(&self) -> Vec<String> {
        vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-y".into(),
            "-ss".into(),
            format_duration(self.seek),
            "-i".into(),
            self.input.display(),
            "-frames:v".into(),
            "1".into(),
            "-vf".into(),
            format!(
                "scale={}:{}:force_original_aspect_ratio=decrease",
                self.width, self.height
            ),
            "-c:v".into(),
            self.format.codec().into(),
            self.output.display(),
        ]
    }
}

fn format_duration(duration: Duration) -> String {
    format!("{}.{:03}", duration.as_secs(), duration.subsec_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_safe_thumbnail_arguments() {
        let spec = ThumbnailSpec::new(
            "uploads/video.mp4",
            "previews/thumb.jpg",
            Duration::from_millis(1250),
            640,
            360,
            PreviewFormat::Jpeg,
        )
        .unwrap();
        let args = spec.command_line();
        assert!(args.windows(2).any(|pair| pair == ["-ss", "1.250"]));
        assert!(args
            .iter()
            .any(|arg| arg == "scale=640:360:force_original_aspect_ratio=decrease"));
        assert!(!args.join(" ").contains(";"));
    }
    #[test]
    fn rejects_invalid_seek_dimensions_paths_and_extensions() {
        assert_eq!(
            ThumbnailSpec::new(
                "../video.mp4",
                "thumb.jpg",
                Duration::ZERO,
                640,
                360,
                PreviewFormat::Jpeg
            ),
            Err(PreviewError::UnsafePath)
        );
        assert_eq!(
            ThumbnailSpec::new(
                "video.mp4",
                "thumb.png",
                Duration::ZERO,
                640,
                360,
                PreviewFormat::Jpeg
            ),
            Err(PreviewError::UnsupportedFormat)
        );
        assert_eq!(
            ThumbnailSpec::new(
                "video.mp4",
                "thumb.jpg",
                Duration::from_secs(86_401),
                640,
                360,
                PreviewFormat::Jpeg
            ),
            Err(PreviewError::InvalidSeek)
        );
        assert_eq!(
            ThumbnailSpec::new(
                "video.mp4",
                "thumb.jpg",
                Duration::ZERO,
                8,
                360,
                PreviewFormat::Jpeg
            ),
            Err(PreviewError::InvalidDimensions)
        );
    }
    #[test]
    fn supports_png_and_webp_outputs() {
        assert!(ThumbnailSpec::new(
            "video.webm",
            "thumb.png",
            Duration::ZERO,
            320,
            180,
            PreviewFormat::Png
        )
        .is_ok());
        assert_eq!(
            ThumbnailSpec::new(
                "video.mkv",
                "thumb.webp",
                Duration::ZERO,
                320,
                180,
                PreviewFormat::Webp
            )
            .unwrap()
            .format,
            PreviewFormat::Webp
        );
    }
}
