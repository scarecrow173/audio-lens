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
    let source = noise(4_096, 7);
    let mut capture = noise(20_000, 11);
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
    for a in tensors {
        assert_eq!(a.provenance.algorithm_id, "reference.v1", "{}", a.path);
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

    // Hand-written rejected locate result at the canonical path.
    let rel = PathBuf::from("locate")
        .join(&fx.source_input_id)
        .join(&fx.capture_input_id)
        .join("result.json");
    fs::create_dir_all(fx.dir.join(rel.parent().unwrap())).unwrap();
    fs::write(
        fx.dir.join(&rel),
        serde_json::to_string(&serde_json::json!({
            "result": {
                "alignment_id": "aln-rejected",
                "source_input_id": fx.source_input_id,
                "capture_input_id": fx.capture_input_id,
                "source_range": {"start": 0, "end": 0},
                "capture_range": {"start": 0, "end": 0},
                "evidence": [],
                "agreeing_methods": 0,
                "position_spread": 0,
                "confidence": "low",
                "validity": "ambiguous_alignment",
                "reject_reason": "no consensus",
                "accepted": false,
                "algorithm_id": "locate.v1",
                "algorithm_version": "1",
                "params": {},
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fx.manifest.register_artifact(ArtifactRecord {
        artifact_id: al_core::ArtifactId::new(),
        role: "locate_result".into(),
        path: rel,
        produced_by: Some(StageKind::Locate),
        inputs: vec![],
        params: serde_json::json!({}),
    });

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
            Some("aln-rejected"),
            "record {} must reference the issued alignment",
            r["feature_id"]
        );
    }

    fs::remove_dir_all(&fx.dir).ok();
}
