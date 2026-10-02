//! Integration tests for the ingest stage: synthetic WAV fixtures through
//! the real runner, manifest inspection, resume, and explicit failures.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use al_core::{InputRole, StageKind, StageStatus};
use al_job::job::{JobDefinition, input};
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::runner::StageRunner;
use al_job::stage::{Stage, StageContext, StageError};
use al_media::{Dictionary, IngestStage};

/// Deterministic 16-bit stereo WAV bytes.
fn wav_bytes(channels: u16, rate: u32, frames: &[i16]) -> Vec<u8> {
    let block_align = channels * 2;
    let data_len = frames.len() as u32 * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in frames {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-ingest-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn bundled_dictionary() -> Dictionary {
    Dictionary::load(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../dictionaries/capture-artifacts.v1.json"),
    )
    .unwrap()
}

fn write_fixture(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

#[test]
fn ingest_wav_produces_pcm_and_metadata_artifacts() {
    let dir = temp_dir("wav");
    let samples: Vec<i16> = (0..4800).map(|i| (i % 200) as i16 * 100).collect();
    let wav = write_fixture(&dir, "cap.wav", &wav_bytes(2, 48000, &samples));

    let job_dir = temp_dir("job");
    let runner = StageRunner::new(&job_dir);
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &wav)],
        serde_json::json!({}),
        vec![StageKind::Ingest],
    );
    let mut stages: Vec<Box<dyn Stage>> =
        vec![Box::new(IngestStage::with_dictionary(bundled_dictionary()))];

    let manifest = runner.run(&def, &mut stages).unwrap();
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );

    let pcm = manifest
        .artifacts
        .iter()
        .find(|a| a.role == "decoded_pcm")
        .expect("decoded_pcm artifact");
    let pcm_path = runner.job_dir(&manifest.job_id).join(&pcm.path);
    let raw = std::fs::read(&pcm_path).unwrap();
    assert_eq!(raw.len(), samples.len() * 8); // f64 per sample

    let meta_record = manifest
        .artifacts
        .iter()
        .find(|a| a.role == "ingest_metadata")
        .expect("ingest_metadata artifact");
    let meta_path = runner.job_dir(&manifest.job_id).join(&meta_record.path);
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(meta_path).unwrap()).unwrap();
    assert_eq!(meta["audio"]["codec"], "pcm_s16le");
    assert_eq!(meta["audio"]["sample_rate"], 48000);
    assert_eq!(meta["audio"]["channels"], 2);
    assert_eq!(meta["audio"]["source_hash"].as_str().unwrap().len(), 64);
    // Dictionary provenance (version + content hash) is recorded.
    assert_eq!(meta["dictionary"]["sha256"].as_str().unwrap().len(), 64);
    // The dictionary annotates, never alters raw measurements.
    assert!(!meta["annotations"].as_array().unwrap().is_empty());

    // Produced artifacts trace lineage back to the declared input.
    assert!(!pcm.inputs.is_empty());
    let input_record = manifest.artifact(&pcm.inputs[0]).unwrap();
    assert_eq!(input_record.role, "declared_input");
    assert_eq!(input_record.produced_by, None);

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn same_input_reproduces_same_hash() {
    let dir = temp_dir("repro");
    let wav = write_fixture(&dir, "cap.wav", &wav_bytes(1, 44100, &[1, -1, 100, -100]));

    let job_dir = temp_dir("job2");
    let runner = StageRunner::new(&job_dir);
    let mut hashes = Vec::new();
    for _ in 0..2 {
        let def = JobDefinition::create(
            vec![input(InputRole::CaptureBug, &wav)],
            serde_json::json!({}),
            vec![StageKind::Ingest],
        );
        let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(IngestStage::new())];
        let manifest = runner.run(&def, &mut stages).unwrap();
        let meta = manifest
            .artifacts
            .iter()
            .find(|a| a.role == "ingest_metadata")
            .unwrap();
        hashes.push(meta.params["source_hash"].as_str().unwrap().to_owned());
    }
    assert_eq!(hashes[0], hashes[1]);

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn unknown_format_fails_explicitly() {
    let dir = temp_dir("unknown");
    let bogus = write_fixture(&dir, "capture.bin", b"\x00\x11\x22\x33garbage");

    let job_dir = temp_dir("job3");
    let runner = StageRunner::new(&job_dir);
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &bogus)],
        serde_json::json!({}),
        vec![StageKind::Ingest],
    );
    let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(IngestStage::new())];

    let err = runner.run(&def, &mut stages).unwrap_err();
    assert!(err.message.contains("decode/input unavailable"));

    // Failed stage must leave an explicit failed record, not a success.
    let manifest = Manifest::load(&runner.job_dir(&def.job_id)).unwrap();
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Failed
    );

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn resume_reuses_completed_ingest_artifacts() {
    let dir = temp_dir("resume");
    let wav = write_fixture(&dir, "cap.wav", &wav_bytes(1, 48000, &[0; 960]));

    // Second stage observes whether it ran.
    struct Probe {
        ran: Rc<Cell<usize>>,
    }
    impl Stage for Probe {
        fn kind(&self) -> StageKind {
            StageKind::Locate
        }
        fn depends_on(&self) -> Vec<StageKind> {
            vec![StageKind::Ingest]
        }
        fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
            self.ran.set(self.ran.get() + 1);
            Ok(vec![ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: "probe".into(),
                path: "intermediate/probe.json".into(),
                produced_by: Some(StageKind::Locate),
                inputs: vec![],
                params: serde_json::json!({}),
            }])
        }
    }

    let job_dir = temp_dir("job4");
    let runner = StageRunner::new(&job_dir);
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, &wav)],
        serde_json::json!({}),
        vec![StageKind::Ingest, StageKind::Locate],
    );
    let probe_ran = Rc::new(Cell::new(0));
    let mut stages: Vec<Box<dyn Stage>> = vec![
        Box::new(IngestStage::new()),
        Box::new(Probe {
            ran: probe_ran.clone(),
        }),
    ];

    runner.run(&def, &mut stages).unwrap();
    assert_eq!(probe_ran.get(), 1);
    let before = runner
        .job_dir(&def.job_id)
        .join("manifest.json")
        .metadata()
        .unwrap();

    // Resume after success must not re-run completed stages.
    runner.resume(&def, &mut stages).unwrap();
    assert_eq!(probe_ran.get(), 1);
    let after = Manifest::load(&runner.job_dir(&def.job_id)).unwrap();
    assert!(
        after
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed)
    );
    let _ = before;

    std::fs::remove_dir_all(&dir).ok();
    std::fs::remove_dir_all(&job_dir).ok();
}

#[test]
fn timing_observation_serializes_with_annotations_separately() {
    use al_media::{ClockKind, observe_timing};

    // VFR-like sequence: alternating short/long frame intervals + one gap.
    let mut pts = vec![0i64];
    for i in 0..40 {
        pts.push(pts[i] + if i % 2 == 0 { 1000 } else { 2000 });
    }
    for p in pts.iter_mut().skip(20) {
        *p += 20_000;
    }
    let obs = observe_timing(ClockKind::Video, 90_000, &pts, None);
    assert_eq!(obs.clock, ClockKind::Video);
    assert!(!obs.discontinuities.is_empty());

    // Observation and dictionary annotation stay in separate fields.
    let record = serde_json::json!({
        "observation": serde_json::to_value(&obs).unwrap(),
        "annotations": bundled_dictionary().annotate(Some("h264"), Some("mkv")),
    });
    assert!(
        !record["observation"]["discontinuities"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(record["annotations"].as_array().unwrap().len(), 1); // src_route wildcard
}
