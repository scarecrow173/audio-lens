//! ReferenceAnalysisStage integration tests over a synthesized job.
//!
//! The accepted-alignment test runs the real LocateStage first so the
//! `locate_result` artifact and `result.json` come from the actual
//! upstream contract, then runs the reference stage against the
//! job-scoped `analysis/` dataset.

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

/// A job with one `source_asset` and one `capture_bug` whose decoded
/// mono 48 kHz PCM lives under `inputs/`. Inputs carry a pinned
/// `source_sha256` so `input_references` succeeds.
fn fixture(source: &[f64], capture: &[f64]) -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-ref-test-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut src_input = input(InputRole::SourceAsset, dir.join("src.bin"));
    let mut cap_input = input(InputRole::CaptureBug, dir.join("cap.bin"));
    src_input.source_sha256 = Some("aa".repeat(32));
    cap_input.source_sha256 = Some("bb".repeat(32));
    let source_input_id = src_input.input_id.to_string();
    let capture_input_id = cap_input.input_id.to_string();
    let input_ids = [src_input.input_id.clone(), cap_input.input_id.clone()];
    let mut manifest = Manifest::new(
        JobId::new(),
        vec![src_input, cap_input],
        serde_json::json!({}),
        &[
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
            StageKind::ReferenceAnalysis,
        ],
    );

    for (idx, (rel, samples)) in [("inputs/src.f64le", source), ("inputs/cap.f64le", capture)]
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
            params: serde_json::json!({"sample_rate": 48_000, "channels": 1}),
        });
    }

    Fixture {
        dir,
        manifest,
        source_input_id,
        capture_input_id,
    }
}

fn ctx<'a>(
    fx: &'a Fixture,
    registry: &'a ArtifactRegistry,
    config: &'a serde_json::Value,
) -> StageContext<'a> {
    StageContext {
        manifest: &fx.manifest,
        registry,
        config,
    }
}

/// Read the persisted feature records for the pair.
fn feature_records(fx: &Fixture) -> serde_json::Value {
    let path = fx
        .dir
        .join("analysis")
        .join("features")
        .join("reference")
        .join(format!(
            "{}__{}.json",
            fx.source_input_id, fx.capture_input_id
        ));
    serde_json::from_str(&fs::read_to_string(path).expect("feature records persisted"))
        .expect("feature records parse")
}

fn scalar(records: &serde_json::Value, id: &str) -> serde_json::Value {
    records["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["feature_id"] == id)
        .unwrap_or_else(|| panic!("missing record {id}"))
        .clone()
}

fn scalar_value(r: &serde_json::Value) -> f64 {
    r["value"]["scalar"].as_f64().expect("scalar value")
}

#[test]
fn accepted_alignment_produces_features_residuals_and_dataset() {
    // Capture = embedded 0.5× source at 5000 + low background noise.
    let source = noise(16_384, 7);
    let mut capture = noise(30_000, 11);
    for v in &mut capture {
        *v *= 0.05;
    }
    for (i, &v) in source.iter().enumerate() {
        capture[5_000 + i] += 0.5 * v;
    }
    let mut fx = fixture(&source, &capture);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});

    // Run locate, register its artifacts (runner role).
    let records = LocateStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("locate");
    for rec in records {
        fx.manifest.register_artifact(rec);
    }

    let records = al_analysis::stage::ReferenceAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("reference stage");
    assert!(records.iter().any(|r| r.role == "reference_features"));

    let body = feature_records(&fx);
    let gain = scalar(&body, "ref.gain_ls");
    assert_eq!(gain["validity"], "valid", "gain record: {gain}");
    let g = scalar_value(&gain);
    assert!(g > 0.2 && g < 0.8, "gain {g}");

    // Every record references the alignment id issued by locate —
    // not the dataset path stem.
    let result: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            fx.dir
                .join("locate")
                .join(&fx.source_input_id)
                .join(&fx.capture_input_id)
                .join("result.json"),
        )
        .expect("locate result.json"),
    )
    .expect("locate result parses");
    let aln = result["result"]["alignment_id"]
        .as_str()
        .expect("alignment_id");
    for r in body["records"].as_array().unwrap() {
        assert_eq!(
            r["alignment_id"].as_str(),
            Some(aln),
            "record {} must reference the locate alignment",
            r["feature_id"]
        );
    }

    // Intermediate tensors produced by the reference stage record
    // reference.v1 + the same alignment, not the cheap producer.
    let reader =
        al_dataset::reader::DatasetReader::open(&fx.dir.join("analysis")).expect("dataset opens");
    let pair_prefix = format!(
        "intermediate/{}__{}",
        fx.source_input_id, fx.capture_input_id
    );
    let tensors: Vec<_> = reader
        .artifacts("intermediate")
        .into_iter()
        .filter(|a| a.path.starts_with(&pair_prefix))
        .collect();
    assert!(!tensors.is_empty(), "no intermediate tensors persisted");
    let band_tensor = tensors
        .iter()
        .find(|a| a.path.ends_with("channel_0/band_gain"))
        .expect("persisted band gain tensor");
    assert_eq!(band_tensor.provenance.params["max_lag_samples"], 1920);
    assert_eq!(
        band_tensor.provenance.params["coherence_estimator"],
        "welch_magnitude_squared"
    );
    let name = band_tensor.path.strip_prefix("intermediate/").unwrap();
    let tensor = reader.open_tensor("intermediate", name).unwrap();
    assert!(tensor.meta().shape[0] > 0 && tensor.meta().shape[1] > 0);
    let shape = tensor.meta().shape.clone();
    let (values, _) = reader
        .read_tensor("intermediate", name, &[[0, shape[0]], [0, shape[1]]])
        .unwrap();
    assert!(
        values
            .iter()
            .any(|v| v.is_finite() && (*v - 0.5).abs() < 0.1)
    );
    let m2 = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| {
            r["feature_id"] == "ref.band_gain_m2"
                && r["params"]["polarity"] == "normal"
                && r["validity"] == "valid"
        })
        .expect("supported M2 observation");
    assert_eq!(m2["algorithm_id"], "reference.band_lag_ls.v1");
    assert_eq!(m2["params"]["max_lag_samples"], 1920);
    assert!(
        body["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["feature_id"] == "ref.band_gain_psd_ratio")
    );
    for a in tensors {
        assert_eq!(
            a.provenance.algorithm_id,
            if a.path.contains("/channel_") {
                "reference.band_lag_ls.v1"
            } else {
                "reference.v1"
            },
            "{}",
            a.path
        );
        assert_eq!(
            a.provenance.alignment_id.as_deref(),
            Some(aln),
            "{}",
            a.path
        );
    }

    // R0-R3 residual RMS ratios are present and non-increasing.
    let ratios: Vec<f64> = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["feature_id"] == "ref.residual_rms_ref_ratio")
        .map(scalar_value)
        .collect();
    assert_eq!(ratios.len(), 4, "residual stages: {ratios:?}");
    for w in ratios.windows(2) {
        assert!(w[1] <= w[0] + 1e-9, "non-monotonic: {ratios:?}");
    }

    // Residual audio is materialized under the dataset's residual role.
    for stage in ["r0", "r1", "r2", "r3"] {
        assert!(
            fx.dir
                .join("analysis")
                .join("residual")
                .join(format!("{}__{}", fx.source_input_id, fx.capture_input_id))
                .join(format!("{stage}.wav"))
                .exists(),
            "missing residual wav {stage}"
        );
    }

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn rejected_alignment_persists_invalid_rows_and_completes() {
    let source = noise(4_096, 13);
    let capture = noise(8_000, 17);
    let mut fx = fixture(&source, &capture);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});

    for rec in LocateStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap()
    {
        fx.manifest.register_artifact(rec);
    }

    let alignment_id = fx
        .manifest
        .artifacts
        .iter()
        .find(|a| a.role == "locate_result")
        .unwrap()
        .params["alignment_id"]
        .as_str()
        .unwrap()
        .to_string();
    al_analysis::stage::ReferenceAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("stage must complete on rejected alignment");

    let body = feature_records(&fx);
    let records = body["records"].as_array().unwrap();
    assert!(!records.is_empty());
    for r in records {
        assert_ne!(r["validity"], "valid", "record {}", r["feature_id"]);
        assert_eq!(
            r["alignment_id"].as_str(),
            Some(alignment_id.as_str()),
            "record {} must reference the issued alignment",
            r["feature_id"]
        );
    }

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn normalized_alignment_survives_process_boundary() {
    use al_locate::decode::read_alignment_pcm;
    use al_locate::result::LocateResult;
    const CHILD_ENV: &str = "AL_TEST_NORMALIZED_JOB";
    if let Some(path) = std::env::var_os(CHILD_ENV) {
        let dir = PathBuf::from(path);
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(dir.join("snapshot.json")).unwrap()).unwrap();
        let source_input_id = manifest.inputs[0].input_id.to_string();
        let capture_input_id = manifest.inputs[1].input_id.to_string();
        let fx = Fixture {
            dir,
            manifest,
            source_input_id,
            capture_input_id,
        };
        let registry = ArtifactRegistry::new(&fx.dir);
        let config =
            serde_json::json!({"analysis":{"expensive":{"always_run":true,"stft_window":512}}});
        let artifact = fx
            .manifest
            .artifacts
            .iter()
            .find(|a| a.role == "locate_result")
            .unwrap();
        let body: serde_json::Value =
            serde_json::from_slice(&fs::read(fx.dir.join(&artifact.path)).unwrap()).unwrap();
        let result: LocateResult = serde_json::from_value(body["result"].clone()).unwrap();
        let pcm = read_alignment_pcm(&result, &ctx(&fx, &registry, &config))
            .unwrap()
            .unwrap();
        assert_eq!(pcm.0.params.original_sample_rate, 44100);
        assert_eq!(pcm.0.params.sample_rate, 48000);
        assert_eq!(result.source_range.end, 4410);
        assert_eq!(result.normalized_source_range.end, 4800);
        assert!(pcm.0.decoded.channel_samples[0].iter().any(|s| *s != 0.0));
        let expected: Vec<f64> = fs::read(fx.dir.join("expected.f64le"))
            .unwrap()
            .as_chunks::<8>()
            .0
            .iter()
            .map(|b| f64::from_le_bytes(*b))
            .collect();
        assert_eq!(pcm.0.decoded.channel_samples[0], expected);
        let records = al_analysis::stage::ReferenceAnalysisStage::new()
            .run(&ctx(&fx, &registry, &config))
            .unwrap();
        for record in &records {
            assert!(record.inputs.contains(&artifact.artifact_id));
            assert!(record.inputs.contains(&pcm.0.record.artifact_id));
            assert!(record.inputs.contains(&pcm.1.record.artifact_id));
        }
        let gain = scalar(&feature_records(&fx), "ref.gain_ls");
        assert_eq!(gain["validity"], "valid", "{gain}");
        assert!((scalar_value(&gain) - 0.5).abs() < 0.02);
        let records = al_analysis::stage::ExpensiveAnalysisStage::new()
            .run(&ctx(&fx, &registry, &config))
            .unwrap();
        let cap_record = records
            .iter()
            .find(|r| {
                r.role == "expensive_features"
                    && r.path.to_string_lossy().contains(&fx.capture_input_id)
            })
            .unwrap();
        assert!(cap_record.inputs.contains(&artifact.artifact_id));
        assert!(cap_record.inputs.contains(&pcm.0.record.artifact_id));
        assert!(cap_record.inputs.contains(&pcm.1.record.artifact_id));
        let body: serde_json::Value =
            serde_json::from_slice(&fs::read(fx.dir.join(&cap_record.path)).unwrap()).unwrap();
        let record = body["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["feature_id"] == "exp.mix_fit_quality")
            .unwrap();
        assert_eq!(record["validity"], "valid", "{record}");
        let mut inconsistent = result.clone();
        inconsistent.source_range.end -= 1;
        assert!(read_alignment_pcm(&inconsistent, &ctx(&fx, &registry, &config)).is_err());
        inconsistent = result.clone();
        inconsistent.normalized_source_range.end += 1;
        assert!(read_alignment_pcm(&inconsistent, &ctx(&fx, &registry, &config)).is_err());
        fs::remove_file(fx.dir.join(&pcm.0.record.path)).unwrap();
        assert!(
            al_analysis::stage::ReferenceAnalysisStage::new()
                .run(&ctx(&fx, &registry, &config))
                .is_err()
        );
        assert!(
            al_analysis::stage::ExpensiveAnalysisStage::new()
                .run(&ctx(&fx, &registry, &config))
                .is_err()
        );
        return;
    }
    let source = noise(4410, 314);
    let mut fx = fixture(&source, &[0.0; 9000]);
    fx.manifest
        .artifacts
        .iter_mut()
        .find(|a| a.role == "decoded_pcm")
        .unwrap()
        .params["sample_rate"] = serde_json::json!(44100);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    let input =
        al_locate::decode::decode_input(&fx.manifest.inputs[0], &ctx(&fx, &registry, &config))
            .unwrap()
            .unwrap();
    let mut prepared = Vec::new();
    let source_pcm = al_locate::normalize::prepare_input(
        &input,
        48000,
        &mut prepared,
        &ctx(&fx, &registry, &config),
    )
    .unwrap();
    let mut capture = vec![0.0; 9000];
    for (i, sample) in source_pcm.decoded.channel_samples[0].iter().enumerate() {
        capture[1000 + i] = 0.5 * sample;
    }
    write_pcm(&fx.dir, "inputs/cap.f64le", &capture);
    write_pcm(
        &fx.dir,
        "expected.f64le",
        &source_pcm.decoded.channel_samples[0],
    );
    for r in prepared {
        fx.manifest.register_artifact(r);
    }
    for r in LocateStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap()
    {
        fx.manifest.register_artifact(r);
    }
    // 元デコード値を変えても、後段比較は永続化した PCM を使う。
    write_pcm(&fx.dir, "inputs/src.f64le", &[0.0; 4410]);
    fs::write(
        fx.dir.join("snapshot.json"),
        serde_json::to_vec(&fx.manifest).unwrap(),
    )
    .unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "normalized_alignment_survives_process_boundary",
            "--nocapture",
        ])
        .env(CHILD_ENV, &fx.dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn cross_capture_path_gain_is_job_scoped_and_preserves_pair_features() {
    let source = noise(16384, 123);
    let mut bug = vec![0.0; 30000];
    let mut reference = vec![0.0; 30000];
    for (i, v) in source.iter().enumerate() {
        bug[5000 + i] = v * 0.8;
        reference[5000 + i] = v * 0.4;
    }
    let mut fx = fixture(&source, &bug);
    let mut ref_input = input(InputRole::CaptureReference, fx.dir.join("reference.bin"));
    ref_input.source_sha256 = Some("dd".repeat(32));
    let ref_id = ref_input.input_id.clone();
    let mut inputs = fx.manifest.inputs.clone();
    inputs.push(ref_input);
    let old_decoded: Vec<_> = fx
        .manifest
        .artifacts
        .iter()
        .filter(|a| a.role == "decoded_pcm")
        .cloned()
        .collect();
    fx.manifest = Manifest::new(JobId::new(), inputs, serde_json::json!({}), &[]);
    for mut a in old_decoded {
        let id = if a.path.to_string_lossy().contains("src") {
            &fx.manifest.inputs[0].input_id
        } else {
            &fx.manifest.inputs[1].input_id
        };
        a.inputs = vec![fx.manifest.input_artifact(id).unwrap()];
        fx.manifest.register_artifact(a);
    }
    write_pcm(&fx.dir, "inputs/reference.f64le", &reference);
    fx.manifest.register_artifact(ArtifactRecord {
        artifact_id: al_core::ArtifactId::new(),
        role: "decoded_pcm".into(),
        path: "inputs/reference.f64le".into(),
        produced_by: Some(StageKind::Ingest),
        inputs: vec![fx.manifest.input_artifact(&ref_id).unwrap()],
        params: serde_json::json!({"sample_rate":48000,"channels":1}),
    });
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({"analysis":{"reference":{"stft_window":512,"stft_hop":256}}});
    for r in LocateStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap()
    {
        fx.manifest.register_artifact(r);
    }
    for r in al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap()
    {
        fx.manifest.register_artifact(r);
    }
    for r in al_analysis::stage::ReferenceAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap()
    {
        fx.manifest.register_artifact(r);
    }
    let before = fs::read(fx.dir.join(format!(
        "analysis/features/reference/{}__{}.json",
        fx.source_input_id, fx.capture_input_id
    )))
    .unwrap();
    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap();
    let rec = recs
        .iter()
        .find(|r| r.role == "cross_capture_features")
        .unwrap();
    let comparison: al_dataset::types::CaptureComparison =
        serde_json::from_slice(&fs::read(fx.dir.join(&rec.path)).unwrap()).unwrap();
    assert_eq!(comparison.reference_capture_id, ref_id.as_str());
    assert_eq!(comparison.target_capture_id, fx.capture_input_id);
    assert_eq!(comparison.common_source_ids, [fx.source_input_id.clone()]);
    assert_eq!(comparison.records[0].validity, al_core::Validity::Valid);
    assert_eq!(comparison.records[0].input_id, "job");
    let reader = al_dataset::DatasetReader::open(&fx.dir.join("analysis")).unwrap();
    let name = format!(
        "cross_capture/{}__{}/path_gain",
        ref_id, fx.capture_input_id
    );
    let (data, _) = reader
        .read_tensor("intermediate", &name, &[[0, 3], [1, 4]])
        .unwrap();
    assert!(data[..3].iter().all(|r| (r - 2.0).abs() < 0.02), "{data:?}");
    assert_eq!(&data[6..], &[1.0; 3]);
    assert_eq!(
        before,
        fs::read(fx.dir.join(format!(
            "analysis/features/reference/{}__{}.json",
            fx.source_input_id, fx.capture_input_id
        )))
        .unwrap()
    );
    // locate がない場合もキャプチャ組を消さず、観測不能を保存する。
    fx.manifest.artifacts.retain(|a| a.role != "locate_result");
    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .unwrap();
    let rec = recs
        .iter()
        .find(|r| r.role == "cross_capture_features")
        .unwrap();
    let comparison: al_dataset::types::CaptureComparison =
        serde_json::from_slice(&fs::read(fx.dir.join(&rec.path)).unwrap()).unwrap();
    assert_eq!(
        comparison.records[0].validity,
        al_core::Validity::Unsupported
    );
    fs::remove_dir_all(fx.dir).unwrap();
}
