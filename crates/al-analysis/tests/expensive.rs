//! ExpensiveAnalysisStage integration tests.
//!
//! Runs the real `CheapAnalysisStage` first so the candidate
//! extraction reads actual persisted features, then exercises every
//! trigger mode and the non-destructive-output contract.

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_job::job::input;
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};

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
    input_id: String,
}

/// One `capture_bug` input with mono 48 kHz decoded PCM.
fn fixture(capture: &[f64]) -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-exp-test-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut cap_input = input(InputRole::CaptureBug, dir.join("cap.bin"));
    cap_input.source_sha256 = Some("cc".repeat(32));
    let input_id = cap_input.input_id.to_string();
    let declared_id = cap_input.input_id.clone();
    let mut manifest = Manifest::new(
        JobId::new(),
        vec![cap_input],
        serde_json::json!({}),
        &[
            StageKind::Ingest,
            StageKind::CheapAnalysis,
            StageKind::ExpensiveAnalysis,
        ],
    );

    write_pcm(&dir, "inputs/cap.f64le", capture);
    let declared = manifest.input_artifact(&declared_id).unwrap();
    manifest.register_artifact(ArtifactRecord {
        artifact_id: al_core::ArtifactId::new(),
        role: "decoded_pcm".into(),
        path: PathBuf::from("inputs/cap.f64le"),
        produced_by: Some(StageKind::Ingest),
        inputs: vec![declared],
        params: serde_json::json!({"sample_rate": 48_000, "channels": 1}),
    });

    Fixture {
        dir,
        manifest,
        input_id,
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

/// Cheap feature JSON for the input (byte content, for
/// non-destructive comparison).
fn cheap_json(fx: &Fixture) -> String {
    fs::read_to_string(
        fx.dir
            .join("analysis")
            .join("features")
            .join("cheap")
            .join(format!("{}.json", fx.input_id)),
    )
    .expect("cheap features persisted")
}

/// Kind strings (`Str` cells) of every event row.
fn event_kinds(fx: &Fixture) -> Vec<String> {
    let reader =
        al_dataset::reader::DatasetReader::open(&fx.dir.join("analysis")).expect("dataset opens");
    let table = reader
        .open_table("events", "candidates")
        .expect("events table");
    let n = table.row_count();
    if n == 0 {
        return Vec::new();
    }
    let cols = table.read_range(0, n, Some(&[1])).expect("kind column");
    cols[0]
        .iter()
        .filter_map(|c| match c {
            al_dataset::table::Cell::Str(s) => Some(s.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn clipping_candidate_triggers_expensive_without_overwriting_cheap() {
    // Hard-clipped sine: >99% of samples sit at |1.0|.
    let mut cap = noise(24_000, 7);
    for v in &mut cap {
        *v = v.signum();
    }
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }
    let cheap_before = cheap_json(&fx);

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    // Candidate event persisted under the events role.
    let kinds = event_kinds(&fx);
    assert!(kinds.iter().any(|k| k == "clipping"), "events: {kinds:?}");
    // Expensive region was produced and records its trigger.
    assert!(
        recs.iter().any(|r| r.role == "expensive_features"),
        "artifacts: {recs:?}"
    );
    let exp_path = fx.dir.join("analysis").join("features").join("expensive");
    assert!(fs::read_dir(&exp_path).unwrap().next().is_some());
    // Cheap rows were not overwritten.
    assert_eq!(cheap_json(&fx), cheap_before);

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn suppressed_triggers_write_events_but_no_expensive() {
    // A quiet tonal sine: its only threshold hit is the noise-floor
    // heuristic (-23 dB > -40), which the config overlay raises out of
    // range — exercising the suppress path.
    let cap: Vec<f64> = (0..16_000)
        .map(|i| 0.1 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48_000.0).sin())
        .collect();
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {"events": {"noise_floor_dbfs": -10.0}}
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    // The events table exists (role produced) even with no hits;
    // expensive_features are suppressed.
    let _ = event_kinds(&fx);
    assert!(
        !recs.iter().any(|r| r.role == "expensive_features"),
        "unexpected expensive artifacts: {recs:?}"
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn manual_ranges_analyze_without_any_candidate() {
    // No candidates expected from a quiet sine — the manual range
    // alone must select the region and stamp `manual` as trigger.
    let cap: Vec<f64> = (0..24_000)
        .map(|i| 0.1 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48_000.0).sin())
        .collect();
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {
            "events": {"noise_floor_dbfs": -10.0},
            "expensive": {"manual_ranges": [[0.0, 0.25]]}
        }
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    let exp: Vec<_> = recs
        .iter()
        .filter(|r| r.role == "expensive_features")
        .collect();
    assert_eq!(exp.len(), 1, "artifacts: {recs:?}");
    assert!(
        exp[0].params["triggers"]
            .as_array()
            .is_some_and(|t| t.iter().any(|v| v == "manual")),
        "triggers: {}",
        exp[0].params
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn non_valid_candidate_region_records_invalid_observations() {
    // Digital silence: `level.integrated_dbfs` (-240 dBFS) trips the
    // silence rule with `SilenceInsufficientEnergy` — the only
    // candidate is non-`Valid`, so the region must persist invalid
    // observations (null scalars) rather than computed numbers. The
    // zero/stuck-run and noise-floor rules are raised out of range
    // so no `Valid` trigger fires alongside.
    let cap = vec![0.0f64; 24_000];
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {"events": {
            "zero_run_samples": 1e9,
            "stuck_run_samples": 1e9,
            "noise_floor_dbfs": 1e9
        }}
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    let exp = recs
        .iter()
        .find(|r| r.role == "expensive_features")
        .expect("non-valid candidate still produces a region");
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&exp.path)).expect("expensive json"))
            .unwrap();
    let records = body["records"].as_array().unwrap();
    assert!(!records.is_empty());
    for r in records {
        assert_ne!(
            r["validity"].as_str().unwrap(),
            "valid",
            "{} claimed Valid",
            r["feature_id"]
        );
        assert!(
            r["value"]["scalar"].is_null(),
            "{} emitted a finite number: {}",
            r["feature_id"],
            r["value"]
        );
        assert!(r["invalid_reason"].is_string());
    }

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn always_run_analyzes_full_input_without_reference_stage() {
    // No locate/reference stages ran: cheap-only candidates must
    // still drive the stage.
    let cap = noise(20_000, 13);
    let mut fx = fixture(&cap);
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({
        "analysis": {"expensive": {"always_run": true}}
    });

    let cheap_recs = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("cheap stage");
    for rec in cheap_recs {
        fx.manifest.register_artifact(rec);
    }

    let recs = al_analysis::stage::ExpensiveAnalysisStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("expensive stage");

    let exp = recs
        .iter()
        .find(|r| r.role == "expensive_features")
        .expect("always_run produced expensive output");
    assert!(
        exp.params["triggers"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("always_run"))
    );
    // Full-input region: a broad set of family records exists.
    let name = exp.path.file_stem().unwrap().to_str().unwrap().to_string();
    let body: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(fx.dir.join(&exp.path)).expect("expensive json"))
            .unwrap();
    let _ = name;
    let ids: Vec<&str> = body["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|r| r["feature_id"].as_str())
        .collect();
    for family in [
        "exp.cutoff_hz",
        "exp.echo_delay_s",
        "exp.src_image_score",
        "exp.polarity_asymmetry",
        "exp.repeat_block_score",
        "exp.spectral_hole_ratio",
        "exp.tonal_peak_count",
    ] {
        assert!(ids.contains(&family), "missing {family} in {ids:?}");
    }

    fs::remove_dir_all(&fx.dir).ok();
}
