//! ingest の ROI 観測を dataset に投影する。デコードやイベント再検出は行わない。
use al_core::{Confidence, Validity};
use al_dataset::types::{ResolutionClass, Value};
use al_media::video::VideoFeatureArtifact;

use crate::cheap::{CheapResult, NamedTensor};
use crate::records::RecordBuilder;

/// ROI の輝度と対応する実測時刻を出力。名前は params に保持しパスは index で作る。
pub fn roi_records(
    input_id: &str,
    video: Option<&VideoFeatureArtifact>,
    lineage: Vec<String>,
) -> CheapResult {
    let mut out = CheapResult::default();
    let Some(VideoFeatureArtifact::Decoded(video)) = video else {
        return out;
    };
    if video.roi_luminance.is_empty() {
        return out;
    }
    let builder = RecordBuilder::new(input_id, "video.roi_luminance.v1", ResolutionClass::Segment)
        .lineage(lineage);
    let times_path = format!("features/cheap/{input_id}/video/frame_times");
    let times: Vec<f64> = video
        .frame_timestamps
        .iter()
        .map(|t| (*t - video.start_tick) as f64 / video.timebase_hz as f64)
        .collect();
    let mut series = vec![(
        "video.frame_times",
        times_path.clone(),
        times,
        serde_json::json!({
            "timebase_hz": video.timebase_hz, "start_tick": video.start_tick,
        }),
        "s",
    )];
    for (i, roi) in video.roi_luminance.iter().enumerate() {
        series.push((
            "video.roi_luminance",
            format!("features/cheap/{input_id}/video/roi_{i}/luminance"),
            roi.luminance.clone(),
            serde_json::json!({
                "roi": roi.roi, "frame_times": times_path,
                "luminance_step_threshold": video.params.luminance_step_threshold,
            }),
            "luma_0_255",
        ));
    }
    for (id, path, data, params, unit) in series {
        let n = data.len() as u64;
        let mut record = builder.scalar(
            id,
            f64::NAN,
            unit,
            None,
            Confidence::High,
            if n == 0 {
                Validity::InsufficientSupport
            } else {
                Validity::Valid
            },
            (n == 0).then(|| "no decoded video frames".into()),
            n,
        );
        record.target = None;
        record.params = params;
        // 空 tensor は保存されないため、参照も作らない。
        if n > 0 {
            record.value = Value::Tensor {
                array: path.clone(),
                region: vec![[0, 1], [0, n]],
            };
            out.tensors.push(NamedTensor {
                path,
                shape: [1, n],
                data,
            });
        }
        out.records.push(record);
    }
    out
}
