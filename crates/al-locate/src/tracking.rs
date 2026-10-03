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
    /// Which repetition of the loop body, counted on the grid anchored
    /// at the reference position: 0 = the pass containing the anchor,
    /// negative = passes before it.
    pub loop_index: i64,
    /// Detected position of this occurrence in the capture (samples).
    pub occurrence_position: u64,
    /// Offset of the detection into its pass's window on the anchored
    /// grid (`0..body_len` samples).
    pub source_phase: u64,
    /// Source time of the detection relative to the pass start
    /// (samples, loop-relative).
    pub source_time: u64,
    /// Capture time of the occurrence (samples).
    pub capture_time: u64,
}

/// Identify loop occurrences: each accepted `position` is an
/// occurrence of the loop body. `anchor` is the reference capture
/// position defining pass 0 and phase 0 — typically the primary
/// accepted alignment — so `loop_index`/`source_phase` measure the
/// pass grid relative to a real detection, not relative to capture
/// sample 0. `occurrence_position` keeps the detected position as-is.
/// Occurrences are never collapsed into one fuzzy match.
pub fn identify_loop_occurrences(
    positions: &[u64],
    body_len: u64,
    anchor: u64,
) -> Vec<LoopOccurrence> {
    if body_len == 0 {
        return Vec::new();
    }
    let body = body_len as i64;
    positions
        .iter()
        .map(|&capture_time| {
            let rel = capture_time as i64 - anchor as i64;
            let source_phase = rel.rem_euclid(body) as u64;
            LoopOccurrence {
                loop_index: rel.div_euclid(body),
                occurrence_position: capture_time,
                source_phase,
                source_time: source_phase,
                capture_time,
            }
        })
        .collect()
}

/// Segment a sorted anchor list into a piecewise `TimeMap`.
///
/// `rate_tolerance` is the allowed deviation of the local
/// dcapture/dsource slope from 1.0 before a segment is split into
/// `Drift`; off-rate runs shorter than `min_run` anchors are marked
/// `Jitter` instead, and single anchors (no observable slope) stay
/// `Continuous`. `gap_samples` is the slack between the source and
/// capture advances of consecutive anchors: capture advancing
/// farther than the source by more than that slack becomes `Gap`
/// (unmapped capture), the source advancing farther becomes `Skip`
/// (skipped source content), and a source rewind becomes `Resume`
/// regardless of the capture distance — a restart is the more
/// specific observation.
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
        let dc = next.capture.saturating_sub(prev.capture);
        let ds = next.source.saturating_sub(prev.source);
        let kind = if next.source < prev.source {
            SegmentKind::Resume
        } else if dc > ds.saturating_add(gap_samples) {
            SegmentKind::Gap
        } else if ds > dc.saturating_add(gap_samples) {
            SegmentKind::Skip
        } else {
            run.push(next);
            continue;
        };
        finish_run(&mut segments, &mut run, rate_tolerance, min_run);
        segments.push(Segment {
            kind,
            anchors: if kind == SegmentKind::Resume {
                vec![next]
            } else {
                vec![]
            },
            source_range: [prev.source, next.source],
            capture_range: [prev.capture, next.capture],
            rate: None,
        });
        run = vec![next];
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
    // Split at rate-class transitions so each piece is a maximal run
    // of windows that are all on-rate or all off-rate. Consecutive
    // off-rate windows share one piece; piece boundaries reuse the
    // shared anchor so segments tile the run without overlapping.
    let mut pieces: Vec<Vec<Anchor>> = Vec::new();
    let mut cur = vec![run[0]];
    let mut cur_off = false;
    for w in run.windows(2) {
        let off_rate = (local_rate(w) - 1.0).abs() > rate_tolerance;
        if cur.len() > 1 && off_rate != cur_off {
            pieces.push(std::mem::take(&mut cur));
            cur = vec![w[0]];
        }
        cur.push(w[1]);
        cur_off = off_rate;
    }
    if !cur.is_empty() {
        pieces.push(cur);
    }
    for piece in pieces {
        // A single anchor has no observable slope: it marks a mapping,
        // not jitter. Multi-anchor pieces are classified by their
        // overall rate; an off-rate run shorter than `min_run` anchors
        // is a discrete jump — source advancing faster than capture
        // means skipped content — not sustained drift.
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
        let kind = if piece.len() < 2 || (rate.is_finite() && (rate - 1.0).abs() <= rate_tolerance)
        {
            SegmentKind::Continuous
        } else if piece.len() < min_run {
            if ds_ > dc_ {
                SegmentKind::Skip
            } else {
                SegmentKind::Jitter
            }
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
        let occ = identify_loop_occurrences(&[0, 1000, 2000], 1000, 0);
        assert_eq!(occ.len(), 3);
        assert_eq!(occ[1].loop_index, 1);
        assert_eq!(occ[2].loop_index, 2);
        assert_eq!(occ[2].source_phase, 0);
    }

    #[test]
    fn occurrences_keep_detected_positions_with_nonzero_start() {
        // A loop whose first detected pass starts at 500 must not be
        // snapped back to the capture-0 grid: positions and capture
        // times stay as detected, indices count from the anchor pass.
        let occ = identify_loop_occurrences(&[500, 1500, 2500], 1000, 500);
        assert_eq!(occ.len(), 3);
        assert_eq!(
            occ.iter()
                .map(|o| o.occurrence_position)
                .collect::<Vec<_>>(),
            vec![500, 1500, 2500]
        );
        assert_eq!(
            occ.iter().map(|o| o.loop_index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!(occ.iter().all(|o| o.source_phase == 0));
    }

    #[test]
    fn passes_before_the_anchor_count_negative() {
        let occ = identify_loop_occurrences(&[500, 1500, 2500], 1000, 1500);
        assert_eq!(
            occ.iter().map(|o| o.loop_index).collect::<Vec<_>>(),
            vec![-1, 0, 1]
        );
        assert_eq!(occ[0].occurrence_position, 500);
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

    #[test]
    fn consecutive_off_rate_windows_form_one_segment() {
        // A sustained 1.1x drift: every window is off-rate, but the
        // run must become a single Drift segment — not overlapping
        // pieces that duplicate anchors.
        let anchors: Vec<_> = (0..6).map(|i| a(i * 100, i * 110)).collect();
        let map = build_time_map(&anchors, 0.05, 500, 3);
        assert_eq!(map.segments.len(), 1);
        assert_eq!(map.segments[0].kind, SegmentKind::Drift);
        assert_eq!(map.segments[0].source_range, [0, 500]);
        assert_eq!(map.segments[0].capture_range, [0, 550]);
        let rate = map.segments[0].rate.unwrap();
        assert!((rate - 1.1).abs() < 1e-9);
    }

    #[test]
    fn mixed_rate_runs_split_at_transitions() {
        // on-rate, then off-rate, then on-rate windows: three
        // non-overlapping pieces, each a maximal same-class run.
        let mut anchors: Vec<_> = (0..4).map(|i| a(i * 100, i * 100)).collect();
        anchors.extend((4..8).map(|i| a(i * 100, 300 + (i - 3) * 115)));
        anchors.extend((8..12).map(|i| a(i * 100, 760 + (i - 7) * 100)));
        let map = build_time_map(&anchors, 0.05, 500, 3);
        let kinds: Vec<_> = map.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SegmentKind::Continuous,
                SegmentKind::Drift,
                SegmentKind::Continuous
            ]
        );
        for w in map.segments.windows(2) {
            assert!(
                w[1].capture_range[0] >= w[0].capture_range[1],
                "capture ranges overlap or run backwards: {:?}",
                map.segments
            );
            assert!(
                w[1].source_range[0] >= w[0].source_range[1],
                "source ranges overlap or run backwards: {:?}",
                map.segments
            );
        }
    }

    #[test]
    fn forward_source_jump_becomes_skip() {
        // Source jumps 600 samples forward while capture stays
        // contiguous: skipped source content, not a capture gap.
        let mut anchors: Vec<_> = (0..4).map(|i| a(i * 100, i * 100)).collect();
        anchors.extend((10..14).map(|i| a(i * 100, 300 + (i - 9) * 100)));
        let map = build_time_map(&anchors, 0.05, 200, 3);
        let kinds: Vec<_> = map.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SegmentKind::Continuous,
                SegmentKind::Skip,
                SegmentKind::Continuous
            ]
        );
        assert_eq!(map.segments[1].source_range, [300, 1000]);
    }

    #[test]
    fn rewind_is_resume_even_across_a_capture_gap() {
        // A restart is the more specific observation: a source rewind
        // is Resume even when the capture also jumped far.
        let anchors = vec![
            a(0, 0),
            a(100, 100),
            a(200, 200),
            a(50, 5000),
            a(150, 5100),
            a(250, 5200),
        ];
        let map = build_time_map(&anchors, 0.05, 200, 3);
        let kinds: Vec<_> = map.segments.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SegmentKind::Continuous,
                SegmentKind::Resume,
                SegmentKind::Continuous
            ]
        );
    }
}
