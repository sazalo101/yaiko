//! Audio sample-rate and channel-layout policy validation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSampleRate {
    Hz8000,
    Hz16000,
    Hz44100,
    Hz48000,
    Hz96000,
}
impl AudioSampleRate {
    pub fn hz(self) -> u32 {
        match self {
            Self::Hz8000 => 8_000,
            Self::Hz16000 => 16_000,
            Self::Hz44100 => 44_100,
            Self::Hz48000 => 48_000,
            Self::Hz96000 => 96_000,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannels {
    Mono,
    Stereo,
    Surround51,
}
impl AudioChannels {
    pub fn count(self) -> u8 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Surround51 => 6,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AudioFormatError {
    UnsupportedRate,
    UnsupportedChannels,
    InvalidPolicy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormatPolicy {
    pub max_channels: u8,
    pub allow_96khz: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFormat {
    pub rate: AudioSampleRate,
    pub channels: AudioChannels,
}
impl AudioFormatPolicy {
    pub fn validate(&self, format: AudioFormat) -> Result<(), AudioFormatError> {
        if self.max_channels == 0 {
            return Err(AudioFormatError::InvalidPolicy);
        }
        if format.channels.count() > self.max_channels {
            return Err(AudioFormatError::UnsupportedChannels);
        }
        if matches!(format.rate, AudioSampleRate::Hz96000) && !self.allow_96khz {
            return Err(AudioFormatError::UnsupportedRate);
        }
        Ok(())
    }
    pub fn ffmpeg_args(&self, format: AudioFormat) -> Result<Vec<String>, AudioFormatError> {
        self.validate(format)?;
        Ok(vec![
            "-ar".into(),
            format.rate.hz().to_string(),
            "-ac".into(),
            format.channels.count().to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> AudioFormatPolicy {
        AudioFormatPolicy {
            max_channels: 2,
            allow_96khz: false,
        }
    }
    #[test]
    fn accepts_supported_format_and_builds_args() {
        let format = AudioFormat {
            rate: AudioSampleRate::Hz48000,
            channels: AudioChannels::Stereo,
        };
        assert_eq!(policy().validate(format), Ok(()));
        assert_eq!(
            policy().ffmpeg_args(format).unwrap(),
            vec!["-ar", "48000", "-ac", "2"]
        );
    }
    #[test]
    fn rejects_rate_channels_and_invalid_policy() {
        let format = AudioFormat {
            rate: AudioSampleRate::Hz96000,
            channels: AudioChannels::Stereo,
        };
        assert_eq!(
            policy().validate(format),
            Err(AudioFormatError::UnsupportedRate)
        );
        let format = AudioFormat {
            rate: AudioSampleRate::Hz48000,
            channels: AudioChannels::Surround51,
        };
        assert_eq!(
            policy().validate(format),
            Err(AudioFormatError::UnsupportedChannels)
        );
        let invalid = AudioFormatPolicy {
            max_channels: 0,
            allow_96khz: true,
        };
        assert_eq!(
            invalid.validate(AudioFormat {
                rate: AudioSampleRate::Hz48000,
                channels: AudioChannels::Mono
            }),
            Err(AudioFormatError::InvalidPolicy)
        );
    }
}
