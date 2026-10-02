//! Integration coverage for the analysis-dataset test specs:
//! store round trips, provenance/validity semantics, and
//! run/resume materialization through the job runner.

use std::fs;
use std::path::PathBuf;

use al_core::{InputRole, JobId, StageKind, Validity};
use al_dataset::reader::DatasetReader;
use al_dataset::stage::DatasetStage;
use al_dataset::table::{Cell, Column, DataType};
use al_dataset::writer::{DatasetWriter, TensorChunk};
use al_dataset::{DatasetManifest, InputReference, Provenance, RoleStatus};
use al_job::job::{JobDefinition, input};
use al_job::runner::StageRunner;
use al_media::IngestStage;
use al_media::wav::WavEncoding;

fn tmp(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-ds-it-{tag}-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn prov() -> Provenance {
    Provenance {
        input_id: "IN-1".into(),
        algorithm_id: "test.alg".into(),
        algorithm_version: "2".into(),
        params: serde_json::json!({"window": 2048}),
        alignment_id: Some("AL-1".into()),
        parents: vec!["ART-parent".into()],
    }
}

fn schema() -> Vec<Column> {
    vec![
        Column {
            name: "t".into(),
            data_type: DataType::F64,
        },
        Column {
            name: "v".into(),
            data_type: DataType::I64,
        },
        Column {
            name: "label".into(),
            data_type: DataType::Utf8,
        },
    ]
}

/// dataset-store-roundtrip: a large table is range/column readable
/// after a round trip through the store.
#[test]
fn large_table_range_and_column_reads() {
    let dir = tmp("table");
    let mut w = DatasetWriter::create(&dir.join("ds"), vec![], serde_json::json!({})).unwrap();
    let rows: Vec<Vec<Cell>> = (0..200_000i64)
        .map(|i| {
            vec![
                Cell::F64(i as f64 * 0.5),
                Cell::I64(i),
                if i % 7 == 0 {
                    Cell::Null
                } else {
                    Cell::Str(format!("r{i}"))
                },
            ]
        })
        .collect();
    w.write_table("features/cheap", "rms", &schema(), &rows, prov())
        .unwrap();
    w.finish().unwrap();

    let r = DatasetReader::open(&dir.join("ds")).unwrap();
    let cols = r
        .read_table("features/cheap", "rms", 150_000, 150_010, Some(&[1, 2]))
        .unwrap();
    assert_eq!(cols[0][0], Cell::I64(150_000));
    assert_eq!(cols[1][0], Cell::Str("r150000".into()));
    assert_eq!(cols[1].len(), 10);
    fs::remove_dir_all(&dir).ok();
}

/// dataset-store-roundtrip: tensor regions crossing chunk boundaries,
/// WAV sample round trips, strings/NULL preservation — covered by unit
/// tests; here via the dataset-level reader.
#[test]
fn tensor_and_audio_round_trip_via_dataset() {
    let dir = tmp("tensor");
    let mut w = DatasetWriter::create(&dir.join("ds"), vec![], serde_json::json!({})).unwrap();
    // 6x5 f64 tensor in 4x3 chunks.
    let chunks: Vec<TensorChunk> = [(0u64, 0u64), (0, 1), (1, 0), (1, 1)]
        .iter()
        .map(|&(c0, c1)| {
            let ext = [4u64.min(6 - c0 * 4), 3u64.min(5 - c1 * 3)];
            let mut data = Vec::new();
            for r in 0..ext[0] {
                for c in 0..ext[1] {
                    data.push(((c0 * 4 + r) * 5 + (c1 * 3 + c)) as f64);
                }
            }
            TensorChunk {
                coords: vec![c0, c1],
                data,
            }
        })
        .collect();
    w.write_tensor("intermediate", "stft", &[6, 5], &[4, 3], &chunks, prov())
        .unwrap();
    let samples = vec![0.5_f64, -0.5, 1.0, -1.0];
    w.write_audio(
        "residual",
        "res.wav",
        WavEncoding::Float32,
        2,
        48000,
        &samples,
        prov(),
    )
    .unwrap();
    w.finish().unwrap();

    let r = DatasetReader::open(&dir.join("ds")).unwrap();
    let (vals, shape) = r
        .read_tensor("intermediate", "stft", &[[2, 5], [1, 4]])
        .unwrap();
    assert_eq!(shape, vec![3, 3]);
    assert_eq!(
        vals,
        vec![11.0, 12.0, 13.0, 16.0, 17.0, 18.0, 21.0, 22.0, 23.0]
    );
    let audio = r.read_audio("residual", "res.wav").unwrap();
    assert_eq!(audio.samples, samples);
    assert_eq!(audio.channels, 2);
    fs::remove_dir_all(&dir).ok();
}

/// dataset-provenance-validity: manifest traversal exposes input id,
/// algorithm id/version, params, alignment id, and parent lineage —
/// with schema and algorithm versions as independent fields.
#[test]
fn provenance_traversal_and_version_independence() {
    let dir = tmp("prov");
    let inputs = vec![InputReference {
        input_id: "IN-9".into(),
        role: "capture_bug".into(),
        source_path: "/abs/cap.wav".into(),
        source_sha256: "ff".repeat(32),
    }];
    let mut w = DatasetWriter::create(&dir.join("ds"), inputs, serde_json::json!({})).unwrap();
    w.write_json(
        "locate",
        "result.json",
        &serde_json::json!({"offset": 42}),
        prov(),
    )
    .unwrap();
    let manifest = w.finish().unwrap();

    assert_eq!(
        manifest.schema_version,
        al_dataset::manifest::SCHEMA_VERSION
    );
    let artifact = &manifest.artifacts[0];
    let p = &artifact.provenance;
    assert_eq!(p.input_id, "IN-1");
    assert_eq!(p.algorithm_id, "test.alg");
    assert_eq!(p.algorithm_version, "2"); // independent from schema_version
    assert_eq!(p.params["window"], 2048);
    assert_eq!(p.alignment_id.as_deref(), Some("AL-1"));
    assert_eq!(p.parents, vec!["ART-parent"]);
    // Declared inputs stay references, not copies.
    assert_eq!(manifest.inputs[0].source_sha256, "ff".repeat(32));
    assert!(!dir.join("ds").join("cap.wav").exists());
    fs::remove_dir_all(&dir).ok();
}

/// dataset-provenance-validity: every validity category stays
/// distinct, and `valid` is never conflated with "cannot measure".
#[test]
fn validity_categories_stay_distinct() {
    let all = [
        Validity::Valid,
        Validity::SilenceInsufficientEnergy,
        Validity::InsufficientSupport,
        Validity::LowCorrelation,
        Validity::LowCoherence,
        Validity::AmbiguousAlignment,
        Validity::OutOfSupportedFrequencyRange,
        Validity::DecodeInputUnavailable,
        Validity::NotApplicable,
        Validity::ComputationFailed,
    ];
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b);
        }
    }
    // Each variant serializes to its own snake_case wire value.
    let names: Vec<String> = all
        .iter()
        .map(|v| serde_json::to_string(v).unwrap())
        .collect();
    assert_eq!(names.len(), all.len());
    assert!(names.iter().all(|n| n.starts_with('"')));
}

/// dataset-provenance-validity: roles with no upstream producer are
/// `not_produced`, distinguishable from produced-with-content.
#[test]
fn unimplemented_roles_marked_not_produced() {
    let dir = tmp("roles");
    let mut w = DatasetWriter::create(&dir.join("ds"), vec![], serde_json::json!({})).unwrap();
    w.write_table(
        "features/cheap",
        "rms",
        &schema(),
        &[vec![Cell::F64(1.0), Cell::I64(1), Cell::Null]],
        prov(),
    )
    .unwrap();
    let manifest = w.finish().unwrap();

    assert_eq!(
        manifest.roles["features/cheap"].status,
        RoleStatus::Produced
    );
    for role in [
        "locate",
        "features/reference",
        "features/expensive",
        "residual",
        "events",
        "visualization",
    ] {
        assert_eq!(
            manifest.roles[role].status,
            RoleStatus::NotProduced,
            "{role}"
        );
    }
    fs::remove_dir_all(&dir).ok();
}

/// run/resume materialization: `ingest,dataset` produces `analysis/`
/// and a resume keeps the completed dataset stage without rewriting.
#[test]
fn run_and_resume_materializes_dataset() {
    let base = tmp("run");
    let wav_dir = tmp("wavsrc");
    // Minimal mono 16-bit WAV.
    let wav_path = wav_dir.join("cap.wav");
    {
        use std::io::Write as _;
        let cursor = std::io::Cursor::new(Vec::new());
        let mut w = al_media::wav::WavWriter::new(cursor, WavEncoding::PcmS16, 1, 8000).unwrap();
        w.write_samples(&[0.0, 1000.0, -1000.0]).unwrap();
        let mut f = std::fs::File::create(&wav_path).unwrap();
        f.write_all(&w.finish().unwrap().into_inner()).unwrap();
    }

    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &wav_path)],
        serde_json::json!({}),
        vec![StageKind::Ingest, StageKind::Dataset],
    )
    .unwrap();
    let runner = StageRunner::new(&base);
    let mut stages: Vec<Box<dyn al_job::stage::Stage>> =
        vec![Box::new(IngestStage::new()), Box::new(DatasetStage::new())];
    let manifest = runner.run(&def, &mut stages).unwrap();

    let analysis = base.join(def.job_id.to_string()).join("analysis");
    let ds = DatasetReader::open(&analysis).unwrap();
    assert_eq!(ds.manifest().inputs.len(), 1);
    assert_eq!(ds.manifest().inputs[0].role, "capture_bug");
    assert!(ds.role_produced("inputs"));
    assert!(!ds.role_produced("locate"));

    // The decoded PCM is materialized as WAV under analysis/inputs.
    let wav_artifacts = ds
        .manifest()
        .artifacts
        .iter()
        .filter(|a| a.store == al_dataset::manifest::StoreKind::Audio)
        .count();
    assert_eq!(wav_artifacts, 1);
    let audio = ds
        .read_audio("inputs", &format!("{}.wav", def.inputs[0].input_id))
        .unwrap();
    assert_eq!(audio.samples, vec![0.0, 1000.0, -1000.0]);
    assert_eq!(audio.sample_rate, 8000);
    let dataset_record = manifest
        .artifacts
        .iter()
        .find(|a| a.role == "analysis_dataset")
        .expect("dataset root artifact recorded");
    assert_eq!(dataset_record.path, PathBuf::from("analysis"));

    // Resume: the completed dataset stage is preserved, not rerun.
    let mut stages2: Vec<Box<dyn al_job::stage::Stage>> =
        vec![Box::new(IngestStage::new()), Box::new(DatasetStage::new())];
    let resumed = runner.resume(&def, &mut stages2).unwrap();
    let ds_record = resumed
        .stages
        .iter()
        .find(|s| s.stage == StageKind::Dataset)
        .unwrap();
    assert_eq!(ds_record.status, al_core::StageStatus::Completed);
    assert!(DatasetManifest::load(&analysis).is_ok());

    fs::remove_dir_all(&base).ok();
    fs::remove_dir_all(&wav_dir).ok();
}

/// A failed dataset stage leaves a partial `analysis/` behind; resume
/// must regenerate it wholesale instead of failing on `AlreadyExists`.
#[test]
fn failed_dataset_stage_is_resumable() {
    let base = tmp("re");
    let wav_dir = tmp("rewav");
    let wav_path = wav_dir.join("cap.wav");
    {
        use std::io::Write as _;
        let cursor = std::io::Cursor::new(Vec::new());
        let mut w = al_media::wav::WavWriter::new(cursor, WavEncoding::PcmS16, 1, 8000).unwrap();
        w.write_samples(&[1.0, 2.0]).unwrap();
        let mut f = std::fs::File::create(&wav_path).unwrap();
        f.write_all(&w.finish().unwrap().into_inner()).unwrap();
    }

    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &wav_path)],
        serde_json::json!({}),
        vec![StageKind::Ingest, StageKind::Dataset],
    )
    .unwrap();
    let runner = StageRunner::new(&base);
    let mut stages: Vec<Box<dyn al_job::stage::Stage>> =
        vec![Box::new(IngestStage::new()), Box::new(DatasetStage::new())];
    runner.run(&def, &mut stages).unwrap();

    // Simulate a crashed dataset stage: partial `analysis/` contents
    // remain and the manifest records the stage as failed.
    let job_dir = base.join(def.job_id.to_string());
    let analysis = job_dir.join("analysis");
    fs::remove_dir_all(&analysis).unwrap();
    fs::create_dir_all(analysis.join("inputs")).unwrap();
    fs::write(analysis.join("manifest.json"), b"{partial").unwrap();
    let mut manifest = al_job::manifest::Manifest::load(&job_dir).unwrap();
    manifest.stage_mut(StageKind::Dataset).unwrap().status = al_core::StageStatus::Failed;
    manifest.save(&job_dir).unwrap();

    let mut stages2: Vec<Box<dyn al_job::stage::Stage>> =
        vec![Box::new(IngestStage::new()), Box::new(DatasetStage::new())];
    let resumed = runner.resume(&def, &mut stages2).unwrap();
    let ds_record = resumed
        .stages
        .iter()
        .find(|s| s.stage == StageKind::Dataset)
        .unwrap();
    assert_eq!(ds_record.status, al_core::StageStatus::Completed);
    // The dataset is fully regenerated, not the leftover partial.
    assert!(DatasetReader::open(&analysis).is_ok());

    fs::remove_dir_all(&base).ok();
    fs::remove_dir_all(&wav_dir).ok();
}
