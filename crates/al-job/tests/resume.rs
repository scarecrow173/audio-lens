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
        let id = ctx.registry.issue();
        Ok(vec![ArtifactRecord {
            artifact_id: id,
            role: format!("{:?}_output", self.kind).to_lowercase(),
            path: PathBuf::from(format!("intermediate/{:?}.json", self.kind).to_lowercase()),
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
    b_fail: Rc<Cell<bool>>,
}

fn stub_trio() -> StubTrio {
    let a_runs = Rc::new(Cell::new(0));
    let b_runs = Rc::new(Cell::new(0));
    let c_runs = Rc::new(Cell::new(0));
    let b_fail = Rc::new(Cell::new(false));
    let stages: Vec<Box<dyn Stage>> = vec![
        Box::new(StubStage::new(
            StageKind::Ingest,
            a_runs.clone(),
            Rc::new(Cell::new(false)),
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
