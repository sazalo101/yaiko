//! Validated media export profiles for predictable video-editor output.

use crate::media_processing::MediaPath;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    UnsafePath,
    UnsupportedContainer,
    IncompatibleCodec,
    InvalidResolution,
    InvalidBitrate,
    InvalidDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    H265,
    Vp9,
    Av1,
}
impl VideoCodec {
    fn argument(self) -> &'static str {
        match self {
            Self::H264 => "libx264",
            Self::H265 => "libx265",
            Self::Vp9 => "libvpx-vp9",
            Self::Av1 => "libaom-av1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioCodec {
    Aac,
    Opus,
}
impl AudioCodec {
    fn argument(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Opus => "libopus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Mp4,
    Webm,
    Mkv,
}
impl Container {
    fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Mkv => "mkv",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportProfile {
    pub output: MediaPath,
    pub width: u16,
    pub height: u16,
    pub video_bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub container: Container,
    pub duration: Option<Duration>,
}

impl ExportProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        output: impl Into<PathBuf>,
        width: u16,
        height: u16,
        video_bitrate_kbps: u32,
        audio_bitrate_kbps: u32,
        video_codec: VideoCodec,
        audio_codec: AudioCodec,
        container: Container,
    ) -> Result<Self, ExportError> {
        let output = MediaPath::new(output).map_err(|_| ExportError::UnsafePath)?;
        if output.as_path().extension().and_then(|v| v.to_str()) != Some(container.extension()) {
            return Err(ExportError::UnsupportedContainer);
        }
        if !(16..=7680).contains(&width)
            || !(16..=4320).contains(&height)
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
        {
            return Err(ExportError::InvalidResolution);
        }
        if !(100..=100_000).contains(&video_bitrate_kbps)
            || !(16..=1_024).contains(&audio_bitrate_kbps)
        {
            return Err(ExportError::InvalidBitrate);
        }
        if !compatible(video_codec, audio_codec, container) {
            return Err(ExportError::IncompatibleCodec);
        }
        Ok(Self {
            output,
            width,
            height,
            video_bitrate_kbps,
            audio_bitrate_kbps,
            video_codec,
            audio_codec,
            container,
            duration: None,
        })
    }
    pub fn duration(mut self, duration: Duration) -> Result<Self, ExportError> {
        if duration.is_zero() || duration > Duration::from_secs(86_400) {
            return Err(ExportError::InvalidDuration);
        }
        self.duration = Some(duration);
        Ok(self)
    }
    pub fn command_line(&self) -> Vec<String> {
        let mut args = vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-y".into(),
            "-vf".into(),
            format!("scale={}:{}", self.width, self.height),
            "-c:v".into(),
            self.video_codec.argument().into(),
            "-b:v".into(),
            format!("{}k", self.video_bitrate_kbps),
            "-c:a".into(),
            self.audio_codec.argument().into(),
            "-b:a".into(),
            format!("{}k", self.audio_bitrate_kbps),
            "-movflags".into(),
            "+faststart".into(),
        ];
        if let Some(duration) = self.duration {
            args.extend(["-t".into(), duration.as_secs_f64().to_string()]);
        }
        args.push(self.output.display());
        args
    }
}

fn compatible(video: VideoCodec, audio: AudioCodec, container: Container) -> bool {
    match container {
        Container::Mp4 => {
            matches!(video, VideoCodec::H264 | VideoCodec::H265) && matches!(audio, AudioCodec::Aac)
        }
        Container::Webm => {
            matches!(video, VideoCodec::Vp9 | VideoCodec::Av1) && matches!(audio, AudioCodec::Opus)
        }
        Container::Mkv => {
            matches!(
                video,
                VideoCodec::H264 | VideoCodec::H265 | VideoCodec::Vp9 | VideoCodec::Av1
            ) && matches!(audio, AudioCodec::Aac | AudioCodec::Opus)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_mp4_export_profile() {
        let profile = ExportProfile::new(
            "renders/final.mp4",
            1920,
            1080,
            8000,
            192,
            VideoCodec::H264,
            AudioCodec::Aac,
            Container::Mp4,
        )
        .unwrap()
        .duration(Duration::from_secs(60))
        .unwrap();
        let args = profile.command_line();
        assert!(args.iter().any(|arg| arg == "scale=1920:1080"));
        assert!(args.iter().any(|arg| arg == "8000k"));
        assert!(args.iter().any(|arg| arg == "libx264"));
        assert!(args.iter().any(|arg| arg == "+faststart"));
    }
    #[test]
    fn validates_compatibility_resolution_bitrate_and_paths() {
        assert_eq!(
            ExportProfile::new(
                "out.webm",
                1920,
                1080,
                8000,
                128,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Webm
            ),
            Err(ExportError::IncompatibleCodec)
        );
        assert_eq!(
            ExportProfile::new(
                "../out.mp4",
                1920,
                1080,
                8000,
                128,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Mp4
            ),
            Err(ExportError::UnsafePath)
        );
        assert_eq!(
            ExportProfile::new(
                "out.mp4",
                1919,
                1080,
                8000,
                128,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Mp4
            ),
            Err(ExportError::InvalidResolution)
        );
        assert_eq!(
            ExportProfile::new(
                "out.mp4",
                1920,
                1080,
                99,
                128,
                VideoCodec::H264,
                AudioCodec::Aac,
                Container::Mp4
            ),
            Err(ExportError::InvalidBitrate)
        );
    }
    #[test]
    fn supports_webm_and_mkv_and_rejects_duration_overflow() {
        assert!(ExportProfile::new(
            "out.webm",
            1280,
            720,
            3000,
            128,
            VideoCodec::Vp9,
            AudioCodec::Opus,
            Container::Webm
        )
        .is_ok());
        assert!(ExportProfile::new(
            "out.mkv",
            1280,
            720,
            3000,
            128,
            VideoCodec::Av1,
            AudioCodec::Aac,
            Container::Mkv
        )
        .is_ok());
        let profile = ExportProfile::new(
            "out.mp4",
            1280,
            720,
            3000,
            128,
            VideoCodec::H264,
            AudioCodec::Aac,
            Container::Mp4,
        )
        .unwrap();
        assert_eq!(
            profile.duration(Duration::from_secs(86_401)),
            Err(ExportError::InvalidDuration)
        );
    }
}
