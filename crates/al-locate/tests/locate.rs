//! LocateStage integration tests over a synthesized job fixture.
//!
//! Each test builds a manifest with `decoded_pcm` artifacts (f64le,
//! scale 1.0 — no `ingest_metadata` needed) and runs the real stage.

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_job::job::input;
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};
use al_locate::stage::LocateStage;

/// Unbiased deterministic noise (LCG → uniform over [-1, 1)).
fn noise(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 11) as f64 / ((1u64 << 53) as f64)) * 2.0 - 1.0
        })
        .collect()
}

fn write_pcm(job_dir: &Path, rel: &str, samples: &[f64]) {
    let path = job_dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let bytes: Vec<u8> = samples.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(&path, bytes).unwrap();
}

struct Fixture {
    dir: PathBuf,
    manifest: Manifest,
    source_input_id: String,
    capture_input_id: String,
}

/// A job with one `source_asset` and one `capture_bug`, each holding a
/// mono 48 kHz `decoded_pcm` artifact under `inputs/`.
fn fixture(source: &[f64], capture: &[f64], sample_rates: (u32, u32)) -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-locate-test-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let src_input = input(InputRole::SourceAsset, dir.join("src.bin"));
    let cap_input = input(InputRole::CaptureBug, dir.join("cap.bin"));
    let source_input_id = src_input.input_id.to_string();
    let capture_input_id = cap_input.input_id.to_string();
    let input_ids = [src_input.input_id.clone(), cap_input.input_id.clone()];
    let mut manifest = Manifest::new(
        JobId::new(),
        vec![src_input, cap_input],
        serde_json::json!({}),
        &[StageKind::Ingest, StageKind::Locate],
    );

    for (idx, (rel, samples, rate)) in [
        ("inputs/src.f64le", source, sample_rates.0),
        ("inputs/cap.f64le", capture, sample_rates.1),
    ]
    .into_iter()
    .enumerate()
    {
        write_pcm(&dir, rel, samples);
        let declared = manifest.input_artifact(&input_ids[idx]).unwrap();
        manifest.register_artifact(ArtifactRecord {
            artifact_id: al_core::ArtifactId::new(),
            role: "decoded_pcm".into(),
            path: PathBuf::from(rel),
            produced_by: Some(StageKind::Ingest),
            inputs: vec![declared],
            params: serde_json::json!({"sample_rate": rate, "channels": 1}),
        });
    }

    Fixture {
        dir,
        manifest,
        source_input_id,
        capture_input_id,
    }
}

/// Run the stage and read the pair's persisted result.json.
fn run_and_read(fx: &Fixture) -> (Vec<ArtifactRecord>, serde_json::Value) {
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &config,
    };
    let records = LocateStage::new().run(&ctx).expect("stage run");
    let result_path = fx
        .dir
        .join("locate")
        .join(&fx.source_input_id)
        .join(&fx.capture_input_id)
        .join("result.json");
    let text = fs::read_to_string(result_path).expect("result.json persisted");
    (records, serde_json::from_str(&text).unwrap())
}

#[test]
fn embedded_source_is_located_with_time_map_and_residual() {
    let source = noise(2_000, 7);
    let mut capture = noise(20_000, 11);
    for (i, &v) in source.iter().enumerate() {
        capture[5_000 + i] += v;
    }
    let fx = fixture(&source, &capture, (48_000, 48_000));
    let (records, body) = run_and_read(&fx);
    let result = &body["result"];

    assert_eq!(result["accepted"], true, "result: {result}");
    assert!((result["capture_range"]["start"].as_u64().unwrap() as i64 - 5_000).abs() <= 64);
    assert!(result["agreeing_methods"].as_u64().unwrap() >= 3);

    // Derived artifacts: mixer fit, time map, residual search.
    assert!(body["derived"]["mixer"]["matrix"].is_array());
    assert!(body["derived"]["time_map"]["segments"].is_array());
    assert!(body["derived"]["residual_search"]["child"].is_object());
    // Channel-pair identity is preserved in method evidence.
    assert!(result["evidence"].as_array().unwrap().iter().all(|e| {
        e["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["channel_pair"] == serde_json::json!([0, 0]))
    }));
    // Residual PCM persisted as its own artifact.
    assert!(
        records
            .iter()
            .any(|r| r.role == "locate_residual" && r.path.ends_with("residual.f64le"))
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn tail_aligned_source_reaches_last_lag() {
    let source = noise(1_500, 3);
    let mut capture = noise(8_000, 5);
    let tail = capture.len() - source.len();
    for (i, &v) in source.iter().enumerate() {
        capture[tail + i] += v;
    }
    let fx = fixture(&source, &capture, (48_000, 48_000));
    let (_records, body) = run_and_read(&fx);
    let result = &body["result"];
    assert_eq!(result["accepted"], true, "result: {result}");
    assert!(
        (result["capture_range"]["start"].as_u64().unwrap() as i64 - tail as i64).abs() <= 64,
        "capture_range: {}",
        result["capture_range"]
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn polarity_inverted_source_is_still_located() {
    let source = noise(1_500, 13);
    let mut capture = noise(8_000, 17);
    for (i, &v) in source.iter().enumerate() {
        capture[3_000 + i] -= v;
    }
    let fx = fixture(&source, &capture, (48_000, 48_000));
    let (_records, body) = run_and_read(&fx);
    let result = &body["result"];
    assert_eq!(result["accepted"], true, "result: {result}");
    assert!(
        (result["capture_range"]["start"].as_u64().unwrap() as i64 - 3_000).abs() <= 64,
        "capture_range: {}",
        result["capture_range"]
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn repeated_source_reports_second_occurrence() {
    let source = noise(1_200, 23);
    let mut capture = noise(12_000, 29);
    for (i, &v) in source.iter().enumerate() {
        capture[2_000 + i] += v;
        capture[9_000 + i] += v;
    }
    let fx = fixture(&source, &capture, (48_000, 48_000));
    let (_records, body) = run_and_read(&fx);
    let result = &body["result"];
    assert_eq!(result["accepted"], true, "result: {result}");
    let occurrences: Vec<u64> = body["derived"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    // Exactly one secondary occurrence near the non-primary embed.
    assert_eq!(occurrences.len(), 1, "occurrences: {occurrences:?}");
    let primary = result["capture_range"]["start"].as_u64().unwrap();
    let other = if (primary as i64 - 2_000).abs() <= 64 {
        9_000
    } else {
        2_000
    };
    assert!((occurrences[0] as i64 - other).abs() <= 64);

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn empty_capture_is_marked_not_panics() {
    let source = noise(1_000, 31);
    let fx = fixture(&source, &[], (48_000, 48_000));
    let (_records, body) = run_and_read(&fx);
    let result = &body["result"];
    assert_eq!(result["accepted"], false);
    assert_eq!(result["validity"], "insufficient_support");
    assert!(result["reject_reason"].as_str().unwrap().contains("empty"));

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn sample_rate_mismatch_is_marked_not_resampled() {
    let source = noise(1_000, 37);
    let mut capture = noise(6_000, 41);
    for (i, &v) in source.iter().enumerate() {
        capture[2_500 + i] += v;
    }
    let fx = fixture(&source, &capture, (44_100, 48_000));
    let (_records, body) = run_and_read(&fx);
    let result = &body["result"];
    assert_eq!(result["accepted"], false);
    assert_eq!(result["validity"], "out_of_supported_frequency_range");
    assert!(
        result["reject_reason"]
            .as_str()
            .unwrap()
            .contains("sample rate mismatch")
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn capture_only_job_completes_with_empty_output() {
    let dir = std::env::temp_dir().join(format!("al-locate-caponly-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();
    let manifest = Manifest::new(
        JobId::new(),
        vec![input(InputRole::CaptureBug, dir.join("cap.bin"))],
        serde_json::json!({}),
        &[StageKind::Ingest, StageKind::Locate],
    );
    let registry = ArtifactRegistry::new(&dir);
    let config = serde_json::json!({});
    let ctx = StageContext {
        manifest: &manifest,
        registry: &registry,
        config: &config,
    };
    let records = LocateStage::new().run(&ctx).unwrap();
    assert!(records.is_empty());

    fs::remove_dir_all(&dir).ok();
}
