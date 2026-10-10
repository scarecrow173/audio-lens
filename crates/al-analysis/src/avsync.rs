//! A/V event synchronization: match cheap audio onsets against the
//! ingest-side `video_features` event list and measure the offset.
//!
//! 音声 PCM の開始を共通原点とする。映像 tick は自身の timebase で秒に
//! 変換してから音声 stream の start_tick/timebase を引く。音声 onset は
//! PCM 開始からの秒のままなので、auds/vids の実相対 offset を保持する。
//!
//! Matching is deterministic nearest-neighbor: every onset↔event
//! candidate inside `match_window_s` is considered, pairs are greedily
//! accepted in ascending |delta| order, and each side is used at most
//! once. Missing evidence surfaces as invalid records — never as
//! silence or stage failure.

use al_core::{Confidence, Validity};
use al_dataset::types::{FeatureRecord, ResolutionClass};
use al_media::video::{VideoEventKind, VideoFeatureArtifact, VideoFeatures};

use crate::records::{RecordBuilder, robust_summary};

/// `avsync` tunables (`config.analysis.avsync.*` overrides).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvSyncConfig {
    /// Maximum |video_time − audio_time| for a matched pair, seconds.
    pub match_window_s: f64,
}

impl Default for AvSyncConfig {
    fn default() -> Self {
        Self {
            match_window_s: 0.5,
        }
    }
}

impl AvSyncConfig {
    /// Overlay `config.analysis.avsync` onto the defaults.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let mut cfg = Self::default();
        if let Some(v) = config["analysis"]["avsync"]["match_window_s"].as_f64() {
            cfg.match_window_s = v;
        }
        cfg
    }
}

/// 共通の音声 PCM 原点へ変換した映像イベント。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VideoEventS {
    /// Event classification (kept verbatim from ingest).
    pub kind: VideoEventKind,
    /// Frame index in the decoded series.
    pub frame: u32,
    /// Seconds since the audio PCM origin.
    pub time_s: f64,
}

/// 映像 stream の tick を音声 PCM 原点からの秒へ変換する。
/// nominal FPS を使用せず、両 stream の宣言時計の相対 offset を保つ。
pub fn rebase_events(video: &VideoFeatures, audio_start_s: f64) -> Vec<VideoEventS> {
    video
        .events
        .iter()
        .map(|e| VideoEventS {
            kind: e.kind,
            frame: e.frame,
            time_s: e.time_ticks as f64 / video.timebase_hz as f64 - audio_start_s,
        })
        .collect()
}

/// One accepted onset↔event pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AvSyncPair {
    /// Onset position, seconds from PCM start.
    pub audio_time_s: f64,
    /// Event position, seconds since the audio PCM origin.
    pub video_time_s: f64,
    /// `video_time_s − audio_time_s` — the A/V offset estimate.
    pub delta_s: f64,
    /// Event classification of the video side.
    pub event_kind: VideoEventKind,
    /// Frame index of the video side.
    pub event_frame: u32,
    /// Confidence derived from |delta| as a fraction of the window.
    pub confidence: Confidence,
}

/// Confidence decays with the fraction of the match window the delta
/// consumes: within 25% is high, within 60% medium, beyond that low.
fn pair_confidence(abs_delta: f64, window_s: f64) -> Confidence {
    let frac = if window_s > 0.0 {
        abs_delta / window_s
    } else {
        f64::INFINITY
    };
    if frac <= 0.25 {
        Confidence::High
    } else if frac <= 0.6 {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

/// Greedily match `onsets` (seconds from PCM start) to `events`
/// (audio-PCM-origin seconds) one-to-one within `window_s`. Pairs are
/// returned in audio-time order.
pub fn match_events(onsets: &[f64], events: &[VideoEventS], window_s: f64) -> Vec<AvSyncPair> {
    // Every candidate inside the window, nearest first — greedy
    // one-to-one acceptance in that order is deterministic.
    let mut candidates: Vec<(f64, usize, usize)> = Vec::new();
    for (ai, &a) in onsets.iter().enumerate() {
        for (ei, e) in events.iter().enumerate() {
            let d = (e.time_s - a).abs();
            if d <= window_s {
                candidates.push((d, ai, ei));
            }
        }
    }
    candidates.sort_by(|x, y| {
        x.0.total_cmp(&y.0)
            .then_with(|| x.1.cmp(&y.1))
            .then_with(|| x.2.cmp(&y.2))
    });
    let mut used_onset = vec![false; onsets.len()];
    let mut used_event = vec![false; events.len()];
    let mut pairs = Vec::new();
    for (abs_d, ai, ei) in candidates {
        if used_onset[ai] || used_event[ei] {
            continue;
        }
        used_onset[ai] = true;
        used_event[ei] = true;
        let delta = events[ei].time_s - onsets[ai];
        pairs.push(AvSyncPair {
            audio_time_s: onsets[ai],
            video_time_s: events[ei].time_s,
            delta_s: delta,
            event_kind: events[ei].kind,
            event_frame: events[ei].frame,
            confidence: pair_confidence(abs_d, window_s),
        });
    }
    pairs.sort_by(|a, b| a.audio_time_s.total_cmp(&b.audio_time_s));
    pairs
}

/// Least-squares slope of `y` over `x`; `None` when fewer than two
/// distinct abscissae exist (a slope would be undefined).
fn linear_fit_slope(xs: &[f64], ys: &[f64]) -> Option<f64> {
    let n = xs.len().min(ys.len());
    if n < 2 {
        return None;
    }
    let mx = xs[..n].iter().sum::<f64>() / n as f64;
    let my = ys[..n].iter().sum::<f64>() / n as f64;
    let denom: f64 = xs[..n].iter().map(|x| (x - mx) * (x - mx)).sum();
    if denom == 0.0 {
        return None;
    }
    let numer: f64 = xs[..n]
        .iter()
        .zip(&ys[..n])
        .map(|(x, y)| (x - mx) * (y - my))
        .sum();
    Some(numer / denom)
}

/// Emit this input's `avsync.*` records.
///
/// `onset_times` is `None` when the input produced no decoded PCM
/// (`DecodeInputUnavailable`); `video` is `None` when no `vids` stream
/// exists at all (`NotApplicable`). An `Unavailable` artifact is
/// `DecodeInputUnavailable` on the video side. In every case the
/// series-level records are emitted with an explicit invalidity —
/// nothing is silently omitted.
pub fn sync_records(
    input_id: &str,
    onset_times: Option<&[f64]>,
    video: Option<&VideoFeatureArtifact>,
    audio_start_s: f64,
    cfg: &AvSyncConfig,
    lineage: Vec<String>,
) -> Vec<FeatureRecord> {
    let builder =
        RecordBuilder::new(input_id, "avsync.v1", ResolutionClass::Segment).lineage(lineage);
    let mut records = Vec::new();

    // Why no series-level measurement exists, if any. Audio evidence
    // missing dominates: without onsets there is nothing to sync.
    let blocked: Option<(Validity, String)> = if onset_times.is_none() {
        Some((
            Validity::DecodeInputUnavailable,
            "audio onsets unavailable: no decoded_pcm".into(),
        ))
    } else {
        match video {
            None => Some((Validity::NotApplicable, "input has no video stream".into())),
            Some(VideoFeatureArtifact::Unavailable { reason, .. }) => Some((
                Validity::DecodeInputUnavailable,
                format!("video features unavailable: {reason}"),
            )),
            Some(VideoFeatureArtifact::Decoded(v))
                if v.timebase_hz == 0 || !audio_start_s.is_finite() =>
            {
                Some((
                    Validity::ComputationFailed,
                    "A/V stream clock origin unavailable or invalid".into(),
                ))
            }
            Some(VideoFeatureArtifact::Decoded(_)) => None,
        }
    };
    if let Some((validity, reason)) = &blocked {
        records.push(builder.scalar(
            "avsync.pair_count",
            0.0,
            "count",
            None,
            Confidence::Low,
            *validity,
            Some(reason.clone()),
            0,
        ));
        for id in [
            "avsync.median_delta",
            "avsync.delta_mad",
            "avsync.drift_slope",
        ] {
            records.push(builder.scalar(
                id,
                f64::NAN,
                if id == "avsync.drift_slope" {
                    "s_per_s"
                } else {
                    "s"
                },
                None,
                Confidence::Low,
                *validity,
                Some(reason.clone()),
                0,
            ));
        }
        return records;
    }

    let onsets = onset_times.unwrap_or(&[]);
    let video = match video {
        Some(VideoFeatureArtifact::Decoded(v)) => v,
        _ => unreachable!("blocked cases returned above"),
    };
    let events = rebase_events(video, audio_start_s);
    let pairs = match_events(onsets, &events, cfg.match_window_s);

    // Per-pair evidence: each accepted pair contributes audio_time,
    // video_time, and delta records tagged with the pair index and the
    // video event's classification.
    for (i, p) in pairs.iter().enumerate() {
        let params = serde_json::json!({
            "pair": i,
            "event_kind": p.event_kind,
            "event_frame": p.event_frame,
            "clock_origin": "audio_pcm_start",
            "audio_start_s": audio_start_s,
        });
        for (id, value) in [
            ("avsync.audio_time", p.audio_time_s),
            ("avsync.video_time", p.video_time_s),
            ("avsync.delta", p.delta_s),
        ] {
            let mut rec =
                builder.scalar(id, value, "s", None, p.confidence, Validity::Valid, None, 1);
            rec.params = params.clone();
            records.push(rec);
        }
    }

    // Series-level summary over the matched deltas.
    let support = pairs.len() as u64;
    let summary_validity = if pairs.is_empty() {
        Validity::InsufficientSupport
    } else {
        Validity::Valid
    };
    let summary_reason = pairs
        .is_empty()
        .then(|| "no onset↔event pair inside match window".to_string());
    let deltas: Vec<f64> = pairs.iter().map(|p| p.delta_s).collect();
    let stats = robust_summary(&deltas);
    let slope = linear_fit_slope(
        &pairs.iter().map(|p| p.audio_time_s).collect::<Vec<_>>(),
        &deltas,
    );

    records.push(builder.scalar(
        "avsync.pair_count",
        pairs.len() as f64,
        "count",
        None,
        Confidence::High,
        Validity::Valid,
        None,
        support,
    ));
    let mut rec = builder.scalar(
        "avsync.median_delta",
        stats.map(|s| s.median).unwrap_or(f64::NAN),
        "s",
        None,
        Confidence::Medium,
        summary_validity,
        summary_reason.clone(),
        support,
    );
    rec.params = serde_json::json!({"match_window_s": cfg.match_window_s});
    records.push(rec);
    records.push(builder.scalar(
        "avsync.delta_mad",
        stats.map(|s| s.mad).unwrap_or(f64::NAN),
        "s",
        None,
        Confidence::Medium,
        summary_validity,
        summary_reason.clone(),
        support,
    ));
    records.push(builder.scalar(
        "avsync.drift_slope",
        slope.unwrap_or(f64::NAN),
        "s_per_s",
        None,
        Confidence::Medium,
        if pairs.len() >= 2 && slope.is_some() {
            Validity::Valid
        } else {
            Validity::InsufficientSupport
        },
        if pairs.len() < 2 || slope.is_none() {
            Some("need ≥2 pairs for a drift fit".into())
        } else {
            summary_reason
        },
        support,
    ));
    records.push(builder.scalar(
        "avsync.unmatched_onsets",
        (onsets.len() - pairs.len()) as f64,
        "count",
        None,
        Confidence::High,
        Validity::Valid,
        None,
        onsets.len() as u64,
    ));
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_media::video::{VideoEvent, VideoFeatureConfig};

    fn features(events: &[(i64, VideoEventKind)], start_tick: i64) -> VideoFeatures {
        features_at(events, start_tick, 10)
    }

    fn features_at(
        events: &[(i64, VideoEventKind)],
        start_tick: i64,
        timebase_hz: u64,
    ) -> VideoFeatures {
        VideoFeatures {
            stream: 0,
            codec: "rawvideo".into(),
            width: 4,
            height: 2,
            bits_per_pixel: 24,
            timebase_hz,
            start_tick,
            frame_timestamps: (0..8).map(|i| start_tick + i).collect(),
            luminance: vec![0.0; 8],
            diff: vec![0.0; 8],
            events: events
                .iter()
                .enumerate()
                .map(|(i, &(tick, kind))| VideoEvent {
                    kind,
                    frame: i as u32,
                    time_ticks: tick,
                })
                .collect(),
            params: VideoFeatureConfig::default(),
        }
    }

    fn scalar(rec: &FeatureRecord) -> f64 {
        match rec.value {
            al_dataset::types::Value::Scalar(v) => v,
            _ => panic!("expected scalar"),
        }
    }

    fn find<'a>(records: &'a [FeatureRecord], id: &str) -> &'a FeatureRecord {
        records
            .iter()
            .find(|r| r.feature_id == id)
            .unwrap_or_else(|| panic!("missing record {id}"))
    }

    #[test]
    fn known_offset_pairs_and_summarizes() {
        // Video events at 0.5/1.5/2.5 s; onsets 0.1 s later — every
        // pair must measure delta = −0.1 s.
        let v = features(
            &[
                (5, VideoEventKind::SceneChange),
                (15, VideoEventKind::MotionOnset),
                (25, VideoEventKind::SceneChange),
            ],
            0,
        );
        let onsets = [0.6, 1.6, 2.6];
        let records = sync_records(
            "IN-1",
            Some(&onsets),
            Some(&VideoFeatureArtifact::Decoded(v)),
            0.0,
            &AvSyncConfig::default(),
            vec![],
        );
        assert_eq!(scalar(find(&records, "avsync.pair_count")), 3.0);
        let median = find(&records, "avsync.median_delta");
        assert!((scalar(median) + 0.1).abs() < 1e-9);
        assert_eq!(scalar(find(&records, "avsync.delta_mad")), 0.0);
        // Per-pair ids exist: 3 pairs × 3 records.
        let deltas: Vec<f64> = records
            .iter()
            .filter(|r| r.feature_id == "avsync.delta")
            .map(scalar)
            .collect();
        assert_eq!(deltas.len(), 3);
        assert!(deltas.iter().all(|d| (d + 0.1).abs() < 1e-9));
        assert_eq!(
            records
                .iter()
                .filter(|r| r.feature_id == "avsync.audio_time")
                .count(),
            3
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.feature_id == "avsync.video_time")
                .count(),
            3
        );
    }

    #[test]
    fn equal_stream_origins_cancel_without_discarding_relative_offset() {
        // 両 stream の共通シフト 10 s のみが相殺される。
        let v = features(&[(105, VideoEventKind::SceneChange)], 100);
        let onsets = [0.6];
        let records = sync_records(
            "IN-1",
            Some(&onsets),
            Some(&VideoFeatureArtifact::Decoded(v)),
            10.0,
            &AvSyncConfig::default(),
            vec![],
        );
        let median = find(&records, "avsync.median_delta");
        assert!((scalar(median) + 0.1).abs() < 1e-9);
    }

    #[test]
    fn drift_slope_recovers_clock_skew() {
        // Video runs 1% late: event times = onset * 1.01. A 1 kHz
        // timebase keeps tick quantization far below the drift signal
        // (a 10 fps grid would swallow it entirely).
        let onsets: Vec<f64> = (0..8).map(|i| 0.4 + i as f64 * 0.4).collect();
        let ticks: Vec<(i64, VideoEventKind)> = onsets
            .iter()
            .map(|t| {
                (
                    (t * 1.01 * 1000.0).round() as i64,
                    VideoEventKind::MotionOnset,
                )
            })
            .collect();
        let v = features_at(&ticks, 0, 1000);
        let records = sync_records(
            "IN-1",
            Some(&onsets),
            Some(&VideoFeatureArtifact::Decoded(v)),
            0.0,
            &AvSyncConfig::default(),
            vec![],
        );
        let slope = find(&records, "avsync.drift_slope");
        assert!(
            (scalar(slope) - 0.01).abs() < 0.002,
            "slope={:?}",
            slope.value
        );
    }

    #[test]
    fn window_excludes_far_events() {
        let v = features(&[(20, VideoEventKind::SceneChange)], 0); // 2.0 s
        let onsets = [0.6];
        let records = sync_records(
            "IN-1",
            Some(&onsets),
            Some(&VideoFeatureArtifact::Decoded(v)),
            0.0,
            &AvSyncConfig::default(),
            vec![],
        );
        assert_eq!(scalar(find(&records, "avsync.pair_count")), 0.0);
        assert_eq!(
            find(&records, "avsync.median_delta").validity,
            Validity::InsufficientSupport
        );
        assert_eq!(scalar(find(&records, "avsync.unmatched_onsets")), 1.0);
    }

    #[test]
    fn missing_or_undecodable_video_is_explicit() {
        // No vids stream at all → NotApplicable.
        let records = sync_records(
            "IN-1",
            Some(&[0.5]),
            None,
            0.0,
            &AvSyncConfig::default(),
            vec![],
        );
        assert_eq!(
            find(&records, "avsync.median_delta").validity,
            Validity::NotApplicable
        );
        // Undecodable vids → DecodeInputUnavailable with the reason.
        let records = sync_records(
            "IN-1",
            Some(&[0.5]),
            Some(&VideoFeatureArtifact::Unavailable {
                stream: 0,
                codec: "MJPG".into(),
                reason: "compressed video codec MJPG is not decodable".into(),
            }),
            0.0,
            &AvSyncConfig::default(),
            vec![],
        );
        let r = find(&records, "avsync.median_delta");
        assert_eq!(r.validity, Validity::DecodeInputUnavailable);
        assert!(r.invalid_reason.as_deref().unwrap().contains("MJPG"));
        // And missing audio evidence dominates over everything.
        let records = sync_records(
            "IN-1",
            None,
            Some(&VideoFeatureArtifact::Decoded(features(&[], 0))),
            0.0,
            &AvSyncConfig::default(),
            vec![],
        );
        assert_eq!(
            find(&records, "avsync.median_delta").validity,
            Validity::DecodeInputUnavailable
        );
    }

    #[test]
    fn unavailable_clock_origin_does_not_produce_valid_sync() {
        for (origin, hz) in [(f64::NAN, 10), (0.0, 0)] {
            let v = features_at(&[(5, VideoEventKind::SceneChange)], 0, hz);
            let records = sync_records(
                "IN-1",
                Some(&[0.5]),
                Some(&VideoFeatureArtifact::Decoded(v)),
                origin,
                &AvSyncConfig::default(),
                vec![],
            );
            assert_eq!(
                find(&records, "avsync.median_delta").validity,
                Validity::ComputationFailed
            );
            assert!(!records.iter().any(|r| r.feature_id == "avsync.delta"));
        }
    }
}
