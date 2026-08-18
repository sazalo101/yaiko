//! Typed media timeline composition metadata and safe FFmpeg arguments.

use crate::media_processing::MediaPath;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineError {
    UnsafePath,
    UnsupportedFormat,
    EmptyTimeline,
    InvalidTrim,
    InvalidDuration,
    InvalidCaption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Cut,
    Fade { duration: Duration },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineClip {
    pub input: MediaPath,
    pub trim_start: Duration,
    pub trim_end: Option<Duration>,
    pub transition: Transition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionOverlay {
    pub text: String,
    pub start: Duration,
    pub duration: Duration,
}

impl CaptionOverlay {
    pub fn new(
        text: impl Into<String>,
        start: Duration,
        duration: Duration,
    ) -> Result<Self, TimelineError> {
        let text = text.into();
        if text.is_empty()
            || text.len() > 512
            || text.chars().any(char::is_control)
            || duration.is_zero()
        {
            return Err(TimelineError::InvalidCaption);
        }
        Ok(Self {
            text,
            start,
            duration,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineSpec {
    pub output: MediaPath,
    pub clips: Vec<TimelineClip>,
    pub captions: Vec<CaptionOverlay>,
    pub music: Option<MediaPath>,
    pub max_duration: Duration,
}

impl TimelineSpec {
    pub fn new(output: impl Into<PathBuf>) -> Result<Self, TimelineError> {
        let output = MediaPath::new(output).map_err(|_| TimelineError::UnsafePath)?;
        if !matches!(
            output.as_path().extension().and_then(|v| v.to_str()),
            Some("mp4" | "webm" | "mkv")
        ) {
            return Err(TimelineError::UnsupportedFormat);
        }
        Ok(Self {
            output,
            clips: Vec::new(),
            captions: Vec::new(),
            music: None,
            max_duration: Duration::from_secs(86_400),
        })
    }
    pub fn add_clip(
        mut self,
        input: impl Into<PathBuf>,
        trim_start: Duration,
        trim_end: Option<Duration>,
        transition: Transition,
    ) -> Result<Self, TimelineError> {
        let input = MediaPath::new(input).map_err(|_| TimelineError::UnsafePath)?;
        if !matches!(
            input.as_path().extension().and_then(|v| v.to_str()),
            Some("mp4" | "mov" | "webm" | "mkv")
        ) {
            return Err(TimelineError::UnsupportedFormat);
        }
        if trim_end.is_some_and(|end| end <= trim_start) {
            return Err(TimelineError::InvalidTrim);
        }
        if let Transition::Fade { duration } = transition {
            if duration.is_zero()
                || trim_end.is_some_and(|end| duration >= end.saturating_sub(trim_start))
            {
                return Err(TimelineError::InvalidDuration);
            }
        }
        self.clips.push(TimelineClip {
            input,
            trim_start,
            trim_end,
            transition,
        });
        Ok(self)
    }
    pub fn caption(mut self, caption: CaptionOverlay) -> Self {
        self.captions.push(caption);
        self
    }
    pub fn music(mut self, input: impl Into<PathBuf>) -> Result<Self, TimelineError> {
        let input = MediaPath::new(input).map_err(|_| TimelineError::UnsafePath)?;
        if !matches!(
            input.as_path().extension().and_then(|v| v.to_str()),
            Some("mp3" | "wav" | "m4a" | "aac" | "ogg")
        ) {
            return Err(TimelineError::UnsupportedFormat);
        }
        self.music = Some(input);
        Ok(self)
    }
    pub fn total_trimmed_duration(&self) -> Result<Duration, TimelineError> {
        if self.clips.is_empty() {
            return Err(TimelineError::EmptyTimeline);
        }
        let total = self
            .clips
            .iter()
            .map(|clip| {
                clip.trim_end
                    .unwrap_or(Duration::from_secs(86_400))
                    .saturating_sub(clip.trim_start)
            })
            .fold(Duration::ZERO, |sum, duration| sum.saturating_add(duration));
        Ok(total.min(self.max_duration))
    }
    pub fn command_line(&self) -> Result<Vec<String>, TimelineError> {
        if self.clips.is_empty() {
            return Err(TimelineError::EmptyTimeline);
        }
        let mut args = vec!["-hide_banner".into(), "-nostdin".into(), "-y".into()];
        for clip in &self.clips {
            args.extend(["-i".into(), clip.input.display()]);
        }
        if let Some(music) = &self.music {
            args.extend([
                "-stream_loop".into(),
                "-1".into(),
                "-i".into(),
                music.display(),
            ]);
        }
        let mut filters = Vec::new();
        for (index, clip) in self.clips.iter().enumerate() {
            let end = clip
                .trim_end
                .map(|value| format!(":end={:.3}", value.as_secs_f64()))
                .unwrap_or_default();
            filters.push(format!(
                "[{index}:v]trim=start={:.3}{end},setpts=PTS-STARTPTS[v{index}]",
                clip.trim_start.as_secs_f64()
            ));
        }
        let inputs = (0..self.clips.len())
            .map(|index| format!("[v{index}]"))
            .collect::<String>();
        filters.push(format!("{inputs}concat=n={}:v=1:a=0[v]", self.clips.len()));
        for caption in &self.captions {
            filters.push(format!(
                "[v]drawtext=text='{}':enable='between(t,{:.3},{:.3})'[v]",
                escape_drawtext(&caption.text),
                caption.start.as_secs_f64(),
                (caption.start + caption.duration).as_secs_f64()
            ));
        }
        args.extend(["-filter_complex".into(), filters.join(";")]);
        args.extend(["-map".into(), "[v]".into()]);
        if self.music.is_some() {
            args.extend([
                "-map".into(),
                format!("{}:a:0", self.clips.len()),
                "-shortest".into(),
            ]);
        }
        args.extend([
            "-c:v".into(),
            "libx264".into(),
            "-c:a".into(),
            "aac".into(),
            "-t".into(),
            self.total_trimmed_duration()?.as_secs_f64().to_string(),
            self.output.display(),
        ]);
        Ok(args)
    }
}

fn escape_drawtext(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace('%', "\\%")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn composes_ordered_clips_with_trim_and_caption() {
        let timeline = TimelineSpec::new("renders/final.mp4")
            .unwrap()
            .add_clip(
                "a.mp4",
                Duration::from_secs(1),
                Some(Duration::from_secs(4)),
                Transition::Cut,
            )
            .unwrap()
            .add_clip(
                "b.webm",
                Duration::ZERO,
                Some(Duration::from_secs(3)),
                Transition::Fade {
                    duration: Duration::from_millis(250),
                },
            )
            .unwrap()
            .caption(
                CaptionOverlay::new(
                    "Hello: 100%",
                    Duration::from_secs(1),
                    Duration::from_secs(2),
                )
                .unwrap(),
            );
        let args = timeline.command_line().unwrap();
        assert!(args.iter().any(|value| value.contains("concat=n=2")));
        assert!(args.iter().any(|value| value.contains("Hello\\: 100\\%")));
        assert_eq!(
            timeline.total_trimmed_duration().unwrap(),
            Duration::from_secs(6)
        );
    }
    #[test]
    fn rejects_invalid_paths_formats_trims_and_empty_timelines() {
        assert_eq!(
            TimelineSpec::new("../out.mp4"),
            Err(TimelineError::UnsafePath)
        );
        assert_eq!(
            TimelineSpec::new("out.jpg"),
            Err(TimelineError::UnsupportedFormat)
        );
        let timeline = TimelineSpec::new("out.mp4").unwrap();
        assert_eq!(timeline.command_line(), Err(TimelineError::EmptyTimeline));
        assert_eq!(
            timeline.add_clip(
                "clip.mp4",
                Duration::from_secs(4),
                Some(Duration::from_secs(4)),
                Transition::Cut
            ),
            Err(TimelineError::InvalidTrim)
        );
    }
    #[test]
    fn validates_music_and_caption_limits() {
        assert_eq!(
            CaptionOverlay::new("", Duration::ZERO, Duration::from_secs(1)),
            Err(TimelineError::InvalidCaption)
        );
        let timeline = TimelineSpec::new("out.webm").unwrap();
        assert_eq!(
            timeline.clone().music("music.exe"),
            Err(TimelineError::UnsupportedFormat)
        );
        assert!(timeline.music("music.mp3").is_ok());
    }
}
