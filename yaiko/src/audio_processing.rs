//! Safe audio normalization and background-music ducking specifications.

use crate::media_processing::MediaPath;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioProcessingError {
    UnsafePath,
    UnsupportedFormat,
    InvalidGain,
    InvalidLoudness,
    InvalidDucking,
    InvalidDuration,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessTarget {
    pub integrated_lufs: f32,
    pub true_peak_db: f32,
}

impl LoudnessTarget {
    pub fn new(integrated_lufs: f32, true_peak_db: f32) -> Result<Self, AudioProcessingError> {
        if !integrated_lufs.is_finite()
            || !(-70.0..=0.0).contains(&integrated_lufs)
            || !true_peak_db.is_finite()
            || !(-20.0..=0.0).contains(&true_peak_db)
        {
            return Err(AudioProcessingError::InvalidLoudness);
        }
        Ok(Self {
            integrated_lufs,
            true_peak_db,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ducking {
    pub reduction_db: f32,
    pub attack: Duration,
    pub release: Duration,
}

impl Ducking {
    pub fn new(
        reduction_db: f32,
        attack: Duration,
        release: Duration,
    ) -> Result<Self, AudioProcessingError> {
        if !reduction_db.is_finite()
            || !(0.0..=24.0).contains(&reduction_db)
            || attack.is_zero()
            || release.is_zero()
            || attack > Duration::from_secs(10)
            || release > Duration::from_secs(30)
        {
            return Err(AudioProcessingError::InvalidDucking);
        }
        Ok(Self {
            reduction_db,
            attack,
            release,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioProcessingSpec {
    pub input: MediaPath,
    pub output: MediaPath,
    pub target: LoudnessTarget,
    pub gain_db: f32,
    pub ducking: Option<Ducking>,
    pub max_duration: Option<Duration>,
}

impl AudioProcessingSpec {
    pub fn new(
        input: impl Into<PathBuf>,
        output: impl Into<PathBuf>,
        target: LoudnessTarget,
    ) -> Result<Self, AudioProcessingError> {
        let input = MediaPath::new(input).map_err(|_| AudioProcessingError::UnsafePath)?;
        let output = MediaPath::new(output).map_err(|_| AudioProcessingError::UnsafePath)?;
        if !is_audio(input.as_path().extension().and_then(|v| v.to_str()))
            || !is_audio(output.as_path().extension().and_then(|v| v.to_str()))
        {
            return Err(AudioProcessingError::UnsupportedFormat);
        }
        Ok(Self {
            input,
            output,
            target,
            gain_db: 0.0,
            ducking: None,
            max_duration: None,
        })
    }
    pub fn gain(mut self, gain_db: f32) -> Result<Self, AudioProcessingError> {
        if !gain_db.is_finite() || !(-24.0..=24.0).contains(&gain_db) {
            return Err(AudioProcessingError::InvalidGain);
        }
        self.gain_db = gain_db;
        Ok(self)
    }
    pub fn duck(mut self, ducking: Ducking) -> Self {
        self.ducking = Some(ducking);
        self
    }
    pub fn max_duration(mut self, duration: Duration) -> Result<Self, AudioProcessingError> {
        if duration.is_zero() || duration > Duration::from_secs(86_400) {
            return Err(AudioProcessingError::InvalidDuration);
        }
        self.max_duration = Some(duration);
        Ok(self)
    }
    pub fn command_line(&self) -> Vec<String> {
        let mut filters = vec![format!(
            "loudnorm=I={:.1}:TP={:.1}:LRA=11",
            self.target.integrated_lufs, self.target.true_peak_db
        )];
        if self.gain_db != 0.0 {
            filters.push(format!("volume={:.2}dB", self.gain_db));
        }
        if let Some(ducking) = self.ducking {
            filters.push(format!(
                "sidechaincompress=threshold=0.125:ratio={:.2}:attack={:.3}:release={:.3}",
                10.0_f32.powf(ducking.reduction_db / 20.0),
                ducking.attack.as_secs_f64(),
                ducking.release.as_secs_f64()
            ));
        }
        let mut args = vec![
            "-hide_banner".into(),
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            self.input.display(),
            "-af".into(),
            filters.join(","),
            "-c:a".into(),
            "aac".into(),
        ];
        if let Some(duration) = self.max_duration {
            args.extend(["-t".into(), duration.as_secs_f64().to_string()]);
        }
        args.push(self.output.display());
        args
    }
}

fn is_audio(extension: Option<&str>) -> bool {
    matches!(extension, Some("mp3" | "wav" | "m4a" | "aac" | "ogg"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builds_normalization_and_ducking_filters() {
        let target = LoudnessTarget::new(-16.0, -1.5).unwrap();
        let ducking =
            Ducking::new(12.0, Duration::from_millis(50), Duration::from_millis(300)).unwrap();
        let spec = AudioProcessingSpec::new("uploads/music.mp3", "renders/music.m4a", target)
            .unwrap()
            .gain(3.0)
            .unwrap()
            .duck(ducking);
        let args = spec.command_line();
        assert!(args
            .iter()
            .any(|arg| arg.contains("loudnorm=I=-16.0:TP=-1.5")));
        assert!(args.iter().any(|arg| arg.contains("sidechaincompress")));
        assert!(args.iter().any(|arg| arg.contains("volume=3.00dB")));
    }
    #[test]
    fn rejects_unsafe_formats_and_bounds() {
        assert_eq!(
            LoudnessTarget::new(-90.0, -1.0),
            Err(AudioProcessingError::InvalidLoudness)
        );
        assert_eq!(
            Ducking::new(25.0, Duration::from_secs(1), Duration::from_secs(1)),
            Err(AudioProcessingError::InvalidDucking)
        );
        assert_eq!(
            AudioProcessingSpec::new(
                "../music.mp3",
                "out.m4a",
                LoudnessTarget::new(-16.0, -1.0).unwrap()
            ),
            Err(AudioProcessingError::UnsafePath)
        );
        assert_eq!(
            AudioProcessingSpec::new(
                "music.exe",
                "out.m4a",
                LoudnessTarget::new(-16.0, -1.0).unwrap()
            ),
            Err(AudioProcessingError::UnsupportedFormat)
        );
    }
    #[test]
    fn validates_gain_duration_and_argument_safety() {
        let target = LoudnessTarget::new(-14.0, -1.0).unwrap();
        let spec = AudioProcessingSpec::new("in.wav", "out.ogg", target).unwrap();
        assert_eq!(
            spec.clone().gain(30.0),
            Err(AudioProcessingError::InvalidGain)
        );
        assert_eq!(
            spec.clone().max_duration(Duration::ZERO),
            Err(AudioProcessingError::InvalidDuration)
        );
        assert!(!spec.command_line().join(" ").contains(";"));
    }
}
