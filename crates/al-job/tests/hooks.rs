//! `RunHooks` / `StageEvent` の回帰試験。GUI(al-gui)の進捗表示と
//! キャンセルがこの hook を通じて行われるため、stage 境界での
//! イベント発火・キャンセル時の Pending 保持・resume の Reused
//! 発火を検証する。

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

use al_core::{InputRole, StageKind, StageStatus};
use al_job::job::{JobDefinition, input};
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::runner::{RunHooks, StageEvent, StageEventKind, StageRunner};
use al_job::stage::{Stage, StageContext, StageError};

struct StubStage {
    kind: StageKind,
    runs: Rc<Cell<usize>>,
}

impl Stage for StubStage {
    fn kind(&self) -> StageKind {
        self.kind
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        self.runs.set(self.runs.get() + 1);
        let rel = PathBuf::from(format!("intermediate/{:?}.json", self.kind).to_lowercase());
        let abs = ctx.registry.job_dir().join(&rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(&abs, b"{}").unwrap();
        Ok(vec![ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: format!("{:?}_output", self.kind).to_lowercase(),
            path: rel,
            produced_by: Some(self.kind),
            inputs: vec![],
            params: serde_json::json!({}),
        }])
    }
}

fn fixture(
    dir: &std::path::Path,
    run_counts: &[Rc<Cell<usize>>],
) -> (JobDefinition, Vec<Box<dyn Stage>>) {
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    let def = JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
        ],
    )
    .unwrap();
    let stages: Vec<Box<dyn Stage>> = [
        StageKind::Ingest,
        StageKind::Locate,
        StageKind::CheapAnalysis,
    ]
    .iter()
    .enumerate()
    .map(|(i, kind)| {
        Box::new(StubStage {
            kind: *kind,
            runs: run_counts[i].clone(),
        }) as Box<dyn Stage>
    })
    .collect();
    (def, stages)
}

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-hooks-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn collect_events() -> Rc<RefCell<Vec<StageEvent>>> {
    Rc::new(RefCell::new(Vec::new()))
}

#[test]
fn progress_events_fire_at_stage_boundaries() {
    let dir = temp_dir("progress");
    let runner = StageRunner::new(&dir);
    let counts = [
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
    ];
    let (def, mut stages) = fixture(&dir, &counts);
    let events_sink = collect_events();
    let events = events_sink.clone();

    let on_stage = |ev: StageEvent| events.borrow_mut().push(ev);
    let hooks = RunHooks {
        on_stage: Some(&on_stage),
        is_cancelled: None,
    };
    runner.run_with_hooks(&def, &mut stages, &hooks).unwrap();

    let seen: Vec<(StageKind, usize, usize, bool)> = events_sink
        .borrow()
        .iter()
        .map(|e| {
            let done = matches!(
                e.kind,
                StageEventKind::Completed { .. } | StageEventKind::Reused
            );
            (e.stage, e.index, e.total, done)
        })
        .collect();
    // Started+Completed per stage、index は plan order、total=3。
    assert_eq!(seen.len(), 6);
    for (i, stage) in [
        StageKind::Ingest,
        StageKind::Locate,
        StageKind::CheapAnalysis,
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(seen[i * 2], (*stage, i, 3, false), "started at {i}");
        assert_eq!(seen[i * 2 + 1], (*stage, i, 3, true), "completed at {i}");
    }

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cancel_between_stages_leaves_pending_and_manifest_persisted() {
    let dir = temp_dir("cancel");
    let runner = StageRunner::new(&dir);
    let counts = [
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
    ];
    let (def, mut stages) = fixture(&dir, &counts);

    // 2 stage 目完了後にキャンセル — 3 つ目の stage は起動しない。
    let started = Cell::new(0usize);
    let on_stage = |_: StageEvent| {};
    let is_cancelled = || {
        started.set(started.get() + 1);
        // キャンセル判定は各 stage 起動前に呼ばれる。2 回目の判定
        // (= Locate 起動前)以降で true → Locate は走り、
        // CheapAnalysis 手前で停止する想定は走査順に依存するため、
        // ここでは「最初の判定から常に true」ではなく回数で制御する。
        started.get() > 2
    };
    let hooks = RunHooks {
        on_stage: Some(&on_stage),
        is_cancelled: Some(&is_cancelled),
    };
    let err = runner
        .run_with_hooks(&def, &mut stages, &hooks)
        .unwrap_err();
    assert!(err.message.contains("cancel") || err.message.contains("cancelled"));

    // manifest が保存され、未実行 stage は Pending のまま。
    let manifest = Manifest::load(&runner.job_dir(&def.job_id)).unwrap();
    assert_eq!(
        manifest.stage(StageKind::CheapAnalysis).unwrap().status,
        StageStatus::Pending
    );
    assert!(counts[2].get() == 0);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cancel_before_first_stage_runs_nothing() {
    let dir = temp_dir("cancel0");
    let runner = StageRunner::new(&dir);
    let counts = [
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
    ];
    let (def, mut stages) = fixture(&dir, &counts);

    let always = || true;
    let hooks = RunHooks {
        on_stage: None,
        is_cancelled: Some(&always),
    };
    assert!(runner.run_with_hooks(&def, &mut stages, &hooks).is_err());
    assert_eq!(
        counts.iter().map(|c| c.get()).sum::<usize>(),
        0,
        "no stage body may run once cancelled"
    );
    let manifest = Manifest::load(&runner.job_dir(&def.job_id)).unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Pending)
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn resume_emits_reused_for_completed_stages() {
    let dir = temp_dir("reused");
    let runner = StageRunner::new(&dir);
    let counts = [
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
        Rc::new(Cell::new(0usize)),
    ];
    let (def, mut stages) = fixture(&dir, &counts);
    runner.run(&def, &mut stages).unwrap();

    let events_sink = collect_events();
    let events = events_sink.clone();
    let on_stage = |ev: StageEvent| events.borrow_mut().push(ev);
    let hooks = RunHooks {
        on_stage: Some(&on_stage),
        is_cancelled: None,
    };
    runner.resume_with_hooks(&def, &mut stages, &hooks).unwrap();

    let kinds: Vec<StageEventKind> = events_sink.borrow().iter().map(|e| e.kind).collect();
    // 完了済み 3 stage が全て Reused(再実行なし)。
    assert_eq!(
        kinds,
        vec![
            StageEventKind::Reused,
            StageEventKind::Reused,
            StageEventKind::Reused
        ]
    );
    assert_eq!(
        counts.iter().map(|c| c.get()).sum::<usize>(),
        3,
        "no stage body re-ran on resume"
    );

    std::fs::remove_dir_all(&dir).ok();
}
