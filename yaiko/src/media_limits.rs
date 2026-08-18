//! Cross-module media dimension and duration policy validation.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaLimitsError {
    InvalidDimensions,
    InvalidDuration,
    InvalidFrameRate,
    TooWide,
    TooTall,
    TooLong,
    TooShort,
    FrameRateExceeded,
    AspectRatioMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaLimits {
    pub max_width: u32,
    pub max_height: u32,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub max_frame_rate_milli: u32,
    pub max_aspect_ratio_milli: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaFacts {
    pub width: u32,
    pub height: u32,
    pub duration_ms: u64,
    pub frame_rate_milli: u32,
}

impl MediaLimits {
    pub fn validate(&self, facts: MediaFacts) -> Result<(), MediaLimitsError> {
        if facts.width == 0 || facts.height == 0 {
            return Err(MediaLimitsError::InvalidDimensions);
        }
        if facts.duration_ms == 0 {
            return Err(MediaLimitsError::InvalidDuration);
        }
        if facts.frame_rate_milli == 0 {
            return Err(MediaLimitsError::InvalidFrameRate);
        }
        if facts.width > self.max_width {
            return Err(MediaLimitsError::TooWide);
        }
        if facts.height > self.max_height {
            return Err(MediaLimitsError::TooTall);
        }
        if facts.duration_ms < self.min_duration_ms {
            return Err(MediaLimitsError::TooShort);
        }
        if facts.duration_ms > self.max_duration_ms {
            return Err(MediaLimitsError::TooLong);
        }
        if facts.frame_rate_milli > self.max_frame_rate_milli {
            return Err(MediaLimitsError::FrameRateExceeded);
        }
        let ratio = (facts.width as u64 * 1000) / facts.height as u64;
        if ratio > u64::from(self.max_aspect_ratio_milli) {
            return Err(MediaLimitsError::AspectRatioMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> MediaLimits {
        MediaLimits {
            max_width: 3840,
            max_height: 2160,
            min_duration_ms: 1000,
            max_duration_ms: 3_600_000,
            max_frame_rate_milli: 60_000,
            max_aspect_ratio_milli: 3_000,
        }
    }
    fn facts() -> MediaFacts {
        MediaFacts {
            width: 1920,
            height: 1080,
            duration_ms: 60_000,
            frame_rate_milli: 30_000,
        }
    }
    #[test]
    fn accepts_compatible_media_facts() {
        assert_eq!(limits().validate(facts()), Ok(()));
    }
    #[test]
    fn rejects_dimensions_duration_frame_rate_and_ratio() {
        let mut value = facts();
        value.width = 4000;
        assert_eq!(limits().validate(value), Err(MediaLimitsError::TooWide));
        value = facts();
        value.duration_ms = 500;
        assert_eq!(limits().validate(value), Err(MediaLimitsError::TooShort));
        value = facts();
        value.frame_rate_milli = 61_000;
        assert_eq!(
            limits().validate(value),
            Err(MediaLimitsError::FrameRateExceeded)
        );
        value = facts();
        value.width = 3840;
        value.height = 1000;
        assert_eq!(
            limits().validate(value),
            Err(MediaLimitsError::AspectRatioMismatch)
        );
    }
    #[test]
    fn rejects_zero_and_overflow_sensitive_values() {
        let mut value = facts();
        value.width = 0;
        assert_eq!(
            limits().validate(value),
            Err(MediaLimitsError::InvalidDimensions)
        );
        value = facts();
        value.height = 0;
        assert_eq!(
            limits().validate(value),
            Err(MediaLimitsError::InvalidDimensions)
        );
        value = facts();
        value.duration_ms = 3_600_001;
        assert_eq!(limits().validate(value), Err(MediaLimitsError::TooLong));
    }
}
