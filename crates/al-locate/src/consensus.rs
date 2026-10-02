//! Consensus acceptance across independent locate methods.
//!
//! A position is accepted only when ≥ `min_agreeing_methods`
//! independent methods place a candidate within `position_tolerance`
//! samples of each other, with usable scores. Everything — accepted
//! or rejected — is persisted in a [`LocateResult`].

use al_core::{AlignmentId, Confidence, Validity};

use crate::candidate::MethodEvidence;
use crate::result::{LocateResult, SampleRange};

/// Consensus thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Minimum number of methods that must agree on one position.
    #[serde(default = "default_min_agreeing")]
    pub min_agreeing_methods: u32,
    /// Maximum |position_i − position_j| to count as agreement
    /// (samples).
    #[serde(default = "default_position_tolerance")]
    pub position_tolerance: u64,
    /// Minimum peak score magnitude for a candidate to count as
    /// agreement.
    #[serde(default = "default_min_score")]
    pub min_score: f64,
    /// Minimum peak-to-background ratio for a candidate to count.
    #[serde(default = "default_min_p2b")]
    pub min_peak_to_background: f64,
}

use serde::{Deserialize, Serialize};

fn default_min_agreeing() -> u32 {
    2
}
fn default_position_tolerance() -> u64 {
    256
}
fn default_min_score() -> f64 {
    0.3
}
fn default_min_p2b() -> f64 {
    2.0
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            min_agreeing_methods: 2,
            position_tolerance: 256,
            min_score: 0.3,
            min_peak_to_background: 2.0,
        }
    }
}

/// Evaluate `evidence` into a [`LocateResult`].
///
/// `source_len`/`capture_len` bound the reported sample ranges.
/// `algorithm_version` is recorded verbatim — versions live with the
/// result, not the schema.
#[allow(clippy::too_many_arguments)]
pub fn evaluate(
    evidence: &[MethodEvidence],
    source_input_id: &str,
    capture_input_id: &str,
    source_len: u64,
    capture_len: u64,
    config: &ConsensusConfig,
    algorithm_id: &str,
    algorithm_version: &str,
) -> LocateResult {
    // Usable top candidates: one vote per method, and its top peak
    // clears the score/background floors. Scores are signed (negative
    // = polarity inversion); agreement is judged on magnitude.
    let mut seen_methods: Vec<&str> = Vec::new();
    let votes: Vec<u64> = evidence
        .iter()
        .filter(|e| e.validity != Validity::ComputationFailed)
        .filter_map(|e| {
            if seen_methods.contains(&e.method.as_str()) {
                return None;
            }
            seen_methods.push(e.method.as_str());
            e.candidates.first()
        })
        .filter(|c| {
            c.score.abs() >= config.min_score
                && c.peak_to_background >= config.min_peak_to_background
        })
        .map(|c| c.position)
        .collect();

    let result = |accepted: bool,
                  agreeing: u32,
                  spread: u64,
                  position: Option<u64>,
                  confidence: Confidence,
                  validity: Validity,
                  reason: Option<String>| LocateResult {
        alignment_id: AlignmentId::new(),
        source_input_id: source_input_id.into(),
        capture_input_id: capture_input_id.into(),
        source_range: SampleRange {
            start: 0,
            end: source_len,
        },
        capture_range: position
            .map(|p| {
                let start = p.min(capture_len);
                SampleRange {
                    start,
                    end: (p + source_len).min(capture_len).max(start),
                }
            })
            .unwrap_or(SampleRange { start: 0, end: 0 }),
        evidence: evidence.to_vec(),
        agreeing_methods: agreeing,
        position_spread: spread,
        confidence,
        validity,
        reject_reason: reason,
        accepted,
        algorithm_id: algorithm_id.into(),
        algorithm_version: algorithm_version.into(),
        params: serde_json::to_value(config).unwrap_or_default(),
    };

    if votes.is_empty() {
        let any_valid = evidence.iter().any(|e| e.validity == Validity::Valid);
        return result(
            false,
            0,
            0,
            None,
            Confidence::Low,
            if any_valid {
                Validity::LowCorrelation
            } else {
                Validity::InsufficientSupport
            },
            Some("no method produced a candidate clearing the thresholds".into()),
        );
    }

    // Cluster votes within tolerance. A cluster's diameter (not just
    // neighbour distance) is bounded by position_tolerance, so every
    // member pair is within tolerance — no transitive chaining.
    // Take the largest cluster; a tie or a second ≥min cluster at a
    // distant position is ambiguity, not a silent pick.
    let mut sorted = votes.clone();
    sorted.sort_unstable();
    let mut clusters: Vec<Vec<u64>> = Vec::new();
    for &v in &sorted {
        match clusters.last_mut() {
            Some(c) if v - c[0] <= config.position_tolerance => c.push(v),
            _ => clusters.push(vec![v]),
        }
    }
    clusters.sort_by_key(|c| std::cmp::Reverse(c.len()));
    let best = &clusters[0];
    let agreeing = best.len() as u32;
    let spread = best[best.len() - 1] - best[0];
    let position = best[(best.len() - 1) / 2];

    let contested = clusters.len() > 1 && clusters[1].len() as u32 >= config.min_agreeing_methods;
    if agreeing < config.min_agreeing_methods {
        return result(
            false,
            agreeing,
            spread,
            Some(position),
            Confidence::Low,
            Validity::AmbiguousAlignment,
            Some(format!(
                "only {agreeing} method(s) agree (need {})",
                config.min_agreeing_methods
            )),
        );
    }
    if contested {
        return result(
            false,
            agreeing,
            spread,
            Some(position),
            Confidence::Low,
            Validity::AmbiguousAlignment,
            Some("multiple positions each have enough agreement".into()),
        );
    }
    let confidence = if agreeing >= 3 {
        Confidence::High
    } else {
        Confidence::Medium
    };
    result(
        true,
        agreeing,
        spread,
        Some(position),
        confidence,
        Validity::Valid,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::LocateCandidate;

    fn ev(method: &str, pos: u64, score: f64) -> MethodEvidence {
        MethodEvidence {
            method: method.into(),
            method_version: "1".into(),
            candidates: vec![LocateCandidate {
                position: pos,
                score,
                peak_to_background: 5.0,
                peak_width: 4,
                peak_ratio_12: Some(2.0),
                channel_pair: None,
            }],
            validity: Validity::Valid,
            reject_reason: None,
        }
    }

    fn cfg() -> ConsensusConfig {
        ConsensusConfig {
            position_tolerance: 100,
            ..Default::default()
        }
    }

    #[test]
    fn accepts_when_two_methods_agree() {
        let e = vec![
            ev("ncc", 1000, 0.9),
            ev("phat", 1050, 0.8),
            ev("env", 9999, 0.7),
        ];
        let r = evaluate(&e, "IN-s", "IN-c", 500, 48000, &cfg(), "locate", "1");
        assert!(r.accepted);
        assert_eq!(r.agreeing_methods, 2);
        assert_eq!(r.capture_range.start, 1000);
        assert_eq!(r.position_spread, 50);
        assert!(!r.alignment_id.as_str().is_empty());
    }

    #[test]
    fn rejects_single_method_peak() {
        let e = vec![
            ev("ncc", 1000, 0.95),
            ev("phat", 9000, 0.8),
            ev("env", 4000, 0.7),
        ];
        let r = evaluate(&e, "IN-s", "IN-c", 500, 48000, &cfg(), "locate", "1");
        assert!(!r.accepted);
        assert_eq!(r.validity, Validity::AmbiguousAlignment);
        // Evidence preserved even on rejection.
        assert_eq!(r.evidence.len(), 3);
    }

    #[test]
    fn rejects_when_no_votes() {
        let mut e = ev("ncc", 100, 0.9);
        e.candidates.clear();
        let r = evaluate(&[e], "IN-s", "IN-c", 500, 48000, &cfg(), "locate", "1");
        assert!(!r.accepted);
    }

    #[test]
    fn contested_clusters_are_ambiguous() {
        let e = vec![
            ev("ncc", 1000, 0.9),
            ev("phat", 1010, 0.8),
            ev("env", 9000, 0.85),
            ev("band", 9020, 0.75),
        ];
        let r = evaluate(&e, "IN-s", "IN-c", 500, 48000, &cfg(), "locate", "1");
        assert!(!r.accepted);
        assert_eq!(r.validity, Validity::AmbiguousAlignment);
    }
}
