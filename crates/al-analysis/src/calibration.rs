//! 共通ソースの伝達振幅からキャプチャ間の経路差を観測する。
use al_core::{Confidence, Validity};
use al_dataset::types::{CaptureComparison, ResolutionClass, Value};

use crate::cheap::NamedTensor;
use crate::records::RecordBuilder;
use crate::reference::matrix_to_tensor;

pub(crate) const MIN_COHERENCE: f64 = 0.5;

/// accepted locate と、その alignment が生成した H(f)。
#[derive(Debug)]
pub(crate) struct TransferEvidence {
    pub source: String,
    pub capture: String,
    pub rate: u32,
    pub h: Option<Vec<(f64, f64)>>,
    pub confidence: Confidence,
    pub lineage: Vec<String>,
}

/// reference → target の振幅比。位相は locate の時刻基準に依存するため
/// 集約しない。同一周波数 grid の正周波数 bin のみ比較する。
pub(crate) fn compare(
    reference: &str,
    target: &str,
    evidence: &[TransferEvidence],
) -> (CaptureComparison, Option<NamedTensor>) {
    let mut common: Vec<_> = evidence
        .iter()
        .filter(|a| a.capture == reference)
        .filter_map(|a| {
            evidence
                .iter()
                .find(|b| b.capture == target && b.source == a.source)
                .map(|b| (a, b))
        })
        .collect();
    common.sort_by(|a, b| a.0.source.cmp(&b.0.source));
    common.dedup_by(|a, b| a.0.source == b.0.source);
    let sources = common.iter().map(|(a, _)| a.source.clone()).collect();
    let lineage: Vec<_> = common
        .iter()
        .flat_map(|(a, b)| a.lineage.iter().chain(&b.lineage).cloned())
        .collect();
    let usable: Vec<_> = common
        .iter()
        .filter_map(|(a, b)| {
            let (ha, hb) = (a.h.as_ref()?, b.h.as_ref()?);
            (a.rate > 0 && a.rate == b.rate && ha.len() == hb.len() && ha.len() >= 2)
                .then_some((*a, *b, ha, hb))
        })
        .collect();
    // 異なる grid を暗黙に補間しない。全ソースの grid が一致する場合のみ集約。
    let grid = usable.first().map(|(a, _, h, _)| (a.rate, h.len()));
    let compatible = grid.is_some()
        && usable
            .iter()
            .all(|(a, _, h, _)| Some((a.rate, h.len())) == grid);
    let name = format!("{reference}__{target}");
    let path = format!("intermediate/cross_capture/{name}/path_gain");
    let mut tensor = None;
    let mut supported_bins = 0;
    let mut confidence = Confidence::Low;
    if compatible {
        let (_, fft_size) = grid.unwrap();
        let bins = fft_size / 2 + 1;
        let mut ratio = vec![f64::NAN; bins];
        let mut db = vec![f64::NAN; bins];
        let mut support = vec![0.0; bins];
        confidence = Confidence::Medium;
        for (a, b, _, _) in &usable {
            confidence = confidence.min(a.confidence).min(b.confidence);
        }
        for k in 0..bins {
            let mut values: Vec<_> = usable
                .iter()
                .filter_map(|(_, _, ha, hb)| {
                    let x = ha[k].0.hypot(ha[k].1);
                    let y = hb[k].0.hypot(hb[k].1);
                    let r = y / x;
                    (x > 0.0
                        && y > 0.0
                        && x.is_finite()
                        && y.is_finite()
                        && r.is_finite()
                        && r > 0.0)
                        .then_some(r)
                })
                .collect();
            values.sort_by(f64::total_cmp);
            let n = values.len();
            support[k] = n as f64;
            if n > 0 {
                ratio[k] = if n % 2 == 1 {
                    values[n / 2]
                } else {
                    values[n / 2 - 1] / 2.0 + values[n / 2] / 2.0
                };
                db[k] = 20.0 * ratio[k].log10();
                supported_bins += 1;
            }
        }
        tensor = Some(matrix_to_tensor(path.clone(), &[ratio, db, support]));
    }
    let (validity, reason) = if common.is_empty() {
        (
            Validity::Unsupported,
            Some("no common accepted source".into()),
        )
    } else if !compatible || supported_bins == 0 {
        (
            Validity::InsufficientSupport,
            Some("transfer evidence missing, incompatible grid, or no finite positive bin".into()),
        )
    } else {
        (Validity::Valid, None)
    };
    if validity != Validity::Valid {
        confidence = Confidence::Low;
    }
    let mut record = RecordBuilder::new(
        "job",
        "cross_capture.path_gain.v1",
        ResolutionClass::Segment,
    )
    .lineage(lineage)
    .scalar(
        "cross_capture.path_gain",
        f64::NAN,
        "ratio",
        None,
        confidence,
        validity,
        reason,
        supported_bins,
    );
    record.target = None;
    record.params = serde_json::json!({
        "aggregation": "per_bin_median_magnitude_ratio", "rows": ["magnitude_ratio", "gain_difference_db", "source_support"],
        "sample_rate": grid.map(|g| g.0), "fft_size": grid.map(|g| g.1),
        "reference_capture_id": reference, "target_capture_id": target,
        "usable_source_count": usable.len(), "bin_frequency": "k * sample_rate / fft_size",
        "minimum_magnitude_squared_coherence": MIN_COHERENCE,
        "row_units": ["ratio", "db", "count"],
    });
    if let Some(t) = &tensor {
        record.value = Value::Tensor {
            array: path,
            region: vec![[0, 3], [0, t.shape[1]]],
        };
    }
    (
        CaptureComparison {
            reference_capture_id: reference.into(),
            target_capture_id: target.into(),
            common_source_ids: sources,
            records: vec![record],
        },
        tensor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn evidence(source: &str, capture: &str, gain: f64) -> TransferEvidence {
        TransferEvidence {
            source: source.into(),
            capture: capture.into(),
            rate: 48000,
            h: Some(vec![(gain, 0.0); 4]),
            confidence: Confidence::High,
            lineage: vec![format!("{source}__{capture}")],
        }
    }
    #[test]
    fn ratios_use_all_common_sources_and_median_rejects_outlier() {
        let e = vec![
            evidence("s1", "ref", 2.0),
            evidence("s1", "bug", 4.0),
            evidence("s2", "ref", 3.0),
            evidence("s2", "bug", 6.0),
            evidence("s3", "ref", 1.0),
            evidence("s3", "bug", 100.0),
        ];
        let (c, t) = compare("ref", "bug", &e);
        assert_eq!(c.common_source_ids, ["s1", "s2", "s3"]);
        assert_eq!(c.records[0].validity, Validity::Valid);
        let t = t.unwrap();
        assert_eq!(&t.data[..3], &[2.0; 3]);
        assert!((t.data[3] - 6.020599913).abs() < 1e-8);
        assert_eq!(&t.data[6..], &[3.0; 3]);
        let (_, inverse) = compare("bug", "ref", &e);
        assert_eq!(&inverse.unwrap().data[..3], &[0.5; 3]);
    }
    #[test]
    fn no_common_source_is_unsupported() {
        let (c, t) = compare(
            "a",
            "b",
            &[evidence("s1", "a", 1.0), evidence("s2", "b", 1.0)],
        );
        assert_eq!(c.records[0].validity, Validity::Unsupported);
        assert!(t.is_none());
    }
    #[test]
    fn missing_or_incompatible_transfer_is_not_a_valid_zero() {
        let mut e = vec![evidence("s", "a", 1.0), evidence("s", "b", 2.0)];
        e[1].rate = 44100;
        assert_eq!(
            compare("a", "b", &e).0.records[0].validity,
            Validity::InsufficientSupport
        );
        e[1].rate = 48000;
        e[1].h = None;
        assert_eq!(
            compare("a", "b", &e).0.records[0].validity,
            Validity::InsufficientSupport
        );
        e[1].h = Some(vec![(0.0, 0.0); 4]);
        let (c, t) = compare("a", "b", &e);
        assert_eq!(c.records[0].support_count, 0);
        assert!(t.unwrap().data[0].is_nan());
    }
    #[test]
    fn even_median_and_per_bin_support() {
        let mut e = vec![
            evidence("s1", "a", 1.0),
            evidence("s1", "b", 2.0),
            evidence("s2", "a", 1.0),
            evidence("s2", "b", 4.0),
        ];
        e[0].h.as_mut().unwrap()[0] = (f64::NAN, 0.0);
        let (_, t) = compare("a", "b", &e);
        let t = t.unwrap();
        assert_eq!(&t.data[..3], &[4.0, 3.0, 3.0]);
        assert_eq!(&t.data[6..], &[1.0, 2.0, 2.0]);
    }
}
