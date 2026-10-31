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
fn long_source_persists_coarse_proposals_and_global_evidence() {
    let source = noise(10_241, 901);
    let mut capture = noise(18_003, 902);
    for (out, value) in capture[3001..].iter_mut().zip(&source) {
        *out += value * 3.0;
    }
    let fx = fixture(&source, &capture, (512, 512));
    let (records, result) = run_and_read(&fx);
    assert_eq!(result["result"]["accepted"], true);
    let record = records
        .iter()
        .find(|r| r.role == "locate_evidence")
        .unwrap();
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&record.path)).unwrap()).unwrap();
    let full = body["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["method"] == "full_ncc")
        .unwrap();
    assert_eq!(full["coarse_search"]["coarse_rate"], 128);
    assert_eq!(
        full["coarse_search"]["search_range"],
        serde_json::json!([0, 7762])
    );
    assert_eq!(full["coarse_search"]["full_search_required"], true);
    assert!(
        full["coarse_search"]["refined_positions"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(3001))
    );
    use al_locate::LocateMethod;
    assert_eq!(
        full["candidates"],
        serde_json::to_value(
            al_locate::FullNcc
                .locate(
                    &source,
                    &capture,
                    512,
                    &mut al_locate::pool::CorrelationPool::new()
                )
                .candidates,
        )
        .unwrap()
    );
    fs::remove_dir_all(&fx.dir).ok();
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

    // Derived data: mixer fit and verified occurrences.
    assert!(body["derived"]["mixer"]["matrix"].is_array());
    assert!(body["derived"]["occurrences"].is_array());
    // The piecewise time map is its own artifact.
    let time_map_rec = records
        .iter()
        .find(|r| r.role == "time_map" && r.path.ends_with("time_map.json"))
        .expect("time_map artifact");
    let tm: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&time_map_rec.path)).unwrap())
            .unwrap();
    assert!(tm["time_map"]["segments"].is_array());
    // Per-channel-pair evidence is its own artifact.
    let evidence_rec = records
        .iter()
        .find(|r| r.role == "locate_evidence")
        .expect("locate_evidence artifact");
    let ev: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&evidence_rec.path)).unwrap())
            .unwrap();
    assert!(
        ev["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["source_channel"].is_u64() && p["capture_channel"].is_u64())
    );
    // Residual search ran once per accepted alignment and records
    // the child locate result with subtraction provenance.
    let residual_rec = records
        .iter()
        .find(|r| r.role == "residual" && r.path.extension().is_some_and(|e| e == "json"))
        .expect("residual artifact");
    let res: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&residual_rec.path)).unwrap())
            .unwrap();
    assert!(res["child"].is_object());
    assert!(res["energy"]["before"].is_f64());

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn residual_child_reopens_the_exact_subtracted_signal_and_lineage() {
    use al_locate::{decode::read_alignment_pcm, residual::prepare_residual, result::LocateResult};
    let source = noise(2048, 71);
    let quiet = noise(1024, 72);
    let mut capture = vec![0.0; 8192];
    capture[1024..3072].copy_from_slice(&source);
    for (out, v) in capture[4096..].iter_mut().zip(&quiet) {
        *out += 0.04 * v;
    }
    let mut fx = fixture(&source, &capture, (48000, 48000));
    let mut right = vec![0.0; capture.len()];
    for (out, v) in right[1024..].iter_mut().zip(&source) {
        *out = -0.5 * v;
    }
    for (out, v) in right[4096..].iter_mut().zip(&quiet) {
        *out += 0.06 * v;
    }
    let interleaved: Vec<f64> = capture
        .iter()
        .zip(&right)
        .flat_map(|(&l, &r)| [l, r])
        .collect();
    write_pcm(&fx.dir, "inputs/cap.f64le", &interleaved);
    fx.manifest
        .artifacts
        .iter_mut()
        .find(|r| r.path == Path::new("inputs/cap.f64le"))
        .unwrap()
        .params["channels"] = serde_json::json!(2);
    let child = input(InputRole::SourceAsset, fx.dir.join("quiet.bin"));
    let child_id = child.input_id.to_string();
    // Manifest::new registers the declared input artifacts as well.
    let mut extra = Manifest::new(JobId::new(), vec![child], serde_json::json!({}), &[]);
    let declared = extra.input_artifact(&extra.inputs[0].input_id).unwrap();
    fx.manifest.inputs.append(&mut extra.inputs);
    fx.manifest.artifacts.append(&mut extra.artifacts);
    write_pcm(&fx.dir, "inputs/quiet.f64le", &quiet);
    fx.manifest.register_artifact(ArtifactRecord {
        artifact_id: al_core::ArtifactId::new(),
        role: "decoded_pcm".into(),
        path: "inputs/quiet.f64le".into(),
        produced_by: Some(StageKind::Ingest),
        inputs: vec![declared],
        params: serde_json::json!({"sample_rate": 48000, "channels": 1}),
    });
    let (records, body) = run_and_read(&fx);
    let parent: LocateResult = serde_json::from_value(body["result"].clone()).unwrap();
    assert!(parent.accepted);
    let search_record = records
        .iter()
        .find(|r| {
            r.role == "residual"
                && r.path.extension().is_some_and(|e| e == "json")
                && r.params["parent_source"] == fx.source_input_id
                && r.params["child_source"] == child_id
        })
        .unwrap();
    let search: serde_json::Value =
        serde_json::from_slice(&fs::read(fx.dir.join(&search_record.path)).unwrap()).unwrap();
    let child: LocateResult = serde_json::from_value(search["child"].clone()).unwrap();
    assert!(child.accepted);
    let residual_id = child.capture_pcm.as_ref().unwrap().artifact_id.clone();
    let residual = records
        .iter()
        .find(|r| r.artifact_id == residual_id)
        .unwrap();
    assert_eq!(residual.role, "residual");
    assert_eq!(residual.path.extension().unwrap(), "wav");
    assert_eq!(search["residual_pcm"]["artifact_id"], residual_id.as_str());
    assert!(search_record.inputs.contains(&residual_id));
    let parent_record = records
        .iter()
        .find(|r| {
            r.role == "locate_result" && r.params["alignment_id"] == parent.alignment_id.as_str()
        })
        .unwrap();
    assert!(residual.inputs.contains(&parent_record.artifact_id));
    assert_eq!(
        residual.params["parent_alignment"],
        search["parent_alignment"]
    );
    let matrix: Vec<Vec<f64>> =
        serde_json::from_value(search["channel_transform"].clone()).unwrap();
    let saved_matrix: Vec<Vec<f64>> =
        serde_json::from_value(residual.params["channel_transform"].clone()).unwrap();
    for (saved, json) in saved_matrix.iter().flatten().zip(matrix.iter().flatten()) {
        assert!((saved - json).abs() < 1e-15);
    }
    let mut aligned = vec![0.0; capture.len()];
    let start = parent.capture_range.start as usize;
    let span = (parent.capture_range.end - parent.capture_range.start) as usize;
    aligned[start..start + span].copy_from_slice(&source[..span]);
    let expected = prepare_residual(&[aligned], &[capture.clone(), right], 1e-3).unwrap();
    assert_ne!(expected.channels[0], capture);
    for record in records {
        fx.manifest.register_artifact(record);
    }
    // Only serialized manifest + on-disk artifacts survive the process boundary.
    let path = fx.dir.join("reopened.json");
    fs::write(&path, serde_json::to_vec(&fx.manifest).unwrap()).unwrap();
    let mut reopened: Manifest = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    let ctx = StageContext {
        manifest: &reopened,
        registry: &registry,
        config: &config,
    };
    let (_, loaded) = read_alignment_pcm(&child, &ctx).unwrap().unwrap();
    assert_eq!(loaded.decoded.channel_samples, expected.channels);
    let rms = (loaded
        .decoded
        .channel_samples
        .iter()
        .flatten()
        .map(|v| v * v)
        .sum::<f64>()
        / (capture.len() * 2) as f64)
        .sqrt();
    // マルチch rms が per-channel sum_sq 再合成に移行したため、
    // 平坦シーケンシャル和との厳密比較は許容比較へ緩和する
    // (ulp 級の結合順差 — 許容再合成ポリシー)。
    assert!((rms - expected.energy.after).abs() < 1e-15);
    assert!((search["energy"]["after"].as_f64().unwrap() - expected.energy.after).abs() < 1e-15);
    let mut missing = child.clone();
    missing.capture_pcm = None;
    assert!(read_alignment_pcm(&missing, &ctx).is_err());
    let mut wrong = child.clone();
    wrong.capture_pcm.as_mut().unwrap().channels += 1;
    assert!(read_alignment_pcm(&wrong, &ctx).is_err());
    let mut wrong = child.clone();
    wrong.capture_pcm = parent.capture_pcm.clone();
    assert!(read_alignment_pcm(&wrong, &ctx).is_err());
    let idx = reopened
        .artifacts
        .iter()
        .position(|r| r.artifact_id == residual_id)
        .unwrap();
    let original = reopened.artifacts[idx].clone();
    for (pointer, value) in [
        ("/pcm/frames", serde_json::json!(1)),
        ("/pcm/sample_rate", serde_json::json!(24000)),
        ("/pcm/channels", serde_json::json!(1)),
        ("/encoding", serde_json::json!("wav-f32le")),
        (
            "/parent_alignment",
            serde_json::json!(al_core::AlignmentId::new()),
        ),
        (
            "/channel_transform",
            serde_json::json!([[1.0, 0.0], [0.0, 1.0]]),
        ),
    ] {
        reopened.artifacts[idx] = original.clone();
        *reopened.artifacts[idx].params.pointer_mut(pointer).unwrap() = value;
        let ctx = StageContext {
            manifest: &reopened,
            registry: &registry,
            config: &config,
        };
        assert!(read_alignment_pcm(&child, &ctx).is_err(), "{pointer}");
    }
    reopened.artifacts[idx] = original.clone();
    reopened.artifacts[idx].inputs.clear();
    let ctx = StageContext {
        manifest: &reopened,
        registry: &registry,
        config: &config,
    };
    assert!(read_alignment_pcm(&child, &ctx).is_err());
    reopened.artifacts[idx] = original.clone();
    let ctx = StageContext {
        manifest: &reopened,
        registry: &registry,
        config: &config,
    };
    fs::write(fx.dir.join(&original.path), [0; 8]).unwrap();
    assert!(read_alignment_pcm(&child, &ctx).is_err());
    fs::remove_file(fx.dir.join(&original.path)).unwrap();
    assert!(read_alignment_pcm(&child, &ctx).is_err());
    reopened.artifacts.remove(idx);
    let ctx = StageContext {
        manifest: &reopened,
        registry: &registry,
        config: &config,
    };
    assert!(read_alignment_pcm(&child, &ctx).is_err());
    fs::remove_dir_all(&fx.dir).unwrap();
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
    assert!(
        (occurrences[0] as i64 - other).abs() <= 64,
        "occurrence {} vs expected ~{other} (primary {primary})",
        occurrences[0]
    );

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
fn different_rates_locate_without_false_drift() {
    // 同じ連続時間 chirp を別々のレートで標本化し、リサンプラ自身を正解生成に使わない。
    let signal = |t: f64| (std::f64::consts::TAU * (500.0 * t + 40000.0 * t * t)).sin() * 0.7;
    let source: Vec<f64> = (0..4410).map(|i| signal(i as f64 / 44100.0)).collect();
    let mut capture = vec![0.0; 9600];
    for i in 0..4800 {
        capture[2400 + i] = signal(i as f64 / 48000.0);
    }
    let fx = fixture(&source, &capture, (44100, 48000));
    let (records, body) = run_and_read(&fx);
    let result = &body["result"];
    assert_eq!(result["accepted"], true, "{result}");
    assert!(
        result["capture_range"]["start"]
            .as_u64()
            .unwrap()
            .abs_diff(2400)
            <= 2
    );
    assert_eq!(result["source_range"]["end"], 4410);
    assert_eq!(result["normalized_source_range"]["end"], 4800);
    assert_eq!(body["derived"]["resampled"], true);
    assert_eq!(
        result["params"]["normalization"]["original_sample_rate"],
        44100
    );
    assert_eq!(result["params"]["normalization"]["sample_rate"], 48000);
    assert_eq!(
        result["params"]["normalization"]["transform"]["algorithm_id"],
        "polyphase-blackman-sinc"
    );
    let tm = records.iter().find(|r| r.role == "time_map").unwrap();
    let tm: serde_json::Value =
        serde_json::from_slice(&fs::read(fx.dir.join(&tm.path)).unwrap()).unwrap();
    for segment in tm["time_map"]["segments"].as_array().unwrap() {
        assert_eq!(segment["kind"], "continuous", "{segment}");
    }
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

#[test]
fn normalized_pcm_reuse_layout_coordinates_and_fail_closed_loading() {
    use al_locate::decode::{decode_input, read_normalized_pcm};
    use al_locate::normalize::prepare_input;
    use al_locate::result::SampleRange;
    let mut fx = fixture(&[0.25, -0.5, 1.2], &[0.0; 6], (44100, 48000));
    let original = fx
        .manifest
        .artifacts
        .iter_mut()
        .find(|a| a.role == "decoded_pcm")
        .unwrap();
    original.params["channels"] = serde_json::json!(2);
    write_pcm(
        &fx.dir,
        "inputs/src.f64le",
        &[0.25, -0.25, -0.5, 0.5, 1.2, -1.2],
    );
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &config,
    };
    let source = decode_input(&fx.manifest.inputs[0], &ctx).unwrap().unwrap();
    let mut records = Vec::new();
    let identity = prepare_input(&source, 44100, &mut records, &ctx).unwrap();
    let normalized = prepare_input(&source, 48000, &mut records, &ctx).unwrap();
    let repeated = prepare_input(&source, 48000, &mut records, &ctx).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(normalized.record.artifact_id, repeated.record.artifact_id);
    assert_ne!(identity.record.artifact_id, normalized.record.artifact_id);
    assert_eq!(identity.decoded.channel_samples, source.channel_samples);
    assert_eq!(normalized.params.frames, 4);
    assert_eq!(
        normalized
            .params
            .original_range(SampleRange { start: 1, end: 2 })
            .unwrap(),
        SampleRange { start: 0, end: 2 }
    );
    assert_eq!(
        normalized
            .params
            .original_range(SampleRange { start: 3, end: 4 })
            .unwrap(),
        SampleRange { start: 2, end: 3 }
    );
    assert_eq!(normalized.record.inputs, source.artifact_ids);
    for record in records {
        fx.manifest.register_artifact(record);
    }
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &config,
    };
    let reference = normalized.reference();
    let range = SampleRange { start: 0, end: 4 };
    let loaded = read_normalized_pcm(&reference, range, &ctx).unwrap();
    assert_eq!(
        loaded.decoded.channel_samples,
        normalized.decoded.channel_samples
    );
    assert!(
        loaded.decoded.channel_samples[0]
            .iter()
            .zip(&loaded.decoded.channel_samples[1])
            .all(|(a, b)| a == &-b)
    );
    assert!(read_normalized_pcm(&reference, SampleRange { start: 5, end: 4 }, &ctx).is_err());
    assert!(read_normalized_pcm(&reference, SampleRange { start: 0, end: 5 }, &ctx).is_err());
    let mut wrong = reference.clone();
    wrong.sample_rate = 44100;
    assert!(read_normalized_pcm(&wrong, range, &ctx).is_err());
    wrong = reference.clone();
    wrong.channels = 1;
    assert!(read_normalized_pcm(&wrong, range, &ctx).is_err());
    wrong = reference.clone();
    wrong.artifact_id = al_core::ArtifactId::new();
    assert!(read_normalized_pcm(&wrong, range, &ctx).is_err());
    let index = fx
        .manifest
        .artifacts
        .iter()
        .position(|a| a.artifact_id == reference.artifact_id)
        .unwrap();
    for variant in 0..5 {
        let mut manifest = fx.manifest.clone();
        let record = &mut manifest.artifacts[index];
        match variant {
            0 => record.role = "decoded_pcm".into(),
            1 => record.params["frames"] = serde_json::json!(3),
            2 => record.params["transform"]["cutoff"] = serde_json::json!(0.5),
            3 => record.inputs.clear(),
            _ => record.path = PathBuf::from("../escape.f64le"),
        }
        let ctx = StageContext {
            manifest: &manifest,
            registry: &registry,
            config: &config,
        };
        assert!(
            read_normalized_pcm(&reference, range, &ctx).is_err(),
            "variant {variant}"
        );
    }
    fs::write(fx.dir.join(&normalized.record.path), [0; 8]).unwrap();
    assert!(read_normalized_pcm(&reference, range, &ctx).is_err());
    fs::remove_file(fx.dir.join(&normalized.record.path)).unwrap();
    assert!(read_normalized_pcm(&reference, range, &ctx).is_err());
    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn normalized_pcm_applies_integer_scale_once() {
    use al_locate::decode::{decode_input, read_normalized_pcm};
    use al_locate::normalize::prepare_input;
    use al_locate::result::SampleRange;
    let mut fx = fixture(&[16384.0, -32768.0], &[0.0; 2], (48000, 48000));
    let meta_id = al_core::ArtifactId::new();
    fs::write(
        fx.dir.join("inputs/metadata.json"),
        r#"{"audio":{"sample_format":"pcm_s16"}}"#,
    )
    .unwrap();
    fx.manifest.register_artifact(ArtifactRecord {
        artifact_id: meta_id.clone(),
        role: "ingest_metadata".into(),
        path: PathBuf::from("inputs/metadata.json"),
        produced_by: Some(StageKind::Ingest),
        inputs: vec![
            fx.manifest
                .input_artifact(&fx.manifest.inputs[0].input_id)
                .unwrap(),
        ],
        params: serde_json::json!({}),
    });
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &config,
    };
    let original = decode_input(&fx.manifest.inputs[0], &ctx).unwrap().unwrap();
    assert_eq!(original.channel_samples[0], [0.5, -1.0]);
    let mut records = Vec::new();
    let prepared = prepare_input(&original, 48000, &mut records, &ctx).unwrap();
    assert!(prepared.record.inputs.contains(&meta_id));
    for r in records {
        fx.manifest.register_artifact(r);
    }
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &config,
    };
    let loaded = read_normalized_pcm(
        &prepared.reference(),
        SampleRange { start: 0, end: 2 },
        &ctx,
    )
    .unwrap();
    assert_eq!(loaded.decoded.channel_samples[0], [0.5, -1.0]);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== mechanical-caller-cleanup characterization (A15) ====
// TASK-034EO484s25JM8bsb6zUCh (plan T1). Pins the CURRENT byte-level
// contract of the normalized `.f64le` image `prepare_input` writes:
// frame-major, channel-inner little-endian f64 — byte equality, not
// round-trip tolerance (test-spec Rules C/D).
#[test]
fn a15_normalized_pcm_f64le_bytes_are_exact() {
    use al_locate::decode::decode_input;
    use al_locate::normalize::prepare_input;

    // Stereo decoded fixture: the f64le source is interleaved, so
    // `decode_input` must split it into two channel vectors.
    let left = noise(64, 0x1e);
    let right = noise(64, 0x1f);
    let interleaved: Vec<f64> = (0..64).flat_map(|i| [left[i], right[i]]).collect();
    let mut fx = fixture(&interleaved, &[0.0; 8], (48_000, 48_000));
    let original = fx
        .manifest
        .artifacts
        .iter_mut()
        .find(|a| a.role == "decoded_pcm")
        .unwrap();
    original.params["channels"] = serde_json::json!(2);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &config,
    };
    let source = decode_input(&fx.manifest.inputs[0], &ctx).unwrap().unwrap();
    assert_eq!(source.channel_samples.len(), 2);
    // Identity rate → the resample path returns the channels
    // unchanged, so the byte image is fully reconstructible.
    let mut records = Vec::new();
    let prepared = prepare_input(&source, 48_000, &mut records, &ctx).unwrap();
    let expected: Vec<u8> = (0..prepared.params.frames as usize)
        .flat_map(|f| {
            prepared
                .decoded
                .channel_samples
                .iter()
                .map(move |c| c[f].to_le_bytes())
        })
        .flatten()
        .collect();
    let actual = fs::read(fx.dir.join(&prepared.record.path)).unwrap();
    assert_eq!(actual, expected, "normalized .f64le byte image");
    // The parsed stream deinterleaves back to the written channels
    // bit-for-bit.
    let parsed: Vec<f64> = actual
        .as_chunks::<8>()
        .0
        .iter()
        .map(|b| f64::from_le_bytes(*b))
        .collect();
    let chans = al_locate::deinterleave(&parsed, 2);
    for (c, (a, e)) in chans
        .iter()
        .zip(&prepared.decoded.channel_samples)
        .enumerate()
    {
        assert_eq!(a.len(), e.len(), "ch{c} len");
        for (i, (&x, &y)) in a.iter().zip(e).enumerate() {
            assert_eq!(x.to_bits(), y.to_bits(), "ch{c}[{i}]");
        }
    }
    fs::remove_dir_all(&fx.dir).ok();
}
