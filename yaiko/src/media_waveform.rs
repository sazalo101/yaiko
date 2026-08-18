//! Deterministic waveform and chapter metadata for video-editor previews.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveformError {
    EmptySamples,
    TooManySamples,
    InvalidSample,
    InvalidRange,
    InvalidLabel,
    InvalidColor,
    Overlap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Waveform {
    pub samples: Vec<f32>,
    pub duration_ms: u64,
}

impl Waveform {
    pub fn new(samples: Vec<f32>, duration_ms: u64) -> Result<Self, WaveformError> {
        if samples.is_empty() {
            return Err(WaveformError::EmptySamples);
        }
        if samples.len() > 100_000 || duration_ms == 0 {
            return Err(WaveformError::TooManySamples);
        }
        if samples
            .iter()
            .any(|value| !value.is_finite() || !(-1.0..=1.0).contains(value))
        {
            return Err(WaveformError::InvalidSample);
        }
        Ok(Self {
            samples,
            duration_ms,
        })
    }
    pub fn normalized_peaks(&self) -> Vec<u8> {
        self.samples
            .iter()
            .map(|value| (value.abs().clamp(0.0, 1.0) * 255.0).round() as u8)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chapter {
    pub start_ms: u64,
    pub end_ms: u64,
    pub label: String,
    pub color: String,
}

impl Chapter {
    pub fn new(
        start_ms: u64,
        end_ms: u64,
        label: impl Into<String>,
        color: impl Into<String>,
    ) -> Result<Self, WaveformError> {
        let label = label.into();
        let color = color.into();
        if start_ms >= end_ms {
            return Err(WaveformError::InvalidRange);
        }
        if label.is_empty() || label.len() > 128 || label.chars().any(char::is_control) {
            return Err(WaveformError::InvalidLabel);
        }
        if !valid_color(&color) {
            return Err(WaveformError::InvalidColor);
        }
        Ok(Self {
            start_ms,
            end_ms,
            label,
            color,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterTrack {
    pub duration_ms: u64,
    pub chapters: Vec<Chapter>,
}

impl ChapterTrack {
    pub fn new(duration_ms: u64, mut chapters: Vec<Chapter>) -> Result<Self, WaveformError> {
        if duration_ms == 0 {
            return Err(WaveformError::InvalidRange);
        }
        chapters.sort_by_key(|chapter| chapter.start_ms);
        for window in chapters.windows(2) {
            if window[0].end_ms > window[1].start_ms {
                return Err(WaveformError::Overlap);
            }
        }
        if chapters.iter().any(|chapter| chapter.end_ms > duration_ms) {
            return Err(WaveformError::InvalidRange);
        }
        Ok(Self {
            duration_ms,
            chapters,
        })
    }
    pub fn serialized(&self) -> String {
        self.chapters
            .iter()
            .map(|chapter| {
                format!(
                    "{}-{}:{}:{}",
                    chapter.start_ms,
                    chapter.end_ms,
                    escape(&chapter.label),
                    chapter.color
                )
            })
            .collect::<Vec<_>>()
            .join("|")
    }
}

fn valid_color(value: &str) -> bool {
    value.len() == 7 && value.starts_with('#') && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(':', "\\:")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_waveform_peaks_deterministically() {
        let waveform = Waveform::new(vec![-1.0, -0.5, 0.0, 0.5, 1.0], 1000).unwrap();
        assert_eq!(waveform.normalized_peaks(), vec![255, 128, 0, 128, 255]);
    }
    #[test]
    fn validates_waveform_bounds_and_samples() {
        assert_eq!(
            Waveform::new(Vec::new(), 1000),
            Err(WaveformError::EmptySamples)
        );
        assert_eq!(
            Waveform::new(vec![2.0], 1000),
            Err(WaveformError::InvalidSample)
        );
        assert_eq!(
            Waveform::new(vec![0.1; 100_001], 1000),
            Err(WaveformError::TooManySamples)
        );
    }
    #[test]
    fn sorts_and_serializes_non_overlapping_chapters() {
        let chapters = ChapterTrack::new(
            10_000,
            vec![
                Chapter::new(5000, 9000, "End: Credits", "#00ff00").unwrap(),
                Chapter::new(0, 4000, "Intro", "#ff0000").unwrap(),
            ],
        )
        .unwrap();
        assert_eq!(
            chapters.serialized(),
            "0-4000:Intro:#ff0000|5000-9000:End\\: Credits:#00ff00"
        );
    }
    #[test]
    fn rejects_bad_ranges_labels_colors_and_overlap() {
        assert_eq!(
            Chapter::new(2, 1, "x", "#ffffff"),
            Err(WaveformError::InvalidRange)
        );
        assert_eq!(
            Chapter::new(0, 1, "x", "red"),
            Err(WaveformError::InvalidColor)
        );
        assert_eq!(
            ChapterTrack::new(
                10,
                vec![
                    Chapter::new(0, 6, "a", "#ffffff").unwrap(),
                    Chapter::new(5, 8, "b", "#ffffff").unwrap()
                ]
            ),
            Err(WaveformError::Overlap)
        );
    }
}
