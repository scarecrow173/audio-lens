//! Candidate event model, threshold extraction, and `events`-role
//! persistence.
//!
//! Events are measurement highlights — never confirmed bugs
//! (measurement-over-verdict ADR). They carry `strength`, propagate
//! `confidence`/`validity` from their evidence records, and link back
//! to the feature rows (or tensor regions) that produced them. Event
//! IDs are content-derived so regeneration is idempotent; the
//! `ExpensiveAnalysisStage` owns writing them to the `events` role.

use al_core::{Confidence, Validity, sha256_hex};
use al_dataset::table::{Cell, Column, DataType};
use al_dataset::types::FeatureRecord;
use serde::{Deserialize, Serialize};

use crate::records::propagate_confidence;

/// Kinds of measurement highlights the pipeline can surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Sample-level discontinuity (click/pop candidate).
    DiscontinuityCandidate,
    /// Sustained digital clipping.
    Clipping,
    /// Alignment lag step.
    LagJump,
    /// Unexplained energy remaining after staged correction.
    LargeResidual,
    /// Low band correlation/coherence between source and capture.
    LowCoherence,
    /// Stuck or repeated buffer content.
    RepeatedBlockCandidate,
    /// Suspected codec/DSP high-frequency cutoff.
    CutoffCandidate,
    /// Silence or dropout region.
    SilenceDropoutCandidate,
    /// Transfer/gain change over time.
    TransferChangeCandidate,
    /// Noise floor above the expected level (elevated floor, not a
    /// dropout).
    ElevatedNoiseFloor,
}

/// Pointer from an event back to the measurement that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Evidence {
    /// Feature-record rows inside a JSON store.
    Rows {
        /// Dataset-relative artifact path (`<role>/<name>`).
        artifact: String,
        /// Row indices into the record array.
        rows: Vec<u64>,
    },
    /// A tensor region inside a chunked array store.
    Region {
        /// Dataset-relative artifact path (`<role>/<name>`).
        artifact: String,
        /// Per-dimension `[start, end)` coordinates.
        region: serde_json::Value,
    },
}

/// One candidate event: a measurement highlight, not a verdict.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateEvent {
    /// Content-derived stable id (`EVT-<alg>-<input>-<start>-<hash>`).
    pub event_id: String,
    /// Event classification.
    pub kind: EventKind,
    /// Input the event was measured on.
    pub input_id: String,
    /// Start time in seconds.
    pub start_s: f64,
    /// End time in seconds; `None` marks a point event.
    pub end_s: Option<f64>,
    /// Measurement strength (threshold exceedance ratio).
    pub strength: f64,
    /// Propagated confidence (min of evidence confidences).
    pub confidence: Confidence,
    /// Propagated validity.
    pub validity: Validity,
    /// The feature record that triggered the event.
    pub source_feature: String,
    /// Optional frequency scope.
    pub frequency_range: Option<(f64, f64)>,
    /// Channels the event applies to.
    pub channels: Vec<u16>,
    /// Analyzer identity.
    pub algorithm_id: String,
    /// Analyzer version.
    pub algorithm_version: String,
    /// Links back to feature rows / tensor regions.
    pub evidence: Vec<Evidence>,
    /// Extraction parameters in effect.
    pub params: serde_json::Value,
}

/// Config-overlayable extraction thresholds. Defaults are
/// conservative; `config.events.*` overrides them.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct EventConfig {
    /// `dig.clipping_rate` above this flags clipping.
    pub clipping_rate: f64,
    /// `dig.discontinuity_max` above this flags a discontinuity.
    pub discontinuity_max: f64,
    /// `dig.stuck_run_max` (samples) above this flags stuck content.
    pub stuck_run_samples: f64,
    /// `dig.zero_run_max` (samples) above this flags a dropout.
    pub zero_run_samples: f64,
    /// `dig.click_count` above this flags click bursts.
    pub click_count: f64,
    /// `level.integrated_dbfs` below this flags a dropout region.
    pub silence_dbfs: f64,
    /// `ref.lag_step_candidates` above this flags a lag jump.
    pub lag_step_count: f64,
    /// `ref.residual_r3_ratio` above this flags a large residual.
    pub residual_ratio: f64,
    /// `ref.band_correlation_min` below this flags low coherence.
    pub low_correlation: f64,
    /// `ref.similarity_fullband` below this flags low similarity.
    pub low_similarity: f64,
    /// `ref.crosstalk_ratio` above this flags leakage.
    pub crosstalk_ratio: f64,
    /// `noise.floor_dbfs` above this flags an elevated floor.
    pub noise_floor_dbfs: f64,
    /// `mod.lag_drift_samples` absolute value above this flags
    /// a time-scale change.
    pub lag_drift_samples: f64,
}

impl Default for EventConfig {
    fn default() -> Self {
        Self {
            clipping_rate: 0.001,
            discontinuity_max: 0.5,
            stuck_run_samples: 64.0,
            zero_run_samples: 2_400.0,
            click_count: 4.0,
            silence_dbfs: -60.0,
            lag_step_count: 1.0,
            residual_ratio: 0.3,
            low_correlation: 0.3,
            low_similarity: 0.5,
            crosstalk_ratio: 0.1,
            noise_floor_dbfs: -40.0,
            lag_drift_samples: 8.0,
        }
    }
}

impl EventConfig {
    /// Overlay `config.analysis.events.*` onto the defaults.
    pub fn from_job(config: &serde_json::Value) -> Self {
        let mut c = Self::default();
        let Some(e) = config.get("analysis").and_then(|a| a.get("events")) else {
            return c;
        };
        macro_rules! ov {
            ($field:ident) => {
                if let Some(v) = e.get(stringify!($field)).and_then(|v| v.as_f64()) {
                    c.$field = v;
                }
            };
        }
        ov!(clipping_rate);
        ov!(discontinuity_max);
        ov!(stuck_run_samples);
        ov!(zero_run_samples);
        ov!(click_count);
        ov!(silence_dbfs);
        ov!(lag_step_count);
        ov!(residual_ratio);
        ov!(low_correlation);
        ov!(low_similarity);
        ov!(crosstalk_ratio);
        ov!(noise_floor_dbfs);
        ov!(lag_drift_samples);
        c
    }
}

/// Scalar record accessor.
fn scalar_of(r: &FeatureRecord) -> Option<f64> {
    match &r.value {
        al_dataset::types::Value::Scalar(v) => Some(*v),
        al_dataset::types::Value::Tensor { .. } => None,
    }
}

/// One threshold rule: `feature_id` crossing `threshold` (in
/// `direction`) yields `kind` with strength `value/threshold`.
struct Rule {
    feature_id: &'static str,
    kind: EventKind,
    /// `true` = flag when mapped value > threshold; `false` = <
    /// threshold.
    above: bool,
    threshold: fn(&EventConfig) -> f64,
    /// Map the raw measured value before comparison (e.g. `abs` for
    /// bidirectional drift).
    map: fn(f64) -> f64,
    /// Normalize the measured value into a strength. Receives the
    /// mapped value and the threshold; must be positive.
    strength: fn(value: f64, threshold: f64) -> f64,
}

fn ratio_strength(value: f64, threshold: f64) -> f64 {
    if threshold == 0.0 {
        value.abs()
    } else {
        (value / threshold).abs()
    }
}

/// Distance below an (upper) threshold expressed as a ratio —
/// used for "flag when below" rules where 0 is the extreme.
fn deficit_strength(value: f64, threshold: f64) -> f64 {
    if threshold == 0.0 {
        0.0
    } else {
        (threshold - value).abs() / threshold.abs()
    }
}

const RULES: &[Rule] = &[
    Rule {
        feature_id: "dig.clipping_rate",
        kind: EventKind::Clipping,
        above: true,
        threshold: |c| c.clipping_rate,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "dig.discontinuity_max",
        kind: EventKind::DiscontinuityCandidate,
        above: true,
        threshold: |c| c.discontinuity_max,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "dig.stuck_run_max",
        kind: EventKind::RepeatedBlockCandidate,
        above: true,
        threshold: |c| c.stuck_run_samples,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "dig.zero_run_max",
        kind: EventKind::SilenceDropoutCandidate,
        above: true,
        threshold: |c| c.zero_run_samples,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "dig.click_count",
        kind: EventKind::DiscontinuityCandidate,
        above: true,
        threshold: |c| c.click_count,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "level.integrated_dbfs",
        kind: EventKind::SilenceDropoutCandidate,
        above: false,
        threshold: |c| c.silence_dbfs,
        map: |v| v,
        strength: deficit_strength,
    },
    Rule {
        feature_id: "ref.lag_step_candidates",
        kind: EventKind::LagJump,
        above: true,
        threshold: |c| c.lag_step_count,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "ref.residual_r3_ratio",
        kind: EventKind::LargeResidual,
        above: true,
        threshold: |c| c.residual_ratio,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "ref.band_correlation_min",
        kind: EventKind::LowCoherence,
        above: false,
        threshold: |c| c.low_correlation,
        map: |v| v,
        strength: deficit_strength,
    },
    Rule {
        feature_id: "ref.similarity_fullband",
        kind: EventKind::LowCoherence,
        above: false,
        threshold: |c| c.low_similarity,
        map: |v| v,
        strength: deficit_strength,
    },
    Rule {
        feature_id: "ref.crosstalk_ratio",
        kind: EventKind::LargeResidual,
        above: true,
        threshold: |c| c.crosstalk_ratio,
        map: |v| v,
        strength: ratio_strength,
    },
    Rule {
        feature_id: "noise.floor_dbfs",
        kind: EventKind::ElevatedNoiseFloor,
        above: true,
        threshold: |c| c.noise_floor_dbfs,
        map: |v| v,
        strength: |v, t| ratio_strength(v - t, 60.0),
    },
    Rule {
        feature_id: "mod.lag_drift_samples",
        kind: EventKind::TransferChangeCandidate,
        above: true,
        threshold: |c| c.lag_drift_samples,
        map: |v| v.abs(),
        strength: ratio_strength,
    },
];

/// Content-derived, stable event id. Hash inputs: kind, input,
/// times, channel scope, source artifact and feature — never
/// wall-clock/random. Two records that differ in any of these
/// fields must produce different ids.
#[allow(clippy::too_many_arguments)]
fn event_id(
    algorithm: &str,
    artifact: &str,
    input_id: &str,
    kind: EventKind,
    start_s: f64,
    end_s: Option<f64>,
    source_feature: &str,
    channels: &[u16],
) -> String {
    let canonical = serde_json::json!([
        format!("{kind:?}"),
        artifact,
        input_id,
        start_s,
        end_s,
        source_feature,
        channels,
    ]);
    let hash = &sha256_hex(canonical.to_string().as_bytes())[..12];
    let start_ms = (start_s * 1_000.0).round() as i64;
    format!("EVT-{algorithm}-{input_id}-{start_ms}ms-{hash}")
}

/// Extract candidate events from one artifact's feature records.
/// `artifact` is the dataset-relative path that produced `records`
/// (e.g. `features/cheap/<input>`); evidence links point back to it
/// by row index. Records that are not `Valid` still produce events —
/// the event carries their propagated validity so downstream never
/// upgrades a weak observation into a strong claim.
///
/// `segment_s` is the measured segment's duration when known:
/// records without a `time_range` are segment-scoped, so their
/// events span the whole segment rather than collapsing to a point
/// at t=0. `None` marks them as point events.
pub fn extract_candidates(
    artifact: &str,
    input_id: &str,
    records: &[FeatureRecord],
    cfg: &EventConfig,
    segment_s: Option<f64>,
) -> Vec<CandidateEvent> {
    let mut events = Vec::new();
    for (row, r) in records.iter().enumerate() {
        let Some(rule) = RULES.iter().find(|rule| rule.feature_id == r.feature_id) else {
            continue;
        };
        let Some(value) = scalar_of(r) else {
            continue;
        };
        let threshold = (rule.threshold)(cfg);
        let mapped = (rule.map)(value);
        let hit = if rule.above {
            mapped > threshold
        } else {
            mapped < threshold
        };
        if !hit {
            continue;
        }
        let (start_s, end_s, segment_scope) = r
            .time_range
            .map(|t| (t.start, Some(t.end), false))
            .unwrap_or((0.0, segment_s, segment_s.is_some()));
        let freq = r.freq_range.map(|f| (f.start, f.end));
        let channels = r
            .channels
            .map(|c| match c {
                al_dataset::types::ChannelSelector::Channel(i) => vec![i],
                al_dataset::types::ChannelSelector::ChannelPair(a, b) => vec![a, b],
            })
            .unwrap_or_default();
        let strength = {
            let s = (rule.strength)(mapped, threshold);
            if s.is_finite() { s.max(0.0) } else { 0.0 }
        };
        events.push(CandidateEvent {
            event_id: event_id(
                "events.v1",
                artifact,
                input_id,
                rule.kind,
                start_s,
                end_s,
                &r.feature_id,
                &channels,
            ),
            kind: rule.kind,
            input_id: input_id.to_string(),
            start_s,
            end_s,
            strength,
            confidence: propagate_confidence(Confidence::High, r.confidence),
            validity: r.validity,
            source_feature: r.feature_id.clone(),
            frequency_range: freq,
            channels,
            algorithm_id: "events.v1".into(),
            algorithm_version: "1".into(),
            evidence: vec![Evidence::Rows {
                artifact: artifact.to_string(),
                rows: vec![row as u64],
            }],
            params: serde_json::json!({
                "threshold": threshold,
                "measured": value,
                "segment_scope": segment_scope,
            }),
        });
    }
    events
}

/// Column schema of the `events`-role table (FeatureRecord-compatible
/// core plus the event extension fields).
pub fn events_schema() -> Vec<Column> {
    [
        ("event_id", DataType::Utf8),
        ("kind", DataType::Utf8),
        ("input_id", DataType::Utf8),
        ("start_s", DataType::F64),
        ("end_s", DataType::F64),
        ("strength", DataType::F64),
        ("confidence", DataType::Utf8),
        ("validity", DataType::Utf8),
        ("source_feature", DataType::Utf8),
        ("freq_lo_hz", DataType::F64),
        ("freq_hi_hz", DataType::F64),
        ("channels", DataType::Utf8),
        ("algorithm_id", DataType::Utf8),
        ("algorithm_version", DataType::Utf8),
        ("evidence", DataType::Utf8),
        ("params", DataType::Utf8),
    ]
    .iter()
    .map(|(name, t)| Column {
        name: (*name).to_string(),
        data_type: *t,
    })
    .collect()
}

/// Serialize an enum to its snake_case tag.
fn tag<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// Flatten events into table rows matching [`events_schema`].
pub fn events_to_rows(events: &[CandidateEvent]) -> Vec<Vec<Cell>> {
    events
        .iter()
        .map(|e| {
            vec![
                Cell::Str(e.event_id.clone()),
                Cell::Str(tag(&e.kind)),
                Cell::Str(e.input_id.clone()),
                Cell::F64(e.start_s),
                e.end_s.map(Cell::F64).unwrap_or(Cell::Null),
                Cell::F64(e.strength),
                Cell::Str(tag(&e.confidence)),
                Cell::Str(tag(&e.validity)),
                Cell::Str(e.source_feature.clone()),
                e.frequency_range
                    .map(|f| Cell::F64(f.0))
                    .unwrap_or(Cell::Null),
                e.frequency_range
                    .map(|f| Cell::F64(f.1))
                    .unwrap_or(Cell::Null),
                Cell::Str(serde_json::to_string(&e.channels).unwrap_or_default()),
                Cell::Str(e.algorithm_id.clone()),
                Cell::Str(e.algorithm_version.clone()),
                Cell::Str(serde_json::to_string(&e.evidence).unwrap_or_default()),
                Cell::Str(e.params.to_string()),
            ]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_dataset::types::ResolutionClass;

    fn record(id: &str, value: f64, validity: Validity) -> FeatureRecord {
        crate::records::RecordBuilder::new("in1", "cheap.v1", ResolutionClass::Segment).scalar(
            id,
            value,
            "ratio",
            None,
            Confidence::High,
            validity,
            None,
            1,
        )
    }

    #[test]
    fn event_ids_are_deterministic() {
        let a = event_id(
            "events.v1",
            "features/cheap/in1",
            "in1",
            EventKind::Clipping,
            1.25,
            Some(2.0),
            "f",
            &[0],
        );
        let b = event_id(
            "events.v1",
            "features/cheap/in1",
            "in1",
            EventKind::Clipping,
            1.25,
            Some(2.0),
            "f",
            &[0],
        );
        let c = event_id(
            "events.v1",
            "features/cheap/in1",
            "in1",
            EventKind::Clipping,
            1.26,
            Some(2.0),
            "f",
            &[0],
        );
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert!(a.starts_with("EVT-events.v1-in1-1250ms-"));
    }

    #[test]
    fn event_ids_differ_by_channel_and_artifact() {
        let base = |artifact: &str, channels: &[u16]| {
            event_id(
                "events.v1",
                artifact,
                "in1",
                EventKind::RepeatedBlockCandidate,
                0.0,
                Some(4.0),
                "dig.stuck_run_max",
                channels,
            )
        };
        let ch0 = base("features/cheap/in1", &[0]);
        let ch1 = base("features/cheap/in1", &[1]);
        let ref_src = base("features/reference/src__in1", &[0]);
        assert_ne!(ch0, ch1);
        assert_ne!(ch0, ref_src);
    }

    #[test]
    fn threshold_crossing_produces_linked_event() {
        let records = vec![
            record("dig.clipping_rate", 0.002, Validity::Valid),
            record("level.rms_dbfs", -20.0, Validity::Valid),
        ];
        let events = extract_candidates(
            "features/cheap/in1",
            "in1",
            &records,
            &EventConfig::default(),
            Some(8.0),
        );
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.kind, EventKind::Clipping);
        assert_eq!(e.strength, 2.0);
        assert_eq!(
            e.evidence,
            vec![Evidence::Rows {
                artifact: "features/cheap/in1".into(),
                rows: vec![0],
            }]
        );
        assert_eq!(e.confidence, Confidence::High);
        assert_eq!(e.validity, Validity::Valid);
    }

    #[test]
    fn invalid_record_propagates_validity() {
        let records = vec![record(
            "dig.discontinuity_max",
            0.9,
            Validity::InsufficientSupport,
        )];
        let events = extract_candidates(
            "features/cheap/in1",
            "in1",
            &records,
            &EventConfig::default(),
            Some(8.0),
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].validity, Validity::InsufficientSupport);
    }

    #[test]
    fn segment_scoped_event_covers_whole_segment() {
        // A segment-level record (no time_range) must produce an
        // event spanning the whole segment, not a point at t=0 —
        // otherwise deep analysis would only look at [0, window].
        let records = vec![record("dig.clipping_rate", 0.01, Validity::Valid)];
        let events = extract_candidates(
            "features/cheap/in1",
            "in1",
            &records,
            &EventConfig::default(),
            Some(30.0),
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].start_s, 0.0);
        assert_eq!(events[0].end_s, Some(30.0));
        assert_eq!(events[0].params["segment_scope"], true);
    }

    #[test]
    fn below_threshold_produces_nothing() {
        let records = vec![
            record("dig.clipping_rate", 0.0005, Validity::Valid),
            record("ref.residual_r3_ratio", 0.1, Validity::Valid),
        ];
        let events = extract_candidates(
            "features/cheap/in1",
            "in1",
            &records,
            &EventConfig::default(),
            Some(8.0),
        );
        assert!(events.is_empty());
    }

    #[test]
    fn events_flatten_to_table_rows() {
        let records = vec![record("ref.lag_step_candidates", 3.0, Validity::Valid)];
        let events = extract_candidates(
            "features/ref/in1",
            "in1",
            &records,
            &EventConfig::default(),
            Some(8.0),
        );
        let schema = events_schema();
        let rows = events_to_rows(&events);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].len(), schema.len());
        assert_eq!(rows[0][1], Cell::Str("lag_jump".into()));
        assert_eq!(rows[0][6], Cell::Str("high".into()));
    }
}
