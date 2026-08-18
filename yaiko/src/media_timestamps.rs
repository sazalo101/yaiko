//! Presentation-timestamp validation and normalization for media previews.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimestampError {
    Empty,
    NonMonotonic,
    GapTooLarge,
    FrameRateExceeded,
    DurationMismatch,
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimestampPolicy {
    pub max_gap_ms: u64,
    pub max_frame_rate_milli: u32,
    pub duration_tolerance_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampReport {
    pub normalized_ms: Vec<u64>,
    pub duration_ms: u64,
    pub frame_rate_milli: u32,
}

pub fn validate_and_normalize(
    timestamps_ms: &[u64],
    declared_duration_ms: u64,
    policy: TimestampPolicy,
) -> Result<TimestampReport, TimestampError> {
    if timestamps_ms.is_empty() || declared_duration_ms == 0 {
        return Err(TimestampError::Empty);
    }
    if timestamps_ms
        .windows(2)
        .any(|window| window[1] <= window[0])
    {
        return Err(TimestampError::NonMonotonic);
    }
    if timestamps_ms
        .windows(2)
        .any(|window| window[1] - window[0] > policy.max_gap_ms)
    {
        return Err(TimestampError::GapTooLarge);
    }
    let span = timestamps_ms.last().copied().ok_or(TimestampError::Empty)?;
    let normalized_ms = timestamps_ms
        .iter()
        .map(|value| value.saturating_sub(timestamps_ms[0]))
        .collect::<Vec<_>>();
    let duration_ms = span.saturating_sub(timestamps_ms[0]);
    let tolerance = duration_ms.abs_diff(declared_duration_ms);
    if tolerance > policy.duration_tolerance_ms {
        return Err(TimestampError::DurationMismatch);
    }
    if timestamps_ms.len() > 1 && duration_ms > 0 {
        let rate_milli =
            ((timestamps_ms.len() - 1) as u128 * 1_000_000u128) / u128::from(duration_ms);
        if rate_milli > u128::from(policy.max_frame_rate_milli) {
            return Err(TimestampError::FrameRateExceeded);
        }
        return Ok(TimestampReport {
            normalized_ms,
            duration_ms,
            frame_rate_milli: rate_milli as u32,
        });
    }
    Ok(TimestampReport {
        normalized_ms,
        duration_ms,
        frame_rate_milli: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> TimestampPolicy {
        TimestampPolicy {
            max_gap_ms: 100,
            max_frame_rate_milli: 60_000,
            duration_tolerance_ms: 10,
        }
    }
    #[test]
    fn normalizes_monotonic_timestamps_and_reports_rate() {
        let report = validate_and_normalize(&[100, 150, 200], 100, policy()).unwrap();
        assert_eq!(report.normalized_ms, vec![0, 50, 100]);
        assert_eq!(report.duration_ms, 100);
        assert_eq!(report.frame_rate_milli, 20_000);
    }
    #[test]
    fn rejects_duplicates_gaps_rate_and_duration_mismatch() {
        assert_eq!(
            validate_and_normalize(&[0, 0, 10], 10, policy()),
            Err(TimestampError::NonMonotonic)
        );
        assert_eq!(
            validate_and_normalize(&[0, 101], 101, policy()),
            Err(TimestampError::GapTooLarge)
        );
        assert_eq!(
            validate_and_normalize(&[0, 10], 100, policy()),
            Err(TimestampError::DurationMismatch)
        );
        let strict = TimestampPolicy {
            max_frame_rate_milli: 1_000,
            ..policy()
        };
        assert_eq!(
            validate_and_normalize(&[0, 10, 20], 20, strict),
            Err(TimestampError::FrameRateExceeded)
        );
    }
    #[test]
    fn rejects_empty_and_zero_duration_inputs() {
        assert_eq!(
            validate_and_normalize(&[], 1, policy()),
            Err(TimestampError::Empty)
        );
        assert_eq!(
            validate_and_normalize(&[0], 0, policy()),
            Err(TimestampError::Empty)
        );
    }
}
