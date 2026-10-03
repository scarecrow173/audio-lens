//! VisualizationStage / ExportStage integration tests.
//!
//! A small dataset is written directly through `DatasetWriter`, then
//! the real stages run against it: derived products, skipped-product
//! bookkeeping, determinism, and package portability are asserted.

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{Confidence, InputRole, JobId, StageKind, Validity};
use al_dataset::provenance::{self, Provenance};
use al_dataset::tensor::TensorReader;
use al_dataset::writer::{DatasetWriter, TensorChunk};
use al_job::job::input;
use al_job::manifest::Manifest;
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};

fn prov(input_id: &str) -> Provenance {
    Provenance {
        input_id: input_id.into(),
        algorithm_id: "test.v1".into(),
        algorithm_version: "1".into(),
        params: serde_json::json!({}),
        alignment_id: None,
        parents: vec![],
    }
}

struct Fixture {
    dir: PathBuf,
    manifest: Manifest,
    input_id: String,
}

/// One `capture_bug` input; the job manifest only lists the stages
/// the fixture exercises.
fn fixture() -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-viz-test-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut cap = input(InputRole::CaptureBug, dir.join("cap.wav"));
    cap.source_sha256 = Some("cc".repeat(32));
    let input_id = cap.input_id.to_string();
    let manifest = Manifest::new(
        JobId::new(),
        vec![cap],
        serde_json::json!({}),
        &[
            StageKind::Ingest,
            StageKind::Dataset,
            StageKind::Visualization,
            StageKind::Export,
        ],
    );
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

fn analysis_root(fx: &Fixture) -> PathBuf {
    fx.dir.join("analysis")
}

fn open_writer(fx: &Fixture, config: &serde_json::Value) -> DatasetWriter {
    DatasetWriter::open_or_create(
        &analysis_root(fx),
        provenance::input_references(&fx.manifest).unwrap(),
        config.clone(),
    )
    .expect("dataset writer")
}

/// A dataset with inputs audio, cheap features, and an intermediate
/// magnitude tensor — the roles every core product reads.
fn seed_dataset(fx: &Fixture, config: &serde_json::Value) {
    let mut w = open_writer(fx, config);
    // 48 kHz mono, 8192 samples: enough for several LOD buckets.
    let samples: Vec<f64> = (0..8192)
        .map(|i| 0.5 * (2.0 * std::f64::consts::PI * 1000.0 * i as f64 / 48_000.0).sin())
        .collect();
    w.write_audio(
        "inputs",
        &fx.input_id,
        al_media::wav::WavEncoding::Float32,
        1,
        48_000,
        &samples,
        prov(&fx.input_id),
    )
    .unwrap();
    w.write_json(
        "features/cheap",
        &fx.input_id,
        &serde_json::json!({"records": [
            {
                "feature_id": "level.rms_dbfs",
                "value": {"scalar": -8.0},
                "unit": "dbfs",
                "time_range": null,
                "freq_range": null,
                "channels": null,
                "resolution": null,
                "resolution_class": "segment",
                "target": -8.0,
                "reference": null,
                "delta": null,
                "confidence": "high",
                "validity": "valid",
                "invalid_reason": null,
                "support_count": 8192,
                "algorithm_id": "cheap.v1",
                "algorithm_version": "1",
                "params": {},
                "input_id": fx.input_id,
                "alignment_id": null,
                "lineage": []
            },
            {
                "feature_id": "stereo.mid_dbfs",
                "value": {"scalar": -10.0},
                "unit": "dbfs",
                "time_range": null,
                "freq_range": null,
                "channels": {"channel_pair": [0, 1]},
                "resolution": null,
                "resolution_class": "segment",
                "target": -10.0,
                "reference": null,
                "delta": null,
                "confidence": "medium",
                "validity": "valid",
                "invalid_reason": null,
                "support_count": 8192,
                "algorithm_id": "cheap.v1",
                "algorithm_version": "1",
                "params": {},
                "input_id": fx.input_id,
                "alignment_id": null,
                "lineage": []
            }
        ]}),
        prov(&fx.input_id),
    )
    .unwrap();
    // [4 frames, 8 bins] magnitude spectrogram.
    let mag: Vec<f64> = (0..32).map(|i| 0.01 * (i + 1) as f64).collect();
    w.write_tensor(
        "intermediate",
        &format!("{}/ch0/magnitude", fx.input_id),
        &[4, 8],
        &[4, 8],
        &[TensorChunk {
            coords: vec![0, 0],
            data: mag,
        }],
        prov(&fx.input_id),
    )
    .unwrap();
    // One candidate event so the events role + view are exercised.
    let event = al_analysis::events::CandidateEvent {
        event_id: "EVT-events.v1-x-0ms-abc".into(),
        kind: al_analysis::events::EventKind::Clipping,
        input_id: fx.input_id.clone(),
        start_s: 0.0,
        end_s: Some(0.17),
        strength: 2.0,
        confidence: Confidence::High,
        validity: Validity::Valid,
        source_feature: "dig.clipping_rate".into(),
        frequency_range: None,
        channels: vec![0],
        algorithm_id: "events.v1".into(),
        algorithm_version: "1".into(),
        evidence: vec![al_analysis::events::Evidence::Rows {
            artifact: format!("features/cheap/{}.json", fx.input_id),
            rows: vec![0],
        }],
        params: serde_json::json!({}),
    };
    w.write_table(
        "events",
        "candidates",
        &al_analysis::events::events_schema(),
        &al_analysis::events::events_to_rows(&[event]),
        prov("job"),
    )
    .unwrap();
    w.finish().unwrap();
}

/// Sorted (path, sha256) pairs for every file under `root`.
fn snapshot(root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let p = entry.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else {
                let rel = p
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, al_core::sha256_hex(&fs::read(&p).unwrap())));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn visualization_produces_timeline_lod_tiles_views_and_summary() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    let recs = al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");
    assert_eq!(recs.len(), 1);

    let viz = analysis_root(&fx).join("visualization");
    for path in [
        "timeline.json",
        "skipped.json",
        "summary.json",
        "views/events.json",
    ] {
        assert!(viz.join(path).exists(), "missing visualization/{path}");
    }
    // Waveform LOD tensors exist per level for channel 0.
    for level in 0..5 {
        assert!(
            viz.join(format!("waveform/{}/ch0/l{level}", fx.input_id))
                .is_dir(),
            "missing waveform LOD level {level}"
        );
    }
    // Spectrogram tile derived from the magnitude tensor.
    assert!(
        viz.join(format!("spectrogram/{}/ch0/magnitude", fx.input_id))
            .is_dir(),
        "missing spectrogram tile"
    );

    // Skipped bookkeeping covers roles that were not produced.
    let skipped: serde_json::Value =
        serde_json::from_slice(&fs::read(viz.join("skipped.json")).unwrap()).unwrap();
    let products: Vec<String> = skipped["skipped_products"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["product"].as_str().unwrap().to_string())
        .collect();
    assert!(
        products.contains(&"locate_view".to_string()),
        "{products:?}"
    );
    assert!(
        products.contains(&"residual_view".to_string()),
        "{products:?}"
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn visualization_rerun_replaces_products_deterministically() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    let mut stage = al_viz::stage::VisualizationStage::new();
    stage.run(&ctx(&fx, &registry, &config)).expect("first run");
    let first = snapshot(&analysis_root(&fx));
    stage
        .run(&ctx(&fx, &registry, &config))
        .expect("second run");
    let second = snapshot(&analysis_root(&fx));

    assert_eq!(first, second, "visualization output is not deterministic");

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn produced_role_without_expected_store_fails_explicitly() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    // Corrupt the contract: mark `locate` produced with a tensor
    // store instead of the expected locate-result JSON documents.
    let mut w = open_writer(&fx, &config);
    w.write_tensor(
        "locate",
        "bogus",
        &[2],
        &[2],
        &[TensorChunk {
            coords: vec![0],
            data: vec![1.0, 2.0],
        }],
        prov("job"),
    )
    .unwrap();
    w.finish().unwrap();

    let err = al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect_err("produced-but-wrong-store must fail");
    assert!(
        err.to_string().contains("locate"),
        "unexpected error: {err}"
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_package_is_portable_deterministic_and_manifested() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);
    al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");

    let recs = al_viz::stage::ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export stage");
    assert!(
        recs.iter().any(|r| r.role == "export_manifest")
            && recs.iter().any(|r| r.role == "export_golden"),
        "artifacts: {recs:?}"
    );

    let pkg = fx.dir.join("export").join("analysis");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pkg.join("export_manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    // Inputs are file-name + hash references, never absolute paths.
    let src = manifest["inputs"][0]["file_name"].as_str().unwrap();
    assert_eq!(src, "cap.wav");
    assert!(manifest["inputs"][0]["sha256"].is_string());
    // No absolute path survives in any text file (the stage also
    // enforces this internally; assert the observable result).
    let abs = fx.dir.to_string_lossy().replace('\\', "/");
    for (rel, _) in snapshot(&pkg) {
        if rel.ends_with(".json") {
            let body = fs::read_to_string(pkg.join(&rel)).unwrap();
            assert!(!body.contains(&abs), "{rel} embeds an absolute path");
        }
    }

    // Golden snapshot exists and a rebuild reproduces it byte-for-byte.
    let golden1 = fs::read_to_string(pkg.join("golden/snapshot.json")).unwrap();
    al_viz::stage::ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export rerun");
    let golden2 = fs::read_to_string(pkg.join("golden/snapshot.json")).unwrap();
    assert_eq!(golden1, golden2, "export is not deterministic");

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_name_rejects_unsafe_directory_names() {
    let fx = fixture();
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    // Sentinel under the job dir: `../keep` would reach it through
    // the unvalidated join + remove_dir_all.
    let sentinel = fx.dir.join("keep").join("marker.txt");
    fs::create_dir_all(sentinel.parent().unwrap()).unwrap();
    fs::write(&sentinel, b"keep").unwrap();

    let mut names = vec!["../keep", "..", ".", "a/b", ""];
    #[cfg(windows)]
    names.extend(["a\\b", "C:/keep", "C:\\keep", "\\\\server\\share"]);
    #[cfg(not(windows))]
    names.push("/keep");
    for name in names {
        let cfg = al_viz::package::ExportConfig {
            copy_media: false,
            name: name.into(),
        };
        let err = al_viz::package::build(&fx.dir, &fx.manifest, &cfg)
            .expect_err(&format!("export name {name:?} must be rejected"));
        assert_eq!(
            err.kind(),
            std::io::ErrorKind::InvalidData,
            "export name {name:?} produced wrong error kind"
        );
    }
    assert!(sentinel.exists(), "traversal deleted the sentinel file");

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn export_shrinks_absolute_paths_inside_config() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let dict = fx.dir.join("cfg").join("dict.bin");
    let config = serde_json::json!({
        "analysis": {"dict": dict.to_string_lossy()},
        "extra": {"refs": ["plain", dict.to_string_lossy()]},
    });
    seed_dataset(&fx, &config);
    al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");
    al_viz::stage::ExportStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("export stage");

    let pkg_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(fx.dir.join("export/analysis/analysis/manifest.json")).unwrap(),
    )
    .unwrap();
    // Absolute config paths travel as file-name references; the
    // original host path must not survive anywhere in config.
    assert_eq!(pkg_manifest["config"]["analysis"]["dict"], "dict.bin");
    assert_eq!(pkg_manifest["config"]["extra"]["refs"][1], "dict.bin");
    assert_eq!(pkg_manifest["config"]["extra"]["refs"][0], "plain");
    let body = fs::read_to_string(fx.dir.join("export/analysis/analysis/manifest.json")).unwrap();
    let dir_fwd = fx.dir.to_string_lossy().replace('\\', "/");
    let dir_back = fx.dir.to_string_lossy().replace('/', "\\");
    assert!(!body.contains(&dir_fwd) && !body.contains(&dir_back));

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn validity_mask_marks_invalid_frames_and_covers_whole_spectrogram() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);
    // Replace the cheap records with one invalid record covering
    // 0.0..0.03 s — frames 0..2 at the default 1024-sample hop —
    // while the seeded spectrogram has 4 frames.
    let mut w = open_writer(&fx, &config);
    w.write_json(
        "features/cheap",
        &fx.input_id,
        &serde_json::json!({"records": [{
            "feature_id": "level.rms_dbfs",
            "value": {"scalar": null},
            "unit": "dbfs",
            "time_range": {"start": 0.0, "end": 0.03},
            "freq_range": null,
            "channels": null,
            "resolution": null,
            "resolution_class": "mid20ms",
            "target": null,
            "reference": null,
            "delta": null,
            "confidence": "low",
            "validity": "low_coherence",
            "invalid_reason": "test",
            "support_count": 0,
            "algorithm_id": "cheap.v1",
            "algorithm_version": "1",
            "params": {},
            "input_id": fx.input_id,
            "alignment_id": null,
            "lineage": []
        }]}),
        prov(&fx.input_id),
    )
    .unwrap();
    w.finish().unwrap();

    al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");

    let mask_dir =
        analysis_root(&fx).join(format!("visualization/spectrogram/{}/mask", fx.input_id));
    let mask = TensorReader::open(&mask_dir).expect("validity mask tensor");
    assert_eq!(
        mask.meta().shape,
        vec![4],
        "mask must cover the spectrogram's full frame count"
    );
    let (data, _) = mask.read_range(&[[0, 4]]).unwrap();
    assert_eq!(data, vec![0.0, 0.0, 1.0, 1.0]);

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn timeline_declares_event_series_in_seconds() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");

    let tl: serde_json::Value = serde_json::from_slice(
        &fs::read(analysis_root(&fx).join("visualization/timeline.json")).unwrap(),
    )
    .unwrap();
    // Markers carry start_s/end_s in seconds; the shared timeline
    // must not declare them as sample positions.
    assert_eq!(tl["series"]["events"]["unit"], "seconds");

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn visualization_provenance_parents_trace_to_source_artifacts() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(analysis_root(&fx).join("manifest.json")).unwrap())
            .unwrap();
    let artifacts = manifest["artifacts"].as_array().unwrap();
    let find = |path: &str| -> &serde_json::Value {
        artifacts
            .iter()
            .find(|a| a["path"].as_str() == Some(path))
            .unwrap_or_else(|| panic!("missing artifact {path}"))
    };
    let parents = |path: &str| find(path)["provenance"]["parents"].clone();

    let audio = format!("inputs/{}.wav", fx.input_id);
    assert_eq!(
        parents("visualization/timeline.json"),
        serde_json::json!([audio])
    );
    assert_eq!(
        parents(&format!("visualization/waveform/{}/ch0/l0", fx.input_id)),
        serde_json::json!([audio])
    );
    assert_eq!(
        parents(&format!(
            "visualization/spectrogram/{}/ch0/magnitude",
            fx.input_id
        )),
        serde_json::json!([format!("intermediate/{}/ch0/magnitude", fx.input_id)])
    );
    assert_eq!(
        parents("visualization/views/events.json"),
        serde_json::json!(["events/candidates"])
    );
    assert!(
        !parents("visualization/summary.json")
            .as_array()
            .unwrap()
            .is_empty(),
        "summary.json must record its product parents"
    );

    fs::remove_dir_all(&fx.dir).ok();
}

#[test]
fn tile_tensors_are_chunked_and_range_readable() {
    let fx = fixture();
    let registry = ArtifactRegistry::new(&fx.dir);
    let config = serde_json::json!({});
    seed_dataset(&fx, &config);

    // A second magnitude tensor large enough to force a multi-chunk
    // tile (1024x256 > the ~1 MiB element cap).
    let (rows, bins) = (1024u64, 256u64);
    let mut w = open_writer(&fx, &config);
    let data: Vec<f64> = (0..rows * bins).map(|i| 0.01 + i as f64 * 1e-6).collect();
    w.write_tensor(
        "intermediate",
        &format!("{}/ch1/magnitude", fx.input_id),
        &[rows, bins],
        &[rows, bins],
        &[TensorChunk {
            coords: vec![0, 0],
            data,
        }],
        prov(&fx.input_id),
    )
    .unwrap();
    w.finish().unwrap();

    al_viz::stage::VisualizationStage::new()
        .run(&ctx(&fx, &registry, &config))
        .expect("visualization stage");

    let tile_dir = analysis_root(&fx).join(format!(
        "visualization/spectrogram/{}/ch1/magnitude",
        fx.input_id
    ));
    let tile = TensorReader::open(&tile_dir).expect("spectrogram tile");
    assert_eq!(tile.meta().shape, vec![rows, bins]);
    assert_eq!(
        tile.meta().chunk_shape,
        vec![512, bins],
        "time dim must split into size-capped chunks"
    );
    let chunk_files = fs::read_dir(tile_dir.join("chunks")).unwrap().count();
    assert_eq!(chunk_files, 2, "expected two chunk files");

    // A range read across the chunk boundary returns dB-mapped data.
    let (vals, region) = tile.read_range(&[[510, 514], [0, 64]]).unwrap();
    assert_eq!(region, vec![4, 64]);
    let expect = 20.0 * (0.01 + (510 * bins) as f64 * 1e-6).log10();
    assert!(
        (vals[0] - expect).abs() < 1e-9,
        "expected {expect}, got {}",
        vals[0]
    );

    fs::remove_dir_all(&fx.dir).ok();
}
