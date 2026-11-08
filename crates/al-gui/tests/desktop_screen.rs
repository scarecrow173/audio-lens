//! AL-G4: al-desktop の root 画面(Jobs・Run・Dataset の Tabs)の射影を実 job で
//! 検証する。`JobService::run` で作った job を base_dir に置き、Fake 層
//! (`ViewHost`)へ mount して tab の構造・Job 一覧・Run 結果・Dataset 画面を確かめる。

mod common;

use al_core::{JobId, StageKind};
use al_gui::view::desktop::{
    DATASET_TAB_KEY, DESKTOP_KEY, JOBS_TAB_KEY, RUN_TAB_KEY, desktop_screen,
};
use al_gui::view::run_screen::{JOBS_KEY, RUN_STATUS_KEY, STAGES_KEY};
use al_gui::view::screen::SPECTROGRAM_KEY;
use al_gui::view::workspace::WORKSPACE_KEY;
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::parse_input_spec;
use al_testkit::temp::TempDir;
use common::{JOB_STAGES, make_job};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpochAllocator, CellValue, ComponentKey, ComponentSpec, TabsSpec, ViewHost, ViewSpec,
};
use serde_json::json;

fn mount(view: &ViewSpec) -> ViewHost {
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpochAllocator::new().mint(),
    );
    host.mount(view.clone()).expect("mount");
    host
}

fn root_tabs(view: &ViewSpec) -> &TabsSpec {
    let ComponentSpec::Tabs(tabs) = &view.root else {
        panic!("root must be tabs");
    };
    tabs
}

/// `tabs` の key の child 画面。
fn tab<'a>(tabs: &'a TabsSpec, key: &str) -> &'a ComponentSpec {
    tabs.tabs
        .iter()
        .find(|t| t.key == ComponentKey::from(key))
        .map(|t| t.child.as_ref())
        .unwrap_or_else(|| panic!("tab {key} missing"))
}

/// 画面(Panel)の child を key で引く。
fn child<'a>(node: &'a ComponentSpec, key: &str) -> &'a ComponentSpec {
    let ComponentSpec::Panel(panel) = node else {
        panic!("{} must be a panel", node.kind());
    };
    panel
        .children
        .iter()
        .find(|c| c.key() == &ComponentKey::from(key))
        .unwrap_or_else(|| panic!("child {key} missing"))
}

fn label_text(node: &ComponentSpec) -> &str {
    let ComponentSpec::Label(label) = node else {
        panic!("{} must be a label", node.kind());
    };
    &label.text
}

fn setup() -> (TempDir, std::path::PathBuf, JobId) {
    let dir = TempDir::new("al-gui-desktop-screen");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    (dir, base, job_id)
}

#[test]
fn desktop_mounts_jobs_run_and_dataset_tabs_for_the_selected_job() {
    let (_dir, base, job_id) = setup();
    let view = desktop_screen(Some(&base), Some(&job_id));

    let tabs = root_tabs(&view);
    assert_eq!(tabs.key, ComponentKey::from(DESKTOP_KEY));
    let keys: Vec<&str> = tabs.tabs.iter().map(|t| t.key.0.as_str()).collect();
    assert_eq!(keys, [JOBS_TAB_KEY, RUN_TAB_KEY, DATASET_TAB_KEY]);

    let host = mount(&view);
    for key in keys {
        assert!(
            host.observe(&ComponentKey::from(key))
                .expect("tab mounted")
                .mounted
        );
    }

    let ComponentSpec::Table(jobs) = child(tab(tabs, JOBS_TAB_KEY), JOBS_KEY) else {
        panic!("jobs must be a table");
    };
    assert_eq!(jobs.rows.len(), 1);
    assert_eq!(
        jobs.rows[0][0],
        Some(CellValue::Text(job_id.as_str().to_string()))
    );

    let run = tab(tabs, RUN_TAB_KEY);
    assert_eq!(
        label_text(child(run, RUN_STATUS_KEY)),
        format!(
            "job {}: completed {JOB_STAGES}/{JOB_STAGES}, failed 0",
            job_id.as_str()
        )
    );
    let ComponentSpec::Table(stages) = child(run, STAGES_KEY) else {
        panic!("stages must be a table");
    };
    assert_eq!(stages.rows.len(), JOB_STAGES);
    assert!(
        stages
            .rows
            .iter()
            .all(|r| r[1] == Some(CellValue::Text("completed".to_string())))
    );

    let ComponentSpec::Tabs(dataset) = tab(tabs, DATASET_TAB_KEY) else {
        panic!("dataset must be the workspace tabs");
    };
    assert_eq!(dataset.key, ComponentKey::from(WORKSPACE_KEY));
    let spectrogram = tab(dataset, "tab-spectrogram");
    assert!(
        matches!(child(spectrogram, SPECTROGRAM_KEY), ComponentSpec::Spectrogram(s) if !s.columns.is_empty())
    );
}

#[test]
fn jobs_tab_lists_jobs_while_run_and_dataset_report_no_selection() {
    let (_dir, base, job_id) = setup();
    let view = desktop_screen(Some(&base), None);
    let tabs = root_tabs(&view);

    let ComponentSpec::Table(jobs) = child(tab(tabs, JOBS_TAB_KEY), JOBS_KEY) else {
        panic!("jobs must be a table");
    };
    assert_eq!(
        jobs.rows[0][0],
        Some(CellValue::Text(job_id.as_str().to_string()))
    );
    assert_eq!(
        label_text(child(tab(tabs, RUN_TAB_KEY), "run-view-unavailable")),
        "unavailable: not found: no job selected"
    );
    assert_eq!(
        label_text(child(
            tab(tabs, DATASET_TAB_KEY),
            "dataset-view-unavailable"
        )),
        "unavailable: not found: no job selected"
    );
    mount(&view);
}

#[test]
fn without_base_dir_every_tab_reports_the_missing_base() {
    let view = desktop_screen(None, None);
    let tabs = root_tabs(&view);
    for (key, panel_key) in [
        (JOBS_TAB_KEY, "jobs-view"),
        (RUN_TAB_KEY, "run-view"),
        (DATASET_TAB_KEY, "dataset-view"),
    ] {
        assert_eq!(
            label_text(child(tab(tabs, key), &format!("{panel_key}-unavailable"))),
            "unavailable: not found: base_dir is not set"
        );
    }
    mount(&view);
}

fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let target = dst.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn dataset_tab_opens_the_requested_job_not_the_id_in_its_manifest() {
    let (_dir, base, job_id) = setup();
    // job の複製: manifest は元の job id のまま、analysis は持たない。
    let copy = JobId::new();
    copy_dir(&base.join(job_id.as_str()), &base.join(copy.as_str()));
    std::fs::remove_dir_all(base.join(copy.as_str()).join("analysis")).unwrap();

    let view = desktop_screen(Some(&base), Some(&copy));
    let tabs = root_tabs(&view);
    assert!(
        label_text(child(
            tab(tabs, DATASET_TAB_KEY),
            "dataset-view-unavailable"
        ))
        .starts_with("unavailable: "),
        "the requested job has no dataset, so the tab must not show the manifest job's dataset"
    );
}

#[test]
fn job_without_dataset_stage_fails_only_the_dataset_tab() {
    let dir = TempDir::new("al-gui-desktop-no-dataset");
    let base = dir.join("jobs");
    let wav = dir.join("cap.wav");
    let samples: Vec<i16> = (0..4_800).map(|i| (i % 400) as i16 - 200).collect();
    std::fs::write(&wav, al_testkit::media::wav_s16_bytes(1, 48_000, &samples)).unwrap();
    let spec = RunSpec {
        inputs: vec![parse_input_spec(&format!("capture_bug:{}", wav.display())).unwrap()],
        stages: Some(vec![StageKind::Ingest]),
        config: json!({}),
    };
    let (job_id, _) = JobService::new(&base)
        .run(spec, &RunControl::default())
        .unwrap();

    let view = desktop_screen(Some(&base), Some(&job_id));
    let tabs = root_tabs(&view);
    assert!(
        matches!(child(tab(tabs, JOBS_TAB_KEY), JOBS_KEY), ComponentSpec::Table(t) if t.rows.len() == 1)
    );
    assert!(
        matches!(child(tab(tabs, RUN_TAB_KEY), STAGES_KEY), ComponentSpec::Table(t) if t.rows.len() == 1)
    );
    assert!(
        label_text(child(
            tab(tabs, DATASET_TAB_KEY),
            "dataset-view-unavailable"
        ))
        .starts_with("unavailable: ")
    );
    mount(&view);
}

#[test]
fn a_job_outside_the_base_dir_fails_only_its_tabs() {
    let (_dir, base, _job_id) = setup();
    let missing = JobId::new();
    let view = desktop_screen(Some(&base), Some(&missing));
    let tabs = root_tabs(&view);

    assert!(
        matches!(child(tab(tabs, JOBS_TAB_KEY), JOBS_KEY), ComponentSpec::Table(t) if t.rows.len() == 1)
    );
    assert_eq!(
        label_text(child(tab(tabs, RUN_TAB_KEY), "run-view-unavailable")),
        format!("unavailable: job {} not found", missing.as_str())
    );
    assert_eq!(
        label_text(child(
            tab(tabs, DATASET_TAB_KEY),
            "dataset-view-unavailable"
        )),
        format!("unavailable: job {} not found", missing.as_str())
    );
    mount(&view);
}
