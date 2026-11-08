//! AL-G4: Job/Run/Analysis の `ViewSpec` 射影。Label(状態テキスト)と
//! Table(stage 表・events 表・alignment 表・job 一覧)へ写し、Fake 層
//! (`ViewHost`)の mount 検証まで確かめる。数値は typed 写しをそのまま
//! 表示し、非有限値は欠損セルとして位置を保つ。

use al_gui::view::analysis::{AlignmentRow, EventRow};
use al_gui::view::analysis_screen::{
    ALIGNMENTS_KEY, ANALYSIS_PANEL_KEY, EVENTS_KEY, EVENTS_NOTE_KEY, alignments_section,
    analysis_screen, events_section,
};
use al_gui::view::job::{InputRow, JobRow, RunOutcome};
use al_gui::view::run_screen::{JOBS_KEY, RUN_STATUS_KEY, STAGES_KEY, job_list, run_screen};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, LabelSpec, MAX_TABLE_ROWS, TableSpec, ViewHost,
    ViewSpec,
};
use serde_json::json;

fn children(view: &ViewSpec) -> &[ComponentSpec] {
    let ComponentSpec::Panel(panel) = &view.root else {
        panic!("root must be a panel");
    };
    &panel.children
}

fn label(c: &ComponentSpec) -> &LabelSpec {
    match c {
        ComponentSpec::Label(l) => l,
        other => panic!("expected label, got {}", other.kind()),
    }
}

fn table(c: &ComponentSpec) -> &TableSpec {
    match c {
        ComponentSpec::Table(t) => t,
        other => panic!("expected table, got {}", other.kind()),
    }
}

/// 表示文字列の格子。欠損セルは `-`。
fn grid(t: &TableSpec) -> Vec<Vec<String>> {
    t.rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| {
                    c.as_ref()
                        .map_or_else(|| "-".to_string(), ToString::to_string)
                })
                .collect()
        })
        .collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

fn mount(view: ViewSpec) -> ViewHost {
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpoch::new(1),
    );
    host.mount(view).expect("mount");
    host
}

fn run_fixture() -> RunOutcome {
    RunOutcome::from_json(&json!({
        "job_id": "job-1",
        "stages": [
            {"stage": "ingest", "status": "completed", "duration_ms": 12},
            {"stage": "locate", "status": "failed", "duration_ms": null, "error": "no match"},
            {"stage": "dataset", "status": "skipped", "duration_ms": null}
        ]
    }))
    .expect("run outcome")
}

fn event_row(event_id: &str, strength: serde_json::Value) -> EventRow {
    EventRow::from_json(&json!({
        "event_id": event_id,
        "kind": "silence",
        "input_id": "cap",
        "start_s": 1.0,
        "end_s": null,
        "strength": strength,
        "confidence": "low",
        "validity": "insufficient_support",
        "source_feature": "cheap.rms",
        "freq_lo_hz": null,
        "freq_hi_hz": null,
        "algorithm_id": "cheap.v1",
        "algorithm_version": "1"
    }))
    .expect("event row")
}

#[test]
fn run_status_label_and_stage_table_keep_failure_reasons() {
    let view = run_screen(&run_fixture(), Vec::new()).expect("run screen");
    let items = children(&view);

    let status: &LabelSpec = label(&items[0]);
    assert_eq!(status.key, ComponentKey::from(RUN_STATUS_KEY));
    assert_eq!(status.text, "job job-1: completed 1/3, failed 1");

    let stages = table(&items[1]);
    assert_eq!(stages.key, ComponentKey::from(STAGES_KEY));
    assert_eq!(
        stages.columns,
        strs(&["stage", "status", "duration_ms", "error"])
    );
    assert_eq!(
        grid(stages),
        [
            vec!["ingest", "completed", "12", "-"],
            vec!["locate", "failed", "-", "no match"],
            vec!["dataset", "skipped", "-", "-"],
        ]
    );
}

#[test]
fn job_list_is_a_one_column_table_of_job_ids() {
    let job = |id: &str| JobRow {
        job_id: id.to_string(),
        inputs: vec![InputRow {
            input_id: "cap".to_string(),
            role: "capture_bug".to_string(),
            path: "cap.mp4".to_string(),
        }],
        stages: Vec::new(),
        errors: Vec::new(),
        dataset_ready: true,
    };
    let list = job_list(&[job("job-1"), job("job-2")]).expect("job list");
    let t = table(&list);
    assert_eq!(t.key, ComponentKey::from(JOBS_KEY));
    assert_eq!(t.columns, strs(&["job"]));
    assert_eq!(grid(t), [vec!["job-1"], vec!["job-2"]]);
}

#[test]
fn analysis_screen_projects_events_and_alignments_as_tables() {
    let events = [
        EventRow::from_json(&json!({
            "event_id": "ev-1", "kind": "level_anomaly", "input_id": "cap",
            "start_s": 0.25, "end_s": 0.75, "strength": 0.9,
            "confidence": "high", "validity": "valid", "source_feature": "cheap.rms",
            "freq_lo_hz": 100.0, "freq_hi_hz": 8000.0,
            "algorithm_id": "cheap.v1", "algorithm_version": "1.0.0"
        }))
        .expect("event"),
        event_row("ev-2", serde_json::Value::Null),
    ];
    let alignment = AlignmentRow::from_json(&json!({
        "source_input_id": "src", "capture_input_id": "cap", "alignment_id": "al-1",
        "status": "accepted", "validity": "valid", "confidence": "high",
        "agreeing_methods": 3, "capture_start": 100, "capture_end": 4900
    }))
    .expect("alignment");

    let mut sections = events_section(&events, 7).expect("events section");
    sections.push(alignments_section(&[alignment]).expect("alignments section"));
    let view = analysis_screen(sections);
    let items = children(&view);
    assert_eq!(label(&items[0]).key, ComponentKey::from(EVENTS_NOTE_KEY));
    assert_eq!(label(&items[0]).text, "events 2 of 7");

    let ev = table(&items[1]);
    assert_eq!(ev.key, ComponentKey::from(EVENTS_KEY));
    assert_eq!(
        ev.columns,
        strs(&[
            "event",
            "kind",
            "input",
            "start_s",
            "end_s",
            "strength",
            "confidence",
            "validity"
        ])
    );
    assert_eq!(
        grid(ev),
        [
            vec![
                "ev-1",
                "level_anomaly",
                "cap",
                "0.25",
                "0.75",
                "0.9",
                "high",
                "valid"
            ],
            vec![
                "ev-2",
                "silence",
                "cap",
                "1",
                "-",
                "-",
                "low",
                "insufficient_support"
            ],
        ]
    );

    let al = table(&items[2]);
    assert_eq!(al.key, ComponentKey::from(ALIGNMENTS_KEY));
    assert_eq!(
        grid(al),
        [vec![
            "src", "cap", "al-1", "accepted", "valid", "high", "3", "100", "4900"
        ]]
    );
}

#[test]
fn analysis_rejects_more_rows_than_a_table_can_carry() {
    let row = event_row("ev", json!(0.5));
    let rows = vec![row; MAX_TABLE_ROWS + 1];
    assert!(events_section(&rows, rows.len() as u64).is_err());
}

#[test]
fn run_and_analysis_screens_mount_in_the_fake_host() {
    let run = mount(run_screen(&run_fixture(), Vec::new()).expect("run"));
    assert!(
        run.observe(&ComponentKey::from(RUN_STATUS_KEY))
            .expect("status mounted")
            .mounted
    );

    let events = [event_row("ev-2", json!(null))];
    let analysis = mount(analysis_screen(events_section(&events, 1).expect("events")));
    assert!(
        analysis
            .observe(&ComponentKey::from(ANALYSIS_PANEL_KEY))
            .is_some(),
        "analysis panel mounted"
    );
    assert!(
        analysis
            .observe(&ComponentKey::from(EVENTS_KEY))
            .expect("events mounted")
            .mounted
    );
}
