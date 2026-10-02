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
            // Inclusive end: adjacent segments may share a boundary
            // anchor, and the earlier (finer-interpolated) segment
            // resolves it correctly; the last segment's terminal
            // anchor is reachable.
            if capture < cs || capture > ce {
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

/// Identify loop occurrences: each located `position` is an actual
/// occurrence start in the capture. `loop_index` is the occurrence
/// ordinal in capture-time order; `occurrence_position` is the
/// detected position itself (not a synthetic grid); `source_phase`/
/// `source_time` are 0 because each occurrence plays the body from
/// its start. Occurrences are never collapsed into one fuzzy match.
pub fn identify_loop_occurrences(positions: &[u64], body_len: u64) -> Vec<LoopOccurrence> {
    if body_len == 0 {
        return Vec::new();
    }
    let mut sorted: Vec<u64> = positions.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted
        .iter()
        .enumerate()
        .map(|(i, &capture_time)| LoopOccurrence {
            loop_index: i as u32,
            occurrence_position: capture_time,
            source_phase: 0,
            source_time: 0,
            capture_time,
        })
        .collect()
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
    // Split where the local edge rate *changes*: consecutive edges
    // with the same slope stay in one piece, so a sustained drift
    // becomes a single Drift segment rather than fragments. The
    // boundary anchor is shared between adjacent pieces (half-open
    // ranges keep segments non-overlapping).
    let mut pieces: Vec<Vec<Anchor>> = Vec::new();
    let mut cur = vec![run[0]];
    let mut prev_edge_rate: Option<f64> = None;
    for w in run.windows(2) {
        let r = local_rate(w);
        let changed = match prev_edge_rate {
            Some(pr) => {
                r.is_finite() != pr.is_finite()
                    || (r.is_finite() && pr.is_finite() && (r - pr).abs() > rate_tolerance)
            }
            None => false,
        };
        if changed {
            pieces.push(std::mem::take(&mut cur));
            cur = vec![w[0]];
        }
        cur.push(w[1]);
        prev_edge_rate = Some(r);
    }
    if !cur.is_empty() {
        pieces.push(cur);
    }
    for piece in pieces {
        let ds_ = piece[piece.len() - 1]
            .source
            .saturating_sub(piece[0].source);
        let dc_ = piece[piece.len() - 1]
            .capture
            .saturating_sub(piece[0].capture);
        let rate = if ds_ == 0 {
            f64::INFINITY
        } else {
            dc_ as f64 / ds_ as f64
        };
        let kind = if piece.len() < min_run {
            // A short off-rate piece is a discrete jump: source
            // advancing faster than capture means skipped content.
            if ds_ > dc_ {
                SegmentKind::Skip
            } else {
                SegmentKind::Jitter
            }
        } else if rate.is_finite() && (rate - 1.0).abs() <= rate_tolerance {
            SegmentKind::Continuous
        } else {
            SegmentKind::Drift
        };
        segments.push(Segment {
            kind,
            rate: if ds_ == 0 { None } else { Some(rate) },
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
        assert_eq!(
            kinds,
            vec![
                SegmentKind::Drift,
                SegmentKind::Gap,
                SegmentKind::Continuous
            ]
        );
        // A uniform drift is one segment, never a stack of
        // overlapping fragments (half-open ranges may share an edge).
        assert!(
            map.segments
                .windows(2)
                .all(|w| w[1].capture_range[0] >= w[0].capture_range[1])
        );
        // A boundary anchor shared by two segments resolves through
        // the earlier segment; the terminal anchor is reachable.
        let drift_end = map.segments[0].capture_range[1];
        assert!(map.capture_to_source(drift_end).is_some());
        let terminal = map.segments.last().unwrap().capture_range[1];
        assert!(map.capture_to_source(terminal).is_some());
    }

    #[test]
    fn skip_forward_is_a_discrete_segment() {
        // rate-1 run, one source-forward jump (ds >> dc), rate-1 run.
        let mut anchors: Vec<_> = (0..6).map(|i| a(i * 100, 1000 + i * 100)).collect();
        anchors.extend((6..12).map(|i| a(400 + i * 100, 1000 + i * 100)));
        let map = build_time_map(&anchors, 0.05, 50000, 3);
        let kinds: Vec<_> = map.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SegmentKind::Continuous,
                SegmentKind::Skip,
                SegmentKind::Continuous
            ]
        );
    }
}
