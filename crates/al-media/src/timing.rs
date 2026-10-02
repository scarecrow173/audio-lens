//! CFR-free timing observation over timestamp sequences.
//!
//! The module consumes raw timestamp sequences (e.g. per-frame PTS) and
//! produces observations: interval series, drift, discontinuities, and
//! duplicate/drop candidates. It never assumes a nominal frame rate and
//! keeps audio and video clocks as separately addressable sequences.

use serde::{Deserialize, Serialize};

/// Which clock a timestamp sequence belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockKind {
    /// Audio stream clock.
    Audio,
    /// Video stream clock.
    Video,
}

/// A single discontinuity candidate in a timestamp sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Discontinuity {
    /// Index of the interval (between `pts[i]` and `pts[i+1]`).
    pub interval_index: usize,
    /// Observed interval in timebase ticks.
    pub observed_ticks: i64,
    /// Median interval used as the local reference.
    pub reference_ticks: i64,
}

/// Distribution summary of the interval series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalStats {
    /// Smallest observed interval (ticks).
    pub min_ticks: i64,
    /// Largest observed interval (ticks).
    pub max_ticks: i64,
    /// Median interval (ticks).
    pub median_ticks: i64,
    /// Mean interval (ticks).
    pub mean_ticks: f64,
}

/// Timing observations for one timestamp sequence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingObservation {
    /// Clock this sequence belongs to; clocks stay separate end to end.
    pub clock: ClockKind,
    /// Ticks per second of `pts`.
    pub timebase_hz: u64,
    /// Number of timestamps observed.
    pub timestamp_count: usize,
    /// Consecutive deltas `pts[i+1] - pts[i]` in ticks.
    pub intervals: Vec<i64>,
    /// Interval distribution summary (`None` for empty sequences).
    pub interval_stats: Option<IntervalStats>,
    /// Measured-vs-nominal drift in ppm, when a nominal interval was given.
    pub drift_ppm: Option<f64>,
    /// Intervals far outside the local median (gap / jump candidates).
    pub discontinuities: Vec<Discontinuity>,
    /// Indices where `pts[i] == pts[i-1]` (duplicate-frame candidates).
    pub duplicates: Vec<usize>,
    /// Indices where the interval is at least twice the local median
    /// (dropped-frame candidates).
    pub drop_candidates: Vec<usize>,
}

fn median(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

/// Observe a timestamp sequence without assuming a frame rate.
///
/// `pts` is a monotonically non-decreasing sequence in `timebase_hz` ticks.
/// `nominal_interval_ticks` is optional reference information only — it is
/// used to report drift and is never used to "fix" the series.
pub fn observe_timing(
    clock: ClockKind,
    timebase_hz: u64,
    pts: &[i64],
    nominal_interval_ticks: Option<f64>,
) -> TimingObservation {
    let intervals: Vec<i64> = pts.windows(2).map(|w| w[1] - w[0]).collect();
    let med = median(&intervals);
    let stats = if intervals.is_empty() {
        None
    } else {
        Some(IntervalStats {
            min_ticks: *intervals.iter().min().unwrap_or(&0),
            max_ticks: *intervals.iter().max().unwrap_or(&0),
            median_ticks: med,
            mean_ticks: intervals.iter().map(|v| *v as f64).sum::<f64>() / intervals.len() as f64,
        })
    };

    // A discontinuity is an interval at least 2x the median, or negative/
    // zero when the median is positive. The median keeps VFR variation from
    // triggering on itself.
    let mut discontinuities = Vec::new();
    let mut duplicates = Vec::new();
    let mut drop_candidates = Vec::new();
    for (i, &interval) in intervals.iter().enumerate() {
        if interval == 0 {
            duplicates.push(i + 1);
        } else if med > 0 && (interval >= med * 2 || interval < 0) {
            discontinuities.push(Discontinuity {
                interval_index: i,
                observed_ticks: interval,
                reference_ticks: med,
            });
            if interval >= med * 2 {
                drop_candidates.push(i);
            }
        }
    }

    let drift_ppm = nominal_interval_ticks.and_then(|nominal| {
        stats
            .as_ref()
            .and_then(|s| (nominal > 0.0).then(|| (s.mean_ticks - nominal) / nominal * 1e6))
    });

    TimingObservation {
        clock,
        timebase_hz,
        timestamp_count: pts.len(),
        intervals,
        interval_stats: stats,
        drift_ppm,
        discontinuities,
        duplicates,
        drop_candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_sequence_has_no_anomalies() {
        let pts: Vec<i64> = (0..100).map(|i| i * 1000).collect();
        let obs = observe_timing(ClockKind::Video, 90_000, &pts, Some(1000.0));
        assert_eq!(obs.timestamp_count, 100);
        assert_eq!(obs.intervals.len(), 99);
        assert!(obs.discontinuities.is_empty());
        assert!(obs.duplicates.is_empty());
        assert!(obs.drop_candidates.is_empty());
        assert_eq!(obs.drift_ppm, Some(0.0));
    }

    #[test]
    fn vfr_sequence_keeps_interval_distribution() {
        // Alternating 1000/2000-tick intervals (roughly 67.5fps average).
        let mut pts = vec![0i64];
        for i in 0..50 {
            pts.push(pts[i] + if i % 2 == 0 { 1000 } else { 2000 });
        }
        let obs = observe_timing(ClockKind::Video, 90_000, &pts, None);
        let stats = obs.interval_stats.unwrap();
        assert_eq!(stats.min_ticks, 1000);
        assert_eq!(stats.max_ticks, 2000);
        // Median of a mixed series must not collapse to a nominal rate.
        assert!(obs.discontinuities.is_empty());
    }

    #[test]
    fn gap_is_recorded_as_discontinuity_and_drop() {
        let mut pts: Vec<i64> = (0..30).map(|i| i * 1000).collect();
        for p in pts.iter_mut().skip(15) {
            *p += 10_000; // jump mid-sequence
        }
        let obs = observe_timing(ClockKind::Video, 90_000, &pts, None);
        assert_eq!(obs.discontinuities.len(), 1);
        assert_eq!(obs.discontinuities[0].interval_index, 14);
        assert_eq!(obs.drop_candidates, vec![14]);
    }

    #[test]
    fn duplicate_pts_is_recorded() {
        let mut pts: Vec<i64> = (0..10).map(|i| i * 1000).collect();
        pts[5] = pts[4];
        let obs = observe_timing(ClockKind::Video, 90_000, &pts, None);
        assert_eq!(obs.duplicates, vec![5]);
    }

    #[test]
    fn drift_is_reported_against_nominal() {
        // 1% slow clock: intervals of 1010 vs nominal 1000.
        let pts: Vec<i64> = (0..100).map(|i| i * 1010).collect();
        let obs = observe_timing(ClockKind::Audio, 48_000, &pts, Some(1000.0));
        let ppm = obs.drift_ppm.unwrap();
        assert!((ppm - 10_000.0).abs() < 1.0);
    }

    #[test]
    fn clocks_stay_separate() {
        let audio = observe_timing(ClockKind::Audio, 48_000, &[0, 960, 1920], None);
        let video = observe_timing(ClockKind::Video, 90_000, &[0, 3000], None);
        assert_eq!(audio.clock, ClockKind::Audio);
        assert_eq!(video.clock, ClockKind::Video);
    }

    #[test]
    fn empty_sequence_is_an_explicit_empty_result() {
        let obs = observe_timing(ClockKind::Video, 90_000, &[], None);
        assert_eq!(obs.timestamp_count, 0);
        assert!(obs.interval_stats.is_none());
        assert!(obs.intervals.is_empty());
    }
}
