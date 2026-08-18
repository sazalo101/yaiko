//! Bounded waveform serialization for media-editor clients.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaveformSerializationError {
    Empty,
    TooManySamples,
    InvalidAmplitude,
    InvalidChapter,
    MetadataTooLarge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaveformChapter {
    pub at_ms: u64,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct WireWaveform<'a> {
    samples: &'a [u16],
    duration_ms: u64,
    chapters: &'a [WaveformChapter],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerializedWaveform {
    pub json: String,
    pub sample_count: usize,
    pub duration_ms: u64,
}

pub fn serialize_waveform(
    samples: &[f32],
    duration_ms: u64,
    chapters: &[WaveformChapter],
    max_samples: usize,
    max_bytes: usize,
) -> Result<SerializedWaveform, WaveformSerializationError> {
    if samples.is_empty() || duration_ms == 0 {
        return Err(WaveformSerializationError::Empty);
    }
    if samples.len() > max_samples || max_samples == 0 {
        return Err(WaveformSerializationError::TooManySamples);
    }
    if chapters.iter().any(|chapter| {
        chapter.at_ms > duration_ms
            || chapter.label.is_empty()
            || chapter.label.len() > 128
            || chapter.label.chars().any(|c| c.is_control())
    }) {
        return Err(WaveformSerializationError::InvalidChapter);
    }
    let quantized = samples
        .iter()
        .map(|sample| {
            if !sample.is_finite() || !(-1.0..=1.0).contains(sample) {
                return Err(WaveformSerializationError::InvalidAmplitude);
            }
            Ok(((sample.abs() * 65535.0).round()) as u16)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let wire = WireWaveform {
        samples: &quantized,
        duration_ms,
        chapters,
    };
    let json =
        serde_json::to_string(&wire).map_err(|_| WaveformSerializationError::MetadataTooLarge)?;
    if json.len() > max_bytes || max_bytes == 0 {
        return Err(WaveformSerializationError::MetadataTooLarge);
    }
    Ok(SerializedWaveform {
        json,
        sample_count: samples.len(),
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serializes_deterministic_quantized_waveform() {
        let chapters = vec![WaveformChapter {
            at_ms: 0,
            label: "intro".into(),
        }];
        let a = serialize_waveform(&[-1.0, 0.5, 0.0], 1000, &chapters, 10, 1024).unwrap();
        let b = serialize_waveform(&[-1.0, 0.5, 0.0], 1000, &chapters, 10, 1024).unwrap();
        assert_eq!(a, b);
        assert!(a.json.contains("65535"));
        assert_eq!(a.sample_count, 3);
    }
    #[test]
    fn rejects_invalid_samples_chapters_and_bounds() {
        assert_eq!(
            serialize_waveform(&[], 1000, &[], 10, 1024),
            Err(WaveformSerializationError::Empty)
        );
        assert_eq!(
            serialize_waveform(&[f32::NAN], 1000, &[], 10, 1024),
            Err(WaveformSerializationError::InvalidAmplitude)
        );
        assert_eq!(
            serialize_waveform(&[2.0], 1000, &[], 10, 1024),
            Err(WaveformSerializationError::InvalidAmplitude)
        );
        assert_eq!(
            serialize_waveform(
                &[0.5],
                1000,
                &[WaveformChapter {
                    at_ms: 1001,
                    label: "bad".into()
                }],
                10,
                1024
            ),
            Err(WaveformSerializationError::InvalidChapter)
        );
        assert_eq!(
            serialize_waveform(&[0.5; 11], 1000, &[], 10, 1024),
            Err(WaveformSerializationError::TooManySamples)
        );
    }
}
