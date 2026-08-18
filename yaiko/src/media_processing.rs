//! Safe media-processing specifications for FFmpeg-backed workers.
//!
//! This module only builds typed process arguments. Callers should execute the
//! returned program and arguments with `tokio::process::Command`, never through
//! a shell.

use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaError {
    EmptyPath,
    AbsolutePath,
    Traversal,
    UnsupportedExtension,
    InvalidText,
    InvalidDuration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPath(PathBuf);

impl MediaPath {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, MediaError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(MediaError::EmptyPath);
        }
        if path.is_absolute() {
            return Err(MediaError::AbsolutePath);
        }
        if path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(MediaError::Traversal);
        }
        Ok(Self(path))
    }
    pub fn as_path(&self) -> &Path {
        &self.0
    }
    pub fn display(&self) -> String {
        self.0.to_string_lossy().into_owned()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionTrack {
    pub file: MediaPath,
    pub language: Option<String>,
}

impl CaptionTrack {
    pub fn new(file: impl Into<PathBuf>, language: Option<String>) -> Result<Self, MediaError> {
        let file = MediaPath::new(file)?;
        if !matches!(
            file.as_path().extension().and_then(|v| v.to_str()),
            Some("srt" | "vtt" | "ass")
        ) {
            return Err(MediaError::UnsupportedExtension);
        }
        if language.as_deref().is_some_and(|value| {
            value.is_empty()
                || value.len() > 16
                || !value
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        }) {
            return Err(MediaError::InvalidText);
        }
        Ok(Self { file, language })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicTrack {
    pub file: MediaPath,
    pub volume_percent: u8,
}

impl MusicTrack {
    pub fn new(file: impl Into<PathBuf>, volume_percent: u8) -> Result<Self, MediaError> {
        let file = MediaPath::new(file)?;
        if !matches!(
            file.as_path().extension().and_then(|v| v.to_str()),
            Some("mp3" | "wav" | "m4a" | "aac" | "ogg")
        ) {
            return Err(MediaError::UnsupportedExtension);
        }
        Ok(Self {
            file,
            volume_percent,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FfmpegJobSpec {
    pub input: MediaPath,
    pub output: MediaPath,
    pub captions: Vec<CaptionTrack>,
    pub music: Option<MusicTrack>,
    pub max_duration: Option<Duration>,
}

impl FfmpegJobSpec {
    pub fn new(input: impl Into<PathBuf>, output: impl Into<PathBuf>) -> Result<Self, MediaError> {
        let input = MediaPath::new(input)?;
        let output = MediaPath::new(output)?;
        if !matches!(
            input.as_path().extension().and_then(|v| v.to_str()),
            Some("mp4" | "mov" | "webm" | "mkv")
        ) || !matches!(
            output.as_path().extension().and_then(|v| v.to_str()),
            Some("mp4" | "webm" | "mkv")
        ) {
            return Err(MediaError::UnsupportedExtension);
        }
        Ok(Self {
            input,
            output,
            captions: Vec::new(),
            music: None,
            max_duration: None,
        })
    }
    pub fn with_caption(mut self, caption: CaptionTrack) -> Self {
        self.captions.push(caption);
        self
    }
    pub fn with_music(mut self, music: MusicTrack) -> Self {
        self.music = Some(music);
        self
    }
    pub fn max_duration(mut self, duration: Duration) -> Result<Self, MediaError> {
        if duration.is_zero() || duration > Duration::from_secs(86_400) {
            return Err(MediaError::InvalidDuration);
        }
        self.max_duration = Some(duration);
        Ok(self)
    }
    pub fn command_line(&self) -> Vec<String> {
        let mut args = vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            self.input.display(),
        ];
        if let Some(music) = &self.music {
            args.extend([
                "-stream_loop".into(),
                "-1".into(),
                "-i".into(),
                music.file.display(),
            ]);
        }
        if !self.captions.is_empty() {
            args.extend([
                "-vf".into(),
                self.captions
                    .iter()
                    .map(|c| format!("subtitles={}", escape_filter_path(&c.file.display())))
                    .collect::<Vec<_>>()
                    .join(","),
            ]);
        }
        if let Some(music) = &self.music {
            args.extend([
                "-map".into(),
                "0:v:0".into(),
                "-map".into(),
                "1:a:0".into(),
                "-af".into(),
                format!("volume={:.2}", f32::from(music.volume_percent) / 100.0),
                "-shortest".into(),
            ]);
        }
        args.extend(["-c:v".into(), "libx264".into(), "-c:a".into(), "aac".into()]);
        if let Some(duration) = self.max_duration {
            args.extend(["-t".into(), duration.as_secs().to_string()]);
        }
        args.push(self.output.display());
        args
    }
}

fn escape_filter_path(path: &str) -> String {
    path.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_unsafe_paths_and_extensions() {
        assert_eq!(MediaPath::new("../secret"), Err(MediaError::Traversal));
        assert_eq!(
            MediaPath::new("/tmp/video.mp4"),
            Err(MediaError::AbsolutePath)
        );
        assert_eq!(
            MusicTrack::new("audio.exe", 100),
            Err(MediaError::UnsupportedExtension)
        );
    }
    #[test]
    fn builds_composition_without_shell_interpolation() {
        let spec = FfmpegJobSpec::new("uploads/input.mp4", "renders/output.mp4")
            .unwrap()
            .with_caption(CaptionTrack::new("uploads/captions.srt", Some("en-US".into())).unwrap())
            .with_music(MusicTrack::new("uploads/music.mp3", 50).unwrap());
        let args = spec.command_line();
        assert!(args.windows(2).any(|v| v == ["-map", "0:v:0"]));
        assert!(args.iter().any(|v| v == "volume=0.50"));
        assert!(!args.join(" ").contains("|"));
    }
    #[test]
    fn enforces_duration_bounds() {
        let spec = FfmpegJobSpec::new("input.mp4", "output.mp4").unwrap();
        assert_eq!(
            spec.clone().max_duration(Duration::ZERO),
            Err(MediaError::InvalidDuration)
        );
        assert!(spec
            .max_duration(Duration::from_secs(60))
            .unwrap()
            .max_duration
            .is_some());
    }
}
