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
        let reference_body = feature_records(&fx);
        let resolutions: Vec<_> = reference_body["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| !r["resolution"].is_null())
            .collect();
        assert!(!resolutions.is_empty());
        for record in resolutions {
            assert_eq!(
                record["resolution"]["sample_rate"], 48000,
                "{}",
                record["feature_id"]
            );
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
        let resolutions: Vec<_> = body["records"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| !r["resolution"].is_null())
            .collect();
        assert!(!resolutions.is_empty());
        for record in resolutions {
            assert_eq!(
                record["resolution"]["sample_rate"], 48000,
                "{}",
                record["feature_id"]
            );
        }
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

// ==== mechanical-caller-cleanup characterization ====
// TASK-034EO484s25JM8bsb6zUCh (plan T1). These tests pin the
// CURRENT outputs of every site the mechanical caller cleanup
// bundle touches, before any optimization lands. All comparisons
// are `f64::to_bits` / element-exact — no tolerance predicate is
// applied anywhere in this bundle (test-spec Rule A). Oracles are
// re-implemented here over the same public API (`bandpass` / `csd`
// / `StftMatrix::{psd,magnitude}` / `fit_mix` stay public), so the
// tests stay green on both sides of each -opt commit.
mod mechanical_cleanup {
    use super::*;
    use al_analysis::reference::{Aligned, ReferenceConfig, ReferenceResult, analyze_pair};
    use al_analysis::stft::{
        StftMatrix, band_power, complex_coherence, csd, stft, third_octave_bands,
    };
    use al_core::{Confidence, Validity};
    use al_dataset::types::Value;
    use ruviform_numeric::bands::bandpass;
    use ruviform_numeric::vec::corr0;

    const ALN: &str = "ALN-0123456789abcdefghijkl";

    fn scalar(r: &al_dataset::types::FeatureRecord) -> f64 {
        match &r.value {
            Value::Scalar(v) => *v,
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    fn record<'a>(out: &'a ReferenceResult, id: &str) -> &'a al_dataset::types::FeatureRecord {
        out.records
            .iter()
            .find(|r| r.feature_id == id)
            .unwrap_or_else(|| panic!("missing record {id}"))
    }

    fn tensor<'a>(out: &'a ReferenceResult, path: &str) -> &'a al_analysis::cheap::NamedTensor {
        out.tensors
            .iter()
            .find(|t| t.path == path)
            .unwrap_or_else(|| panic!("missing tensor {path}"))
    }

    fn assert_bits(actual: &[f64], expected: &[f64], what: &str) {
        assert_eq!(
            actual.len(),
            expected.len(),
            "{what} length {} vs {}",
            actual.len(),
            expected.len()
        );
        for (i, (&a, &e)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(a.to_bits(), e.to_bits(), "{what}[{i}]");
        }
    }

    /// src mono, cap stereo — the stereo capture keeps the L/R
    /// coherence site and the multi-channel residual interleave on
    /// the analysis path.
    fn pair_fixture() -> (Vec<Vec<f64>>, Vec<Vec<f64>>, u32) {
        let rate = 48_000u32;
        let n = 1 << 15;
        let src0 = noise(n, 0x5eed);
        let cap0: Vec<f64> = (0..n)
            .map(|i| if i >= 5 { 0.7 * src0[i - 5] } else { 0.0 })
            .collect();
        let cap1: Vec<f64> = (0..n)
            .map(|i| if i >= 9 { 0.4 * src0[i - 9] } else { 0.0 })
            .collect();
        (vec![src0], vec![cap0, cap1], rate)
    }

    fn analyze(src: &[Vec<f64>], cap: &[Vec<f64>], rate: u32) -> ReferenceResult {
        let aligned = Aligned {
            src: src.iter().map(Vec::as_slice).collect(),
            cap: cap.iter().map(Vec::as_slice).collect(),
            rate,
            upstream_confidence: Confidence::High,
        };
        analyze_pair(
            "src__cap",
            &al_core::AlignmentId::parse(ALN).unwrap(),
            &aligned,
            Validity::Valid,
            &ReferenceConfig::default(),
        )
    }

    /// Current `analyze_pair` band set (1/3-octave, capped at 8).
    fn ref_bands(rate: u32) -> Vec<(f64, f64)> {
        third_octave_bands(40.0, (rate as f64 / 2.0).min(16_000.0))
    }

    /// Oracle for the Welch accumulation shared by the transfer_h /
    /// coherence / hft sites (reference.rs A7 sites + A8).
    struct Welch {
        h_re: Vec<f64>,
        h_im: Vec<f64>,
        coh: Vec<(f64, f64)>,
        hft: Vec<Vec<f64>>,
    }

    fn welch(ms: &StftMatrix, mc: &StftMatrix) -> Welch {
        let frames = ms.frames().min(mc.frames());
        assert!(frames > 0, "fixture must produce STFT frames");
        let bins = ms.fft_size;
        let mut pxx = vec![0.0; bins];
        let mut pyy = vec![0.0; bins];
        let mut pxy_re = vec![0.0; bins];
        let mut pxy_im = vec![0.0; bins];
        for f in 0..frames {
            let (cr, ci) = csd(&ms.re[f], &ms.im[f], &mc.re[f], &mc.im[f]);
            for k in 0..bins {
                pxx[k] += ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
                pyy[k] += mc.re[f][k] * mc.re[f][k] + mc.im[f][k] * mc.im[f][k];
                pxy_re[k] += cr[k];
                pxy_im[k] += ci[k];
            }
        }
        for k in 0..bins {
            pxx[k] /= frames as f64;
            pyy[k] /= frames as f64;
            pxy_re[k] /= frames as f64;
            pxy_im[k] /= frames as f64;
        }
        let (coh_re_v, coh_im_v) = complex_coherence(&pxx, &pyy, &pxy_re, &pxy_im);
        let coh: Vec<(f64, f64)> = coh_re_v
            .iter()
            .zip(&coh_im_v)
            .map(|(&r, &i)| (r, i))
            .collect();
        let mut h_re = vec![0.0; bins];
        let mut h_im = vec![0.0; bins];
        for k in 0..bins {
            if pxx[k] > 0.0 {
                h_re[k] = pxy_re[k] / pxx[k];
                h_im[k] = pxy_im[k] / pxx[k];
            }
        }
        let half = bins / 2;
        let hft: Vec<Vec<f64>> = (0..frames)
            .map(|f| {
                (0..half)
                    .map(|k| {
                        let xs = ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
                        if xs > 0.0 {
                            let (cr, ci) = csd(
                                &ms.re[f][k..k + 1],
                                &ms.im[f][k..k + 1],
                                &mc.re[f][k..k + 1],
                                &mc.im[f][k..k + 1],
                            );
                            ((cr[0] * cr[0] + ci[0] * ci[0]).sqrt()) / xs
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect();
        Welch {
            h_re,
            h_im,
            coh,
            hft,
        }
    }

    // ---- A6: band_correlation tensor + `bands < 8` edge ----

    fn assert_band_correlation(
        out: &ReferenceResult,
        src_mono: &[f64],
        cap_mono: &[f64],
        rate: u32,
    ) {
        let bands = ref_bands(rate);
        let expected: Vec<f64> = bands
            .iter()
            .take(8)
            .map(|&(lo, hi)| {
                let xs = bandpass(src_mono, rate, lo, hi)
                    .expect("band settings and input must be valid");
                let ys = bandpass(cap_mono, rate, lo, hi)
                    .expect("band settings and input must be valid");
                corr0(&xs, &ys).expect("equal lengths")
            })
            .collect();
        let t = tensor(out, "intermediate/src__cap/band_correlation");
        assert_bits(&t.data, &expected, "band_correlation");
        // ref.band_correlation_min records the minimum of the same
        // element list.
        let min = record(out, "ref.band_correlation_min");
        let expected_min = expected
            .iter()
            .copied()
            .min_by(|a, b| a.total_cmp(b))
            .unwrap();
        assert_eq!(
            scalar(min).to_bits(),
            expected_min.to_bits(),
            "ref.band_correlation_min"
        );
    }

    #[test]
    fn a6_band_correlation_tensor_is_bit_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        assert_band_correlation(&out, &src[0], &cap[0], rate);
    }

    #[test]
    fn a6_band_correlation_bands_below_eight_edge() {
        // Low sample rate → `bands.len() < 8`: the `.min(8)` /
        // `take(8)` bound must yield `bands.len()` elements without
        // panic (test-spec Rule B).
        let rate = 400u32;
        let n = 1 << 14;
        let src = vec![noise(n, 0xbee0)];
        let cap = vec![
            noise(n, 0xca90)
                .iter()
                .enumerate()
                .map(|(i, &v)| if i >= 3 { 0.6 * src[0][i - 3] } else { v * 0.1 })
                .collect(),
        ];
        let bands = ref_bands(rate);
        assert!(
            bands.len() < 8,
            "rate {rate} must yield fewer than 8 bands, got {}",
            bands.len()
        );
        let out = analyze(&src, &cap, rate);
        let t = tensor(&out, "intermediate/src__cap/band_correlation");
        assert_eq!(
            t.data.len(),
            bands.len(),
            "capped band_correlation element count"
        );
        assert_band_correlation(&out, &src[0], &cap[0], rate);
    }

    // ---- A7/A8: Welch CSD consumers (transfer_h / coherence /
    //      transfer_h_t tensors) ----

    #[test]
    fn a7_a8_welch_consumers_are_bit_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        let cfg = ReferenceConfig::default();
        let ms = stft(&src[0], cfg.stft_window, cfg.stft_hop, rate);
        let mc = stft(&cap[0], cfg.stft_window, cfg.stft_hop, rate);
        let w = welch(&ms, &mc);
        let cols = ms.fft_size;

        let th = tensor(&out, "intermediate/src__cap/transfer_h");
        assert_eq!(th.shape, [2, cols as u64]);
        assert_bits(&th.data[..cols], &w.h_re, "transfer_h re");
        assert_bits(&th.data[cols..], &w.h_im, "transfer_h im");

        let coh_t = tensor(&out, "intermediate/src__cap/coherence");
        let coh_re: Vec<f64> = w.coh.iter().map(|v| v.0).collect();
        let coh_im: Vec<f64> = w.coh.iter().map(|v| v.1).collect();
        assert_eq!(coh_t.shape, [2, cols as u64]);
        assert_bits(&coh_t.data[..cols], &coh_re, "coherence re");
        assert_bits(&coh_t.data[cols..], &coh_im, "coherence im");

        let hft_t = tensor(&out, "intermediate/src__cap/transfer_h_t");
        let flat_hft: Vec<f64> = w.hft.iter().flatten().copied().collect();
        assert_eq!(
            hft_t.shape,
            [w.hft.len() as u64, (cols / 2) as u64],
            "transfer_h_t shape"
        );
        assert_bits(&hft_t.data, &flat_hft, "transfer_h_t");
    }

    // ---- A7: band_gain_m2 record `params.coherence` — the
    //      band_gain.rs `coherence()` csd site ----

    /// Oracle for band_gain.rs `coherence()` (A7 site): per-band
    /// magnitude-squared coherence over `bands`.
    fn band_gain_coherence_oracle(
        x: &[f64],
        y: &[f64],
        rate: u32,
        cfg: &ReferenceConfig,
        bands: &[(f64, f64)],
    ) -> Vec<f64> {
        let a = stft(x, cfg.stft_window, cfg.stft_hop, rate);
        let b = stft(y, cfg.stft_window, cfg.stft_hop, rate);
        let frames = a.frames().min(b.frames());
        let bins = a.fft_size;
        let mut xx = vec![0.0; bins];
        let mut yy = vec![0.0; bins];
        let mut xy_re = vec![0.0; bins];
        let mut xy_im = vec![0.0; bins];
        for f in 0..frames {
            let (cr, ci) = csd(&a.re[f], &a.im[f], &b.re[f], &b.im[f]);
            for k in 0..bins {
                xx[k] += a.re[f][k].powi(2) + a.im[f][k].powi(2);
                yy[k] += b.re[f][k].powi(2) + b.im[f][k].powi(2);
                xy_re[k] += cr[k];
                xy_im[k] += ci[k];
            }
        }
        let (c_re, c_im) = complex_coherence(&xx, &yy, &xy_re, &xy_im);
        bands
            .iter()
            .map(|&(lo, hi)| {
                let low = (lo * bins as f64 / rate as f64).ceil() as usize;
                let high = ((hi * bins as f64 / rate as f64).ceil() as usize).min(bins / 2 + 1);
                if frames < 2 || high <= low {
                    0.0
                } else {
                    c_re[low..high]
                        .iter()
                        .zip(&c_im[low..high])
                        .map(|(&r, &i)| r * r + i * i)
                        .sum::<f64>()
                        / (high - low) as f64
                }
            })
            .collect()
    }

    #[test]
    fn a7_band_gain_m2_coherence_params_are_bit_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        let cfg = ReferenceConfig::default();
        let bands = ref_bands(rate);
        // Channel 0 pairs src.src[0] with src.cap[0] (band_gain.rs
        // `src.src.iter().zip(&src.cap)`).
        let oracle = band_gain_coherence_oracle(&src[0], &cap[0], rate, &cfg, &bands);
        let recs: Vec<_> = out
            .records
            .iter()
            .filter(|r| {
                r.feature_id == "ref.band_gain_m2"
                    && r.channels == Some(al_dataset::types::ChannelSelector::Channel(0))
                    && r.params["polarity"] == "normal"
            })
            .collect();
        assert_eq!(
            recs.len(),
            bands.len(),
            "one normal-polarity m2 record per band"
        );
        for (i, r) in recs.iter().enumerate() {
            assert_eq!(r.params["band_index"].as_u64().unwrap(), i as u64);
            assert_eq!(
                r.params["coherence"].as_f64().unwrap().to_bits(),
                oracle[i].to_bits(),
                "ref.band_gain_m2 params.coherence band {i}"
            );
        }
    }

    // ---- A7 stereo: `lr_band_coherence` tensor + record ----

    #[test]
    fn a7_lr_band_coherence_tensor_is_bit_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        let cfg = ReferenceConfig::default();
        let bands = ref_bands(rate);
        // Oracle: the reference.rs `src.cap.len() >= 2` accumulation.
        let m0 = stft(&cap[0], cfg.stft_window, cfg.stft_hop, rate);
        let m1 = stft(&cap[1], cfg.stft_window, cfg.stft_hop, rate);
        let fr = m0.frames().min(m1.frames());
        assert!(fr > 0, "fixture must yield capture frames");
        let bins = m0.fft_size;
        let half = bins / 2;
        let bin_hz = rate as f64 / bins as f64;
        let mut pa = vec![0.0f64; bins];
        let mut pb = vec![0.0f64; bins];
        let mut pab_re = vec![0.0f64; bins];
        let mut pab_im = vec![0.0f64; bins];
        for f in 0..fr {
            let (cr, ci) = csd(&m0.re[f], &m0.im[f], &m1.re[f], &m1.im[f]);
            for k in 0..bins {
                pa[k] += m0.re[f][k].powi(2) + m0.im[f][k].powi(2);
                pb[k] += m1.re[f][k].powi(2) + m1.im[f][k].powi(2);
                pab_re[k] += cr[k];
                pab_im[k] += ci[k];
            }
        }
        let (coh_re, coh_im) = complex_coherence(&pa, &pb, &pab_re, &pab_im);
        let expected_rows: Vec<Vec<f64>> = bands
            .iter()
            .map(|&(lo, hi)| {
                let b0 = (lo / bin_hz) as usize;
                let b1 = ((hi / bin_hz).ceil() as usize).min(half);
                if b1 <= b0 {
                    return vec![0.0, 0.0];
                }
                let (mut re, mut im) = (0.0f64, 0.0f64);
                for (&r, &i) in coh_re[b0..b1].iter().zip(&coh_im[b0..b1]) {
                    re += r;
                    im += i;
                }
                let cnt = (b1 - b0) as f64;
                vec![(re * re + im * im).sqrt() / cnt, im.atan2(re)]
            })
            .collect();
        let flat: Vec<f64> = expected_rows.iter().flatten().copied().collect();
        let t = tensor(&out, "intermediate/src__cap/lr_band_coherence");
        assert_eq!(t.shape, [bands.len() as u64, 2]);
        assert_bits(&t.data, &flat, "lr_band_coherence");
        // ref.lr_lowband_coherence carries band_rows[0].
        let r = record(&out, "ref.lr_lowband_coherence");
        assert_eq!(
            scalar(r).to_bits(),
            expected_rows[0][0].to_bits(),
            "ref.lr_lowband_coherence"
        );
        assert_eq!(
            r.params["phase_rad"].as_f64().unwrap().to_bits(),
            expected_rows[0][1].to_bits(),
            "ref.lr_lowband_coherence phase_rad"
        );
    }

    // ---- A16a: column-scan `mean_psd_*` → band_gain_psd_ratio
    //      tensor + records + psd_ratio tensor ----

    #[test]
    fn a16a_band_gain_psd_ratio_is_bit_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        let cfg = ReferenceConfig::default();
        let bands = ref_bands(rate);
        let ms = stft(&src[0], cfg.stft_window, cfg.stft_hop, rate);
        let mc = stft(&cap[0], cfg.stft_window, cfg.stft_hop, rate);
        let w = welch(&ms, &mc);
        let frames = ms.frames().min(mc.frames());
        let bins = ms.fft_size;
        let half = bins / 2;
        let bin_hz = rate as f64 / bins as f64;
        // Oracle: the current column-scan mean over `take(frames)`.
        let psd_s = ms.psd();
        let psd_c = mc.psd();
        let mean_psd_s: Vec<f64> = (0..bins)
            .map(|k| psd_s.iter().take(frames).map(|r| r[k]).sum::<f64>() / frames as f64)
            .collect();
        let mean_psd_c: Vec<f64> = (0..bins)
            .map(|k| psd_c.iter().take(frames).map(|r| r[k]).sum::<f64>() / frames as f64)
            .collect();
        let band_e_s = band_power(&mean_psd_s, rate, bins, &bands);
        let band_e_c = band_power(&mean_psd_c, rate, bins, &bands);
        let gains: Vec<f64> = band_e_s
            .iter()
            .zip(&band_e_c)
            .map(|(&s, &c)| if s > 0.0 { (c / s).sqrt() } else { f64::NAN })
            .collect();
        let band_coh: Vec<f64> = bands
            .iter()
            .map(|&(lo, hi)| {
                let b0 = (lo / bin_hz) as usize;
                let b1 = ((hi / bin_hz).ceil() as usize).min(half);
                if b1 <= b0 {
                    return 0.0;
                }
                w.coh[b0..b1]
                    .iter()
                    .map(|(r, i)| (r * r + i * i).sqrt())
                    .sum::<f64>()
                    / (b1 - b0) as f64
            })
            .collect();
        let centers: Vec<f64> = bands.iter().map(|b| (b.0 + b.1) / 2.0).collect();
        let bg = tensor(&out, "intermediate/src__cap/band_gain_psd_ratio");
        assert_eq!(bg.shape, [3, bands.len() as u64]);
        let nb = bands.len();
        assert_bits(&bg.data[..nb], &centers, "band_gain centers");
        assert_bits(&bg.data[nb..2 * nb], &gains, "band_gain gains");
        assert_bits(&bg.data[2 * nb..], &band_coh, "band_gain coherence");
        // Per-band records carry the same gain value.
        let recs: Vec<_> = out
            .records
            .iter()
            .filter(|r| r.feature_id == "ref.band_gain_psd_ratio")
            .collect();
        assert_eq!(recs.len(), bands.len());
        for (i, r) in recs.iter().enumerate() {
            assert_eq!(
                scalar(r).to_bits(),
                gains[i].to_bits(),
                "ref.band_gain_psd_ratio band {i}"
            );
            assert_eq!(
                r.params["band_coherence"].as_f64().unwrap().to_bits(),
                band_coh[i].to_bits(),
                "band_coherence param band {i}"
            );
        }
        // psd_ratio tensor: c/s or 0 on silence.
        let psd_ratio: Vec<f64> = mean_psd_s
            .iter()
            .zip(&mean_psd_c)
            .map(|(&s, &c)| if s > 0.0 { c / s } else { 0.0 })
            .collect();
        let pr = tensor(&out, "intermediate/src__cap/psd_ratio");
        assert_bits(&pr.data, &psd_ratio, "psd_ratio");
    }

    // ---- A16b: `dom()` peak → `ref.harmonic_freq_ratio` ----

    #[test]
    fn a16b_harmonic_freq_ratio_is_bit_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        let cfg = ReferenceConfig::default();
        let ms = stft(&src[0], cfg.stft_window, cfg.stft_hop, rate);
        let mc = stft(&cap[0], cfg.stft_window, cfg.stft_hop, rate);
        let bin_hz = rate as f64 / ms.fft_size as f64;
        // Oracle: the current `dom` closure — `magnitude()` then a
        // row-major mean, argmax skipping DC.
        let dom = |m: &StftMatrix| -> f64 {
            let mag = m.magnitude();
            let mut mean = vec![0.0; m.fft_size / 2];
            for row in &mag {
                for (k, v) in mean.iter_mut().enumerate() {
                    *v += row[k];
                }
            }
            mean.iter()
                .enumerate()
                .skip(1)
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(b, _)| b as f64 * bin_hz)
                .unwrap_or(0.0)
        };
        let (ds, dc) = (dom(&ms), dom(&mc));
        assert!(ds > 0.0 && dc > 0.0, "fixture must yield peaks");
        let r = record(&out, "ref.harmonic_freq_ratio");
        assert_eq!(
            scalar(r).to_bits(),
            (dc / ds).to_bits(),
            "ref.harmonic_freq_ratio"
        );
        assert_eq!(
            r.params["src_hz"].as_f64().unwrap().to_bits(),
            ds.to_bits(),
            "src_hz param"
        );
        assert_eq!(
            r.params["cap_hz"].as_f64().unwrap().to_bits(),
            dc.to_bits(),
            "cap_hz param"
        );
    }

    // ---- A15: residual interleave (frame-major nested loop) ----

    #[test]
    fn a15_residual_interleave_is_element_exact() {
        let (src, cap, rate) = pair_fixture();
        let out = analyze(&src, &cap, rate);
        let cfg = ReferenceConfig::default();
        let n = src[0].len();
        // The R0 residual is the capture as-is: its interleave is
        // fully reconstructible as `v[i*ch + c] = cap[c][i]`.
        let r0 = out
            .residual_audio
            .iter()
            .find(|r| r.name == "src__cap/r0")
            .expect("r0 residual audio");
        assert_eq!(r0.channels as usize, cap.len());
        assert_eq!(r0.samples.len(), n * cap.len());
        let expected: Vec<f64> = (0..n)
            .flat_map(|i| cap.iter().map(move |ch| ch[i]))
            .collect();
        assert_bits(&r0.samples, &expected, "r0 interleave");
        // Every stage must use the same frame-major layout:
        // deinterleaving must reproduce a channel-0 signal whose
        // STFT equals the persisted `stft_re` companion tensor.
        for res in &out.residual_audio {
            let chans = al_locate::deinterleave(&res.samples, res.channels);
            assert_eq!(chans.len(), res.channels as usize);
            let rm = stft(&chans[0], cfg.stft_window, cfg.stft_hop, rate);
            let flat_re: Vec<f64> = rm.re.iter().flatten().copied().collect();
            let t = tensor(&out, &format!("residual/{}/stft_re", res.name));
            assert_bits(&t.data, &flat_re, &format!("{} stft_re", res.name));
        }
    }
}
