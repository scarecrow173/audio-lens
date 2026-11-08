//! Fake 層・GPUI 層で共有する Source fixture(AL-G3)と Job fixture(AL-G4)。
#![allow(dead_code)]

use std::path::Path;
use std::sync::Arc;

use al_core::{JobId, StageKind, StageStatus};
use al_gui::view::source::WaveformView;
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::parse_input_spec;
use al_testkit::media::wav_s16_bytes;
use ruviform_viz::{SliceWaveform, WaveformSource};
use serde_json::json;

/// `ingest → cheap_analysis → dataset` の job 段数。
pub const JOB_STAGES: usize = 3;

/// 1 秒の capture `wav` から `base` 配下に完了済みの job を作る。
pub fn make_job(base: &Path, wav: &Path) -> JobId {
    let samples: Vec<i16> = (0..48_000).map(|i| (i % 400) as i16 - 200).collect();
    std::fs::write(wav, wav_s16_bytes(1, 48_000, &samples)).unwrap();
    let spec = RunSpec {
        inputs: vec![parse_input_spec(&format!("capture_bug:{}", wav.display())).unwrap()],
        stages: Some(vec![
            StageKind::Ingest,
            StageKind::CheapAnalysis,
            StageKind::Dataset,
        ]),
        config: json!({}),
    };
    let (job_id, manifest) = JobService::new(base)
        .run(spec, &RunControl::default())
        .unwrap();
    assert_eq!(manifest.stages.len(), JOB_STAGES);
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed)
    );
    job_id
}

pub const SAMPLE_RATE: u32 = 48_000;
/// 0.1 秒。`[1200, 1440)` frame を NaN gap にする。
pub const FRAMES: usize = 4_800;
pub const GAP: std::ops::Range<usize> = 1_200..1_440;

pub fn fixture_view() -> WaveformView {
    let samples: Vec<f64> = (0..FRAMES)
        .map(|i| {
            if GAP.contains(&i) {
                f64::NAN
            } else {
                (i as f64 * 0.01).sin() * 0.75
            }
        })
        .collect();
    let source: Arc<dyn WaveformSource> =
        Arc::new(SliceWaveform::new(SAMPLE_RATE, 1, samples).expect("slice waveform"));
    WaveformView::try_from_source(source).expect("waveform view")
}
