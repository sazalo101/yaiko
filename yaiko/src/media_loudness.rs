//! Measured loudness metadata validation for media audio pipelines.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Surround51,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoudnessError {
    NonFinite,
    LoudnessOutOfRange,
    TruePeakExceeded,
    UnsupportedChannels,
    InvalidDuration,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessPolicy {
    pub min_lufs: f32,
    pub max_lufs: f32,
    pub max_true_peak_db: f32,
    pub max_channels: u8,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoudnessMetadata {
    pub integrated_lufs: f32,
    pub true_peak_db: f32,
    pub channels: ChannelLayout,
    pub duration_ms: u64,
}

impl ChannelLayout {
    fn count(self) -> u8 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Surround51 => 6,
        }
    }
}
impl LoudnessPolicy {
    pub fn validate(&self, metadata: LoudnessMetadata) -> Result<(), LoudnessError> {
        if !metadata.integrated_lufs.is_finite() || !metadata.true_peak_db.is_finite() {
            return Err(LoudnessError::NonFinite);
        }
        if metadata.duration_ms == 0 {
            return Err(LoudnessError::InvalidDuration);
        }
        if metadata.integrated_lufs < self.min_lufs || metadata.integrated_lufs > self.max_lufs {
            return Err(LoudnessError::LoudnessOutOfRange);
        }
        if metadata.true_peak_db > self.max_true_peak_db {
            return Err(LoudnessError::TruePeakExceeded);
        }
        if metadata.channels.count() > self.max_channels {
            return Err(LoudnessError::UnsupportedChannels);
        }
        Ok(())
    }
    pub fn gain_to_target(
        &self,
        metadata: LoudnessMetadata,
        target_lufs: f32,
    ) -> Result<f32, LoudnessError> {
        self.validate(metadata)?;
        if !target_lufs.is_finite() || target_lufs < self.min_lufs || target_lufs > self.max_lufs {
            return Err(LoudnessError::LoudnessOutOfRange);
        }
        Ok(target_lufs - metadata.integrated_lufs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> LoudnessPolicy {
        LoudnessPolicy {
            min_lufs: -30.0,
            max_lufs: -8.0,
            max_true_peak_db: -1.0,
            max_channels: 2,
        }
    }
    fn metadata() -> LoudnessMetadata {
        LoudnessMetadata {
            integrated_lufs: -16.0,
            true_peak_db: -2.0,
            channels: ChannelLayout::Stereo,
            duration_ms: 1000,
        }
    }
    #[test]
    fn accepts_valid_loudness_and_calculates_gain() {
        let value = metadata();
        assert_eq!(policy().validate(value), Ok(()));
        assert_eq!(policy().gain_to_target(value, -14.0).unwrap(), 2.0);
    }
    #[test]
    fn rejects_nonfinite_ranges_peaks_channels_and_duration() {
        let mut value = metadata();
        value.integrated_lufs = f32::NAN;
        assert_eq!(policy().validate(value), Err(LoudnessError::NonFinite));
        value = metadata();
        value.integrated_lufs = -40.0;
        assert_eq!(
            policy().validate(value),
            Err(LoudnessError::LoudnessOutOfRange)
        );
        value = metadata();
        value.true_peak_db = 0.0;
        assert_eq!(
            policy().validate(value),
            Err(LoudnessError::TruePeakExceeded)
        );
        value = metadata();
        value.channels = ChannelLayout::Surround51;
        assert_eq!(
            policy().validate(value),
            Err(LoudnessError::UnsupportedChannels)
        );
        value = metadata();
        value.duration_ms = 0;
        assert_eq!(
            policy().validate(value),
            Err(LoudnessError::InvalidDuration)
        );
    }
}
