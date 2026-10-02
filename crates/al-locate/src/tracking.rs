//! Tracking: piecewise time maps over a source↔capture alignment.
//!
//! `Anchor`s pin individual correspondences; `Segment`s group them
//! into continuous regions annotated with the phenomenon they show
//! (drift, jitter, skip, gap, resume, unmatched). Loop occurrences
//! are identified individually — each keeps its loop index,
//! occurrence position, source phase/time, and capture time.

use serde::{Deserialize, Serialize};

/// What one segment of a time map expresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    /// Source plays continuously at its natural rate.
    Continuous,
    /// Playback rate differs: capture time drifts linearly.
    Drift,
    /// Local clock jitter around a stable mapping.
    Jitter,
    /// A forward jump: capture skips source content.
    Skip,
    /// A break where nothing maps (silence, cut).
    Gap,
    /// The source restarts after a gap.
    Resume,
    /// Capture content with no source correspondence.
    Unmatched,
}

/// One anchor: a source position ↔ capture position pair.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    /// Source sample position.
    pub source: u64,
    /// Capture sample position.
    pub capture: u64,
    /// Local confidence/quality of the anchor.
    pub score: f64,
}

/// One segment of a time map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    /// Phenomenon the segment expresses.
    pub kind: SegmentKind,
    /// Anchors spanning the segment (may be empty for Gap/Unmatched).
    pub anchors: Vec<Anchor>,
    /// Source range covered.
    pub source_range: [u64; 2],
    /// Capture range covered.
    pub capture_range: [u64; 2],
    /// Drift rate when kind is `Drift`/`Jitter` (dcapture/dsource).
    pub rate: Option<f64>,
}

/// A piecewise source→capture time map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeMap {
    /// Ordered, non-overlapping segments in capture-time order.
    pub segments: Vec<Segment>,
}

impl TimeMap {
    /// Map a capture time to a source time inside a mapping segment,
    /// or `None` where the segment has no correspondence.
    pub fn capture_to_source(&self, capture: u64) -> Option<u64> {
        for seg in &self.segments {
            let [cs, ce] = seg.capture_range;
            if capture < cs || capture >= ce {
                continue;
            }
            if seg.anchors.len() < 2 {
                return None;
            }
            // Linear interpolation between bracketing anchors.
            let pair = seg
                .anchors
                .windows(2)
                .find(|w| capture >= w[0].capture && capture <= w[1].capture);
            let w = pair?;
            let span = w[1].capture - w[0].capture;
            let frac = if span == 0 {
                0.0
            } else {
                (capture - w[0].capture) as f64 / span as f64
            };
            return Some(
                (w[0].source as f64 + frac * (w[1].source - w[0].source) as f64).round() as u64,
            );
        }
        None
    }
}

/// One identified loop occurrence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopOccurrence {
    /// Which repetition of the loop body (0 = first pass).
    pub loop_index: u32,
    /// Start position of this occurrence in the capture (samples).
    pub occurrence_position: u64,
    /// Phase of the source loop body at the occurrence start.
    pub source_phase: u64,
    /// Source time of the occurrence (samples, loop-relative).
    pub source_time: u64,
    /// Capture time of the occurrence (samples).
    pub capture_time: u64,
}

/// Identify loop occurrences: each acceptance `position` is an
/// occurrence of the loop body; `loop_index` = `position /
/// body_len`. Occurrences are never collapsed into one fuzzy match.
pub fn identify_loop_occurrences(positions: &[u64], body_len: u64) -> Vec<LoopOccurrence> {
    if body_len == 0 {
        return Vec::new();
    }
    let mut out: Vec<LoopOccurrence> = positions
        .iter()
        .map(|&capture_time| {
            let phase = capture_time % body_len;
            LoopOccurrence {
                loop_index: (capture_time / body_len) as u32,
                occurrence_position: capture_time - phase,
                source_phase: phase,
                source_time: phase,
                capture_time,
            }
        })
        .collect();
    out.sort_by_key(|o| o.capture_time);
    out.dedup_by_key(|o| (o.loop_index, o.source_phase));
    out
}

/// Segment a sorted anchor list into a piecewise `TimeMap`.
///
/// `rate_tolerance` is the allowed deviation of the local
/// dcapture/dsource slope from 1.0 before a segment is split into
/// `Drift`; slopes that cannot hold for `min_run` anchors are marked
/// `Jitter`. Gaps between anchor capture positions larger than
/// `gap_samples` become `Gap` segments; a source-position rewind
/// after a gap becomes `Resume`.
pub fn build_time_map(
    anchors: &[Anchor],
    rate_tolerance: f64,
    gap_samples: u64,
    min_run: usize,
) -> TimeMap {
    if anchors.is_empty() {
        return TimeMap { segments: vec![] };
    }
    let mut segments: Vec<Segment> = Vec::new();
    let mut run: Vec<Anchor> = vec![anchors[0]];
    for w in anchors.windows(2) {
        let (prev, next) = (w[0], w[1]);
        let gap = next.capture > prev.capture + gap_samples;
        let rewind = next.source < prev.source;
        if gap {
            finish_run(&mut segments, &mut run, rate_tolerance, min_run);
            segments.push(Segment {
                kind: SegmentKind::Gap,
                anchors: vec![],
                source_range: [prev.source, next.source],
                capture_range: [prev.capture, next.capture],
                rate: None,
            });
            run = vec![next];
            continue;
        }
        if rewind {
            finish_run(&mut segments, &mut run, rate_tolerance, min_run);
            segments.push(Segment {
                kind: SegmentKind::Resume,
                anchors: vec![next],
                source_range: [prev.source, next.source],
                capture_range: [prev.capture, next.capture],
                rate: None,
            });
            run = vec![next];
            continue;
        }
        run.push(next);
    }
    finish_run(&mut segments, &mut run, rate_tolerance, min_run);
    TimeMap { segments }
}

fn local_rate(w: &[Anchor]) -> f64 {
    if w.len() < 2 {
        return 1.0;
    }
    let ds = w[1].source.saturating_sub(w[0].source);
    let dc = w[1].capture.saturating_sub(w[0].capture);
    if ds == 0 {
        f64::INFINITY
    } else {
        dc as f64 / ds as f64
    }
}

fn finish_run(
    segments: &mut Vec<Segment>,
    run: &mut Vec<Anchor>,
    rate_tolerance: f64,
    min_run: usize,
) {
    if run.is_empty() {
        return;
    }
    // Split at flagged (non-unit-rate) boundaries.
    let mut pieces: Vec<Vec<Anchor>> = Vec::new();
    let mut cur = vec![run[0]];
    for w in run.windows(2) {
        let off_rate = (local_rate(w) - 1.0).abs() > rate_tolerance;
        if off_rate {
            cur.push(w[1]);
            pieces.push(std::mem::take(&mut cur));
            cur = vec![w[0]];
        }
        cur.push(w[1]);
    }
    if !cur.is_empty() {
        pieces.push(cur);
    }
    for piece in pieces {
        let kind = if piece.len() < min_run {
            SegmentKind::Jitter
        } else {
            let ds = piece[piece.len() - 1]
                .source
                .saturating_sub(piece[0].source);
            let dc = piece[piece.len() - 1]
                .capture
                .saturating_sub(piece[0].capture);
            let rate = if ds == 0 {
                f64::INFINITY
            } else {
                dc as f64 / ds as f64
            };
            if rate.is_finite() && (rate - 1.0).abs() <= rate_tolerance {
                SegmentKind::Continuous
            } else {
                SegmentKind::Drift
            }
        };
        let ds_ = piece[piece.len() - 1]
            .source
            .saturating_sub(piece[0].source);
        let dc_ = piece[piece.len() - 1]
            .capture
            .saturating_sub(piece[0].capture);
        segments.push(Segment {
            kind,
            rate: if ds_ == 0 {
                None
            } else {
                Some(dc_ as f64 / ds_ as f64)
            },
            source_range: [piece[0].source, piece[piece.len() - 1].source],
            capture_range: [piece[0].capture, piece[piece.len() - 1].capture],
            anchors: piece,
        });
    }
    run.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(s: u64, c: u64) -> Anchor {
        Anchor {
            source: s,
            capture: c,
            score: 1.0,
        }
    }

    #[test]
    fn loop_occurrences_are_individually_identified() {
        let occ = identify_loop_occurrences(&[0, 1000, 2000], 1000);
        assert_eq!(occ.len(), 3);
        assert_eq!(occ[1].loop_index, 1);
        assert_eq!(occ[2].loop_index, 2);
        assert_eq!(occ[2].source_phase, 0);
    }

    #[test]
    fn continuous_run_maps_linearly() {
        let anchors: Vec<_> = (0..10).map(|i| a(i * 100, 500 + i * 100)).collect();
        let map = build_time_map(&anchors, 0.05, 500, 3);
        assert_eq!(map.segments.len(), 1);
        assert_eq!(map.segments[0].kind, SegmentKind::Continuous);
        assert_eq!(map.capture_to_source(750), Some(250));
    }

    #[test]
    fn drift_and_gap_become_segments() {
        let mut anchors: Vec<_> = (0..10).map(|i| a(i * 100, i * 110)).collect(); // drift 1.1
        anchors.extend((10..15).map(|i| a(i * 100, 50000 + (i - 10) * 100)));
        let map = build_time_map(&anchors, 0.05, 5000, 3);
        let kinds: Vec<_> = map.segments.iter().map(|s| s.kind).collect();
        assert!(kinds.contains(&SegmentKind::Drift));
        assert!(kinds.contains(&SegmentKind::Gap));
    }
}
