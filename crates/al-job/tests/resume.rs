use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

use al_core::{InputRole, StageKind, StageStatus};
use al_job::job::{JobDefinition, input};
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::runner::StageRunner;
use al_job::stage::{Stage, StageContext, StageError};

/// Stub stage that records invocations and can be told to fail.
struct StubStage {
    kind: StageKind,
    runs: Rc<Cell<usize>>,
    fail: Rc<Cell<bool>>,
}

impl StubStage {
    fn new(kind: StageKind, runs: Rc<Cell<usize>>, fail: Rc<Cell<bool>>) -> Self {
        Self { kind, runs, fail }
    }
}

impl Stage for StubStage {
    fn kind(&self) -> StageKind {
        self.kind
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        self.runs.set(self.runs.get() + 1);
        if self.fail.get() {
            return Err(StageError::new(self.kind, "stub failure"));
        }
        let rel = PathBuf::from(format!("intermediate/{:?}.json", self.kind).to_lowercase());
        // A completed stage's recorded artifacts must exist on disk:
        // resume verifies them before skipping the stage.
        let abs = ctx.registry.job_dir().join(&rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(&abs, b"{}").unwrap();
        let id = ctx.registry.issue();
        Ok(vec![ArtifactRecord {
            artifact_id: id,
            role: format!("{:?}_output", self.kind).to_lowercase(),
            path: rel,
            produced_by: Some(self.kind),
            inputs: vec![],
            params: serde_json::json!({}),
        }])
    }
}

struct StubTrio {
    stages: Vec<Box<dyn Stage>>,
    a_runs: Rc<Cell<usize>>,
    b_runs: Rc<Cell<usize>>,
    c_runs: Rc<Cell<usize>>,
    a_fail: Rc<Cell<bool>>,
    b_fail: Rc<Cell<bool>>,
}

fn stub_trio() -> StubTrio {
    let a_runs = Rc::new(Cell::new(0));
    let b_runs = Rc::new(Cell::new(0));
    let c_runs = Rc::new(Cell::new(0));
    let a_fail = Rc::new(Cell::new(false));
    let b_fail = Rc::new(Cell::new(false));
    let stages: Vec<Box<dyn Stage>> = vec![
        Box::new(StubStage::new(
            StageKind::Ingest,
            a_runs.clone(),
            a_fail.clone(),
        )),
        Box::new(StubStage::new(
            StageKind::Locate,
            b_runs.clone(),
            b_fail.clone(),
        )),
        Box::new(StubStage::new(
            StageKind::CheapAnalysis,
            c_runs.clone(),
            Rc::new(Cell::new(false)),
        )),
    ];
    StubTrio {
        stages,
        a_runs,
        b_runs,
        c_runs,
        a_fail,
        b_fail,
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-resume-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn sample_def(dir: &std::path::Path) -> JobDefinition {
    // `JobDefinition::create` pins input content hashes, so the declared
    // input must be a real file.
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({"preset": "test"}),
        vec![
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
        ],
    )
    .unwrap()
}

#[test]
fn failed_stage_preserves_completed_artifacts_and_resume_completes() {
    let dir = temp_dir("partial");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    trio.b_fail.set(true);
    assert!(runner.run(&def, &mut trio.stages).is_err());
    assert_eq!(
        (trio.a_runs.get(), trio.b_runs.get(), trio.c_runs.get()),
        (1, 1, 0)
    );

    let manifest = Manifest::load(&runner.job_dir(&def.job_id)).unwrap();
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().status,
        StageStatus::Completed
    );
    assert_eq!(
        manifest.stage(StageKind::Locate).unwrap().status,
        StageStatus::Failed
    );
    // One declared-input artifact + the single artifact stage A produced.
    assert_eq!(manifest.artifacts.len(), 2);
    assert_eq!(
        manifest
            .artifacts
            .iter()
            .filter(|a| a.produced_by.is_some())
            .count(),
        1
    );

    trio.b_fail.set(false);
    let manifest = runner.resume(&def, &mut trio.stages).unwrap();
    assert_eq!(
        (trio.a_runs.get(), trio.b_runs.get(), trio.c_runs.get()),
        (1, 2, 1)
    );
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed)
    );
    // One declared-input artifact + one artifact per completed stage.
    assert_eq!(manifest.artifacts.len(), 4);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn duration_ms_is_recorded_and_resume_overwrites_only_rerun_stages() {
    let dir = temp_dir("duration");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    // A failed stage is timed and recorded just like a completed one;
    // pending stages stay `None`.
    trio.b_fail.set(true);
    assert!(runner.run(&def, &mut trio.stages).is_err());
    let job_dir = runner.job_dir(&def.job_id);
    let mut manifest = Manifest::load(&job_dir).unwrap();
    assert!(
        manifest
            .stage(StageKind::Ingest)
            .unwrap()
            .duration_ms
            .is_some()
    );
    assert!(
        manifest
            .stage(StageKind::Locate)
            .unwrap()
            .duration_ms
            .is_some()
    );
    assert_eq!(
        manifest
            .stage(StageKind::CheapAnalysis)
            .unwrap()
            .duration_ms,
        None
    );

    // Sentinel a completed record: resume re-runs only the failed and
    // pending stages, so the sentinel must survive on Ingest while
    // Locate receives a fresh measurement.
    const SENTINEL: u64 = u64::MAX;
    manifest.stage_mut(StageKind::Ingest).unwrap().duration_ms = Some(SENTINEL);
    manifest.save(&job_dir).unwrap();

    trio.b_fail.set(false);
    let manifest = runner.resume(&def, &mut trio.stages).unwrap();
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().duration_ms,
        Some(SENTINEL),
        "completed stage must keep its recorded duration"
    );
    let locate = manifest.stage(StageKind::Locate).unwrap().duration_ms;
    assert!(
        locate.is_some() && locate != Some(SENTINEL),
        "re-run stage must carry a fresh measurement: {locate:?}"
    );
    assert!(
        manifest
            .stage(StageKind::CheapAnalysis)
            .unwrap()
            .duration_ms
            .is_some()
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resume_after_full_success_is_a_noop() {
    let dir = temp_dir("noop");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    runner.run(&def, &mut trio.stages).unwrap();
    runner.resume(&def, &mut trio.stages).unwrap();
    assert_eq!(
        (trio.a_runs.get(), trio.b_runs.get(), trio.c_runs.get()),
        (1, 1, 1)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn reexecution_issues_a_new_job_id_and_manifest() {
    let dir = temp_dir("newjob");
    let runner = StageRunner::new(&dir);
    let first = sample_def(&dir);
    let second = sample_def(&dir);
    assert_ne!(first.job_id, second.job_id);

    let mut trio = stub_trio();
    let m1 = runner.run(&first, &mut trio.stages).unwrap();
    let m2 = runner.run(&second, &mut trio.stages).unwrap();
    assert_ne!(m1.job_id, m2.job_id);
    assert!(
        runner
            .job_dir(&m1.job_id)
            .join(Manifest::FILE_NAME)
            .exists()
    );
    assert!(
        runner
            .job_dir(&m2.job_id)
            .join(Manifest::FILE_NAME)
            .exists()
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn rerun_of_same_job_id_is_rejected() {
    let dir = temp_dir("rerun");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    runner.run(&def, &mut trio.stages).unwrap();
    // Re-running the same definition must not silently overwrite the job.
    let err = runner.run(&def, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("already exists"));
    assert_eq!(trio.a_runs.get(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn run_rejects_leftover_job_dir_without_manifest() {
    let dir = temp_dir("leftover");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    // Simulate a crash mid-first-stage: job dir exists with partial
    // artifacts but no manifest.
    let job_dir = runner.job_dir(&def.job_id);
    std::fs::create_dir_all(job_dir.join("inputs")).unwrap();
    std::fs::write(job_dir.join("partial.bin"), b"x").unwrap();

    let err = runner.run(&def, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("already exists"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn manifest_exists_for_failed_first_stage_and_resume_works() {
    let dir = temp_dir("crash");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    // Even when the very first stage fails, the manifest was persisted
    // before execution, so resume is possible.
    trio.a_fail.set(true);
    assert!(runner.run(&def, &mut trio.stages).is_err());
    assert!(runner.job_dir(&def.job_id).join("manifest.json").exists());

    trio.a_fail.set(false);
    runner.resume(&def, &mut trio.stages).unwrap();
    assert_eq!(trio.a_runs.get(), 2);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resume_with_altered_definition_is_rejected() {
    let dir = temp_dir("altered");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    trio.b_fail.set(true);
    assert!(runner.run(&def, &mut trio.stages).is_err());

    // Same job id but different config: struct-literal bypass.
    let mut altered = def.clone();
    altered.config = serde_json::json!({"preset": "changed"});
    let err = runner.resume(&altered, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("differs"));

    // Same job id but a shorter stage plan than the recorded one:
    // validate_plan accepts it (prefix is still dependency-ordered), so
    // the rejection must come from the recorded-definition check.
    let mut altered = def.clone();
    altered.stages.truncate(2);
    let err = runner.resume(&altered, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("differs"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn duplicate_stage_in_plan_is_rejected() {
    let dir = temp_dir("dup");
    let runner = StageRunner::new(&dir);
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![StageKind::Ingest, StageKind::Locate, StageKind::Ingest],
    )
    .unwrap();
    let mut trio = stub_trio();
    let err = runner.run(&def, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("duplicate stage"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn dependency_listed_after_dependent_is_rejected() {
    let dir = temp_dir("deporder");
    let runner = StageRunner::new(&dir);
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    // Locate depends on Ingest, but the plan runs Locate first: the dep
    // can never complete, so the plan must be rejected up front.
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![StageKind::Locate, StageKind::Ingest],
    )
    .unwrap();
    let mut trio = stub_trio();
    let err = runner.run(&def, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("must precede"));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn concurrent_resume_of_same_job_is_rejected() {
    let dir = temp_dir("locked");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    runner.run(&def, &mut trio.stages).unwrap();

    // A second writer holding the job lock must block any new resume —
    // the lock is what is held, not the file's mere existence. The lock
    // file is a sibling of the job dir (`<base>/<job>.lock`).
    let lock = std::fs::File::create(dir.join(format!("{}.lock", def.job_id))).unwrap();
    lock.try_lock().unwrap();
    let err = runner.resume(&def, &mut trio.stages).unwrap_err();
    assert!(err.message.contains("already running"));

    drop(lock);
    runner.resume(&def, &mut trio.stages).unwrap();

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn leftover_lock_file_does_not_block_run() {
    let dir = temp_dir("stalelock");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    // A crashed process may leave <base>/<job>.lock behind; only a held
    // lock blocks, so the fresh run must succeed.
    std::fs::write(dir.join(format!("{}.lock", def.job_id)), b"").unwrap();
    runner.run(&def, &mut trio.stages).unwrap();
    assert_eq!(trio.a_runs.get(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn dependency_order_violation_is_rejected() {
    let dir = temp_dir("dep");
    let runner = StageRunner::new(&dir);
    // Locate は Ingest に依存するが、計画が Locate のみのため拒否される
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![StageKind::Locate],
    )
    .unwrap();
    let mut trio = stub_trio();
    assert!(runner.run(&def, &mut trio.stages).is_err());

    std::fs::remove_dir_all(&dir).ok();
}

/// Stage that writes a file artifact and records its `byte_len`, so
/// resume can detect truncation, not just absence.
struct ByteLenStage {
    runs: Rc<Cell<usize>>,
}

impl Stage for ByteLenStage {
    fn kind(&self) -> StageKind {
        StageKind::Ingest
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        self.runs.set(self.runs.get() + 1);
        let rel = PathBuf::from("intermediate/pcm.bin");
        let abs = ctx.registry.job_dir().join(&rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        let payload = b"sixty-four-bytes-of-payload";
        std::fs::write(&abs, payload).unwrap();
        Ok(vec![ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "decoded_pcm".into(),
            path: rel,
            produced_by: Some(StageKind::Ingest),
            inputs: vec![],
            params: serde_json::json!({"byte_len": payload.len() as u64}),
        }])
    }
}

/// Stage that produces a directory store root and records its member
/// files in `params.files`, like a dataset table/tensor store.
struct DirStage {
    runs: Rc<Cell<usize>>,
}

impl Stage for DirStage {
    fn kind(&self) -> StageKind {
        StageKind::Ingest
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        self.runs.set(self.runs.get() + 1);
        let rel = PathBuf::from("store/table");
        let abs = ctx.registry.job_dir().join(&rel);
        std::fs::create_dir_all(abs.join("col_a")).unwrap();
        std::fs::write(abs.join("index.json"), b"{}").unwrap();
        std::fs::write(abs.join("col_a/000000.bin"), b"chunk").unwrap();
        Ok(vec![ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "table_store".into(),
            path: rel,
            produced_by: Some(StageKind::Ingest),
            inputs: vec![],
            params: serde_json::json!({"files": ["index.json", "col_a/000000.bin"]}),
        }])
    }
}

#[test]
fn resume_regenerates_missing_completed_artifact() {
    let dir = temp_dir("missing");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    runner.run(&def, &mut trio.stages).unwrap();
    // Simulate a power-loss drop of a relaxed-tier artifact that the
    // manifest already recorded under a completed stage.
    std::fs::remove_file(runner.job_dir(&def.job_id).join("intermediate/ingest.json")).unwrap();

    runner.resume(&def, &mut trio.stages).unwrap();
    // Only the demoted stage re-runs; intact completed stages skip.
    assert_eq!(
        (trio.a_runs.get(), trio.b_runs.get(), trio.c_runs.get()),
        (2, 1, 1)
    );
    // The stale artifact record was replaced by the regenerated one.
    let manifest = Manifest::load(&runner.job_dir(&def.job_id)).unwrap();
    assert_eq!(
        manifest.stage(StageKind::Ingest).unwrap().artifacts.len(),
        1
    );
    let path = manifest
        .artifact(&manifest.stage(StageKind::Ingest).unwrap().artifacts[0])
        .unwrap();
    assert!(runner.job_dir(&def.job_id).join(&path.path).exists());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resume_regenerates_truncated_byte_len_artifact() {
    let dir = temp_dir("truncated");
    let runner = StageRunner::new(&dir);
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![StageKind::Ingest],
    )
    .unwrap();
    let runs = Rc::new(Cell::new(0));
    let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(ByteLenStage { runs: runs.clone() })];

    runner.run(&def, &mut stages).unwrap();
    // Truncate the recorded file: existence passes but byte_len fails.
    let artifact = runner.job_dir(&def.job_id).join("intermediate/pcm.bin");
    std::fs::write(&artifact, b"short").unwrap();

    runner.resume(&def, &mut stages).unwrap();
    assert_eq!(runs.get(), 2);
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        b"sixty-four-bytes-of-payload"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resume_regenerates_dir_artifact_with_missing_member() {
    let dir = temp_dir("dirmiss");
    let runner = StageRunner::new(&dir);
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![StageKind::Ingest],
    )
    .unwrap();
    let runs = Rc::new(Cell::new(0));
    let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(DirStage { runs: runs.clone() })];

    runner.run(&def, &mut stages).unwrap();
    std::fs::remove_file(
        runner
            .job_dir(&def.job_id)
            .join("store/table/col_a/000000.bin"),
    )
    .unwrap();

    runner.resume(&def, &mut stages).unwrap();
    assert_eq!(runs.get(), 2);
    assert!(
        runner
            .job_dir(&def.job_id)
            .join("store/table/col_a/000000.bin")
            .exists()
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn manifest_reconstructs_artifact_lineage() {
    let dir = temp_dir("lineage");
    let runner = StageRunner::new(&dir);
    let def = sample_def(&dir);
    let mut trio = stub_trio();

    let manifest = runner.run(&def, &mut trio.stages).unwrap();
    let ingest_id = manifest.stage(StageKind::Ingest).unwrap().artifacts[0].clone();
    let locate_id = manifest.stage(StageKind::Locate).unwrap().artifacts[0].clone();
    let lineage = manifest.lineage_of(&locate_id);
    assert_eq!(lineage, vec![locate_id]);
    assert_eq!(
        manifest.artifact(&ingest_id).unwrap().produced_by,
        Some(StageKind::Ingest)
    );

    std::fs::remove_dir_all(&dir).ok();
}
