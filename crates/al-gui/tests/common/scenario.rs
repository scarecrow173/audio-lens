//! ruviform#464 Step C / AL#127: driver-neutral `SemanticGuiTest`
//! scenario helpers。
//!
//! AL が Own する scenario は通常の Rust 関数としてここに置き、
//! `FakeGuiTest`(headless)と `GpuiGuiTest`(gpui-test)の両 adapter で
//! 同一契約を実行する。scenario・report の語彙は ruviform-ui-test の
//! contract 型だけを使い、GPUI 型はこの module に現れない。

use std::collections::BTreeSet;

use al_core::{JobId, StageKind, StageStatus};
use al_gui::view::analysis_screen::{ALIGNMENTS_KEY, EVENTS_KEY, EVENTS_NOTE_KEY};
use al_gui::view::desktop::{DESKTOP_KEY, RESUME_KEY, desktop_screen};
use al_gui::view::run_screen::JOBS_KEY;
use al_gui::view::screen::{
    SEEK_KEY, SPECTROGRAM_KEY, SPECTROGRAM_PANEL_KEY, WAVEFORM_KEY, source_screen,
};
use al_gui::view::workspace::{WORKSPACE_KEY, WORKSPACE_TAB_COUNT, workspace_screen};
use al_service::dataset::DatasetSession;
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::parse_input_spec;
use al_testkit::media::wav_s16_bytes;
use al_testkit::signal::{embed, noise};
use al_testkit::temp::TempDir;
use ruviform_ui::WindowId;
use ruviform_ui::behavior::{CommandIntent, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{CellValue, ComponentKey, ComponentSnapshot, ComponentSpec, ViewSpec};
use ruviform_ui_test::{
    CaseId, GuiAction, MintedIntent, RunId, ScenarioCapability, ScenarioError, ScenarioId,
    ScenarioReport, ScenarioTarget, SemanticGuiTest, TargetQuery, parity_gesture_scenario,
};
use serde_json::{Value, json};

use super::{fixture_view, make_job};

/// AL product namespace の scenario/case ID。
pub const SCENARIO_DESKTOP_JOBS: &str = "al.desktop.jobs";
pub const CASE_JOB_SELECT_RESUME: &str = "job-select-resume";
pub const SCENARIO_SOURCE_SEEK: &str = "al.source.seek";
pub const CASE_SEEK_COMMIT: &str = "seek-commit";
pub const SCENARIO_DATASET_WORKSPACE: &str = "al.dataset.workspace";
pub const CASE_WORKSPACE_TABS: &str = "workspace-tabs";

/// production composition root(`bin/al-desktop.rs` の `mount_host`)と
/// 同じ view instance id。scenario の target 解決はこの id を使う。
pub const AL_VIEW: u64 = 1;

/// `AL_VIEW` の `ViewInstanceId`。
pub fn al_view() -> ViewInstanceId {
    ViewInstanceId::new(AL_VIEW).expect("nonzero")
}

/// job fixture 付きの desktop 画面 spec。`TempDir` は spec 構築後も
/// 呼び出し側で保持する(dataset session の基底 dir)。
pub fn desktop_spec() -> (TempDir, ViewSpec, JobId) {
    let dir = TempDir::new("al-scenario-desktop");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    let spec = desktop_screen(Some(&base), Some(&job_id));
    (dir, spec, job_id)
}

/// Source 画面 fixture(`tests/gpui_parity.rs` と同一の waveform)。
pub fn source_spec() -> ViewSpec {
    let view = fixture_view();
    source_screen(&view, 0, 0.0, 0.1, 64).expect("screen spec")
}

/// desktop 画面での job 選択→Run tab→Resume 起動 scenario。
///
/// `SemanticGuiTest` port のみで記述する driver-neutral な流れ:
/// Jobs Table の行 0 を `SelectRow` → desktop root Tabs の Run tab を
/// `SelectTab` → Resume Button を `Invoke`。各操作の間で `settle` し、
/// command intent を drain して返す。Jobs Table の選択状態は
/// read-only `observe` で確認する。
pub struct DesktopJobsOutcome {
    /// pin 済み target(Jobs Table へ narrow)。
    pub target: ScenarioTarget,
    /// 発行された command intent(選択→tab→resume の順)。
    pub intents: Vec<MintedIntent>,
    /// Jobs Table が観測した選択行。
    pub jobs_selected: Option<usize>,
}

/// [`DesktopJobsOutcome`] を得る scenario 本体。driver 非依存。
///
/// # Errors
/// target 解決・capability pre-flight・各 port 操作の `ScenarioError`。
pub fn desktop_jobs_scenario(
    gui: &mut impl SemanticGuiTest,
    window: WindowId,
) -> Result<DesktopJobsOutcome, ScenarioError> {
    let jobs = ComponentKey::from(JOBS_KEY);
    let target =
        gui.resolve_target(&TargetQuery::new(window, al_view()).component(jobs.clone()))?;
    gui.require_capabilities(&BTreeSet::from([ScenarioCapability::SemanticAction]))?;

    gui.perform(
        &target,
        GuiAction::SelectRow {
            key: jobs.clone(),
            row: 0,
        },
    )?;
    gui.settle()?;
    let mut intents = gui.drain_intents();
    let jobs_selected = gui.observe(&target, &jobs)?.selected;

    gui.perform(
        &target,
        GuiAction::SelectTab {
            key: ComponentKey::from(DESKTOP_KEY),
            index: 1,
        },
    )?;
    gui.settle()?;
    intents.extend(gui.drain_intents());

    gui.perform(
        &target,
        GuiAction::Invoke {
            key: ComponentKey::from(RESUME_KEY),
        },
    )?;
    gui.settle()?;
    intents.extend(gui.drain_intents());

    Ok(DesktopJobsOutcome {
        target,
        intents,
        jobs_selected,
    })
}

/// Source 画面の Seek へ ruviform 共通 `parity_gesture_scenario`
/// (Begin→Preview→Commit)を適用する scenario。
pub struct SourceSeekOutcome {
    /// pin 済み target(Seek へ narrow)。
    pub target: ScenarioTarget,
    /// gesture が発行した intent 列。
    pub intents: Vec<MintedIntent>,
    /// gesture 後の Seek snapshot。
    pub snapshot: ComponentSnapshot,
}

/// [`SourceSeekOutcome`] を得る scenario 本体。`plain` は Seek の
/// plain-domain 値(秒)。driver 非依存。
///
/// # Errors
/// target 解決・capability pre-flight・gesture 操作の `ScenarioError`。
pub fn source_seek_scenario(
    gui: &mut impl SemanticGuiTest,
    window: WindowId,
    plain: f64,
) -> Result<SourceSeekOutcome, ScenarioError> {
    let seek = ComponentKey::from(SEEK_KEY);
    let target =
        gui.resolve_target(&TargetQuery::new(window, al_view()).component(seek.clone()))?;
    gui.require_capabilities(&BTreeSet::from([ScenarioCapability::SemanticAction]))?;
    let (intents, snapshot) =
        parity_gesture_scenario(gui, &target, &seek, "al-seek-1", ValueInput::Plain(plain))?;
    Ok(SourceSeekOutcome {
        target,
        intents,
        snapshot,
    })
}

/// `MintedIntent` の `CommandIntent::Action` を
/// `(target, action, params)` へ射影する(AL product oracle)。
pub fn command_actions(intents: &[MintedIntent]) -> Vec<(String, String, Option<Value>)> {
    intents
        .iter()
        .filter_map(|m| match &m.intent {
            CommandIntent::Action {
                target,
                action,
                params,
                ..
            } => Some((target.clone(), action.clone(), params.clone())),
            _ => None,
        })
        .collect()
}

/// adapter の実測 `driver`/`capabilities` を記録した
/// `ScenarioReport` の骨格。`verdict`/`expected`/`observed` は
/// 証拠に応じて caller が設定する — fake の pass を実機 pass へ
/// 昇格させないため、capabilities は adapter の申告値をそのまま使う。
pub fn al_scenario_report(
    gui: &impl SemanticGuiTest,
    run: &str,
    scenario: &str,
    case: &str,
) -> ScenarioReport {
    let mut report = ScenarioReport::new(
        RunId::new(run),
        ScenarioId::new(scenario),
        Some(CaseId::new(case)),
        gui.driver(),
    );
    report.capabilities = gui.capabilities();
    report.required = BTreeSet::from([ScenarioCapability::SemanticAction]);
    report.environment = Some(json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }));
    report
}

// ---------------------------------------------------------------------
// Dataset tab→workspace(Source/Spectrogram/Analysis)scenario
// ---------------------------------------------------------------------

/// dataset workspace fixture が source を capture へ埋め込む offset
/// (sample)。`tests/view.rs` の e2e fixture と同一規約 — locate の
/// 受理済み alignment はこの位置を capture 側 sample 範囲として
/// 持つことが AL 側の既知期待値。
pub const DATASET_EMBED_OFFSET: u64 = 9_600;

/// `dataset_workspace` が構築する job の stage 集合(pipeline order)。
/// Locate が alignments、ExpensiveAnalysis が events/candidates を
/// 供給する — Analysis tab の typed cell oracle に必要。
const DATASET_STAGES: [StageKind; 5] = [
    StageKind::Ingest,
    StageKind::Locate,
    StageKind::CheapAnalysis,
    StageKind::ExpensiveAnalysis,
    StageKind::Dataset,
];

/// workspace Tabs の tab 選択順。index 0(Source)は spec の初期
/// active なので、Spectrogram → Analysis → Source の巡回で 3 tab
/// 全ての `tab-select` intent を発行する。
const WORKSPACE_TAB_SEQUENCE: [usize; 3] = [1, 2, 0];

/// 各 tab の mounted component 存在を観測する代表 key。
/// ViewHost は Tabs の全子孫を mount するため、これは「その tab の
/// 画面が spec に存在し mount された」ことの証明であり、表示上の
/// active 切替は `selected` と intent が担う。
const WORKSPACE_COMPONENT_KEYS: [&str; 6] = [
    SEEK_KEY,
    WAVEFORM_KEY,
    SPECTROGRAM_KEY,
    SPECTROGRAM_PANEL_KEY,
    EVENTS_KEY,
    ALIGNMENTS_KEY,
];

/// Dataset workspace scenario の fixture。
///
/// `tests/view.rs` の `real_dataset_produces_view_models` と同じ
/// fixture 規約: source を capture の `DATASET_EMBED_OFFSET` へ
/// gain 1.0 で埋め込み、`Ingest → Locate → CheapAnalysis →
/// ExpensiveAnalysis → Dataset` を完走させた job の dataset を
/// `workspace_screen` で投影する。`dir` は job/dataset の基底を
/// 保持するため呼び出し側で生存させる。
pub struct DatasetWorkspace {
    /// fixture の基底 tempdir。
    pub dir: TempDir,
    /// mount 対象の workspace `ViewSpec`。
    pub spec: ViewSpec,
    /// 完了した job id。
    pub job_id: JobId,
    /// workspace の主 input(capture_bug)の input id。
    pub capture_input_id: String,
    /// alignment oracle の相手側 input(source_asset)の input id。
    pub source_input_id: String,
}

/// f64 [-1,1] 信号を i16 PCM へ(`tests/view.rs` の `to_i16` と同一)。
fn to_i16(samples: &[f64]) -> Vec<i16> {
    samples
        .iter()
        .map(|v| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect()
}

/// Dataset workspace fixture を構築する。全 stage 完走を確認してから
/// `DatasetSession::open` → `workspace_screen` で spec を得る。
///
/// # Panics
/// job 実行・dataset/spec 構築の失敗時(fixture 前提の破壊)。
pub fn dataset_workspace() -> DatasetWorkspace {
    let dir = TempDir::new("al-scenario-dataset");
    let source_f64 = noise(4_096, 7, 1.0);
    let mut capture_f64 = noise(24_576, 11, 0.02);
    embed(
        &mut capture_f64,
        &source_f64,
        DATASET_EMBED_OFFSET as usize,
        1.0,
    );
    let src_wav = dir.join("src.wav");
    let cap_wav = dir.join("cap.wav");
    std::fs::write(&src_wav, wav_s16_bytes(1, 48_000, &to_i16(&source_f64))).unwrap();
    std::fs::write(&cap_wav, wav_s16_bytes(1, 48_000, &to_i16(&capture_f64))).unwrap();

    let service = JobService::new(dir.join("jobs"));
    let run = RunSpec {
        inputs: vec![
            parse_input_spec(&format!("source_asset:{}", src_wav.display())).unwrap(),
            parse_input_spec(&format!("capture_bug:{}", cap_wav.display())).unwrap(),
        ],
        stages: Some(DATASET_STAGES.to_vec()),
        config: json!({}),
    };
    let (job_id, manifest) = service.run(run, &RunControl::default()).unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed),
        "dataset fixture stages completed: {:?}",
        manifest.stages
    );

    // manifest.inputs は spec.inputs と同順 — [source_asset, capture_bug]。
    let source_input_id = manifest.inputs[0].input_id.as_str().to_string();
    let capture_input_id = manifest.inputs[1].input_id.as_str().to_string();
    let session = DatasetSession::open(service.dataset_dir(&job_id)).unwrap();
    let spec = workspace_screen(&session, &capture_input_id).expect("workspace spec");
    DatasetWorkspace {
        dir,
        spec,
        job_id,
        capture_input_id,
        source_input_id,
    }
}

/// workspace Tabs の tab 選択 scenario の観測結果。
pub struct DatasetWorkspaceOutcome {
    /// pin 済み target(workspace Tabs へ narrow)。
    pub target: ScenarioTarget,
    /// 発行された command intent 列(tab-select のみ)。
    pub intents: Vec<MintedIntent>,
    /// 各 `SelectTab` 後に観測した workspace の `selected`。
    pub selected: Vec<Option<usize>>,
    /// 代表 component key ごとの mounted 観測結果。
    pub mounted: Vec<(&'static str, bool)>,
}

/// component が mount 済みかを観測する。未 mount の key は
/// `UnknownComponent` として `false` に写す(観測不能を Pass にしない)。
fn observe_mounted(
    gui: &impl SemanticGuiTest,
    target: &ScenarioTarget,
    key: &ComponentKey,
) -> Result<bool, ScenarioError> {
    match gui.observe(target, key) {
        Ok(snapshot) => Ok(snapshot.mounted),
        Err(ScenarioError::UnknownComponent { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}

/// [`DatasetWorkspaceOutcome`] を得る scenario 本体。driver 非依存。
///
/// workspace Tabs(`WORKSPACE_KEY`)に対し `WORKSPACE_TAB_SEQUENCE`
/// の順で `SelectTab` を発行し、各 step で settle → intent drain →
/// `selected` 観測を行う。最後に全 tab の代表 component の mounted
/// を観測する。typed cell の oracle は [`dataset_workspace_cells`]
/// が spec 側で別途行う(port の snapshot は table 行を持たない)。
///
/// # Errors
/// target 解決・capability pre-flight・各 port 操作の `ScenarioError`。
pub fn dataset_workspace_scenario(
    gui: &mut impl SemanticGuiTest,
    window: WindowId,
) -> Result<DatasetWorkspaceOutcome, ScenarioError> {
    let workspace = ComponentKey::from(WORKSPACE_KEY);
    let target =
        gui.resolve_target(&TargetQuery::new(window, al_view()).component(workspace.clone()))?;
    gui.require_capabilities(&BTreeSet::from([ScenarioCapability::SemanticAction]))?;

    let mut intents = Vec::new();
    let mut selected = Vec::with_capacity(WORKSPACE_TAB_SEQUENCE.len());
    for index in WORKSPACE_TAB_SEQUENCE {
        gui.perform(
            &target,
            GuiAction::SelectTab {
                key: workspace.clone(),
                index,
            },
        )?;
        gui.settle()?;
        intents.extend(gui.drain_intents());
        selected.push(gui.observe(&target, &workspace)?.selected);
    }

    let mut mounted = Vec::with_capacity(WORKSPACE_COMPONENT_KEYS.len());
    for key in WORKSPACE_COMPONENT_KEYS {
        mounted.push((
            key,
            observe_mounted(gui, &target, &ComponentKey::from(key))?,
        ));
    }

    Ok(DatasetWorkspaceOutcome {
        target,
        intents,
        selected,
        mounted,
    })
}

/// workspace spec の root Tabs を取り出す。構造が違えば `None`。
fn workspace_tabs(spec: &ViewSpec) -> Option<&ruviform_ui::view::TabsSpec> {
    match &spec.root {
        ComponentSpec::Tabs(tabs) => Some(tabs),
        _ => None,
    }
}

/// `ComponentSpec` から key 一致の Table を探す(Panel 子を 1 段だけ辿る)。
fn find_table<'a>(node: &'a ComponentSpec, key: &str) -> Option<&'a ruviform_ui::view::TableSpec> {
    match node {
        ComponentSpec::Table(table) if table.key.0 == key => Some(table),
        ComponentSpec::Panel(panel) => panel
            .children
            .iter()
            .find_map(|child| find_table(child, key)),
        _ => None,
    }
}

/// Label の text を key で引く(Panel 子を 1 段だけ辿る)。
fn find_label_text<'a>(node: &'a ComponentSpec, key: &str) -> Option<&'a str> {
    match node {
        ComponentSpec::Label(label) if label.key.0 == key => Some(label.text.as_str()),
        ComponentSpec::Panel(panel) => panel
            .children
            .iter()
            .find_map(|child| find_label_text(child, key)),
        _ => None,
    }
}

/// セルが期待どおりの typed 値かを JSON 化する(oracle 証拠用)。
fn cell_json(cell: &Option<CellValue>) -> Value {
    match cell {
        Some(CellValue::Text(s)) => json!({ "text": s }),
        Some(CellValue::Number(n)) => {
            if n.is_finite() {
                json!({ "number": n })
            } else {
                json!({ "number": "non_finite" })
            }
        }
        _ => Value::Null,
    }
}

/// Analysis tab の typed cell を AL 独自の期待値で照合する oracle。
///
/// `SemanticGuiTest` の snapshot は table 行を持たないため、typed
/// cell の検証は mount される `ViewSpec` 自体に対して行う — spec の
/// cell は ViewHost がそのまま mount する投影であり、ここでの
/// 不一致は mount 後の表示にも同じく現れる。期待値は fixture の
/// 構築規約(embed offset・受理済み alignment・typed enum 語彙)
/// から AL が独立に持つもので、ruviform Test Harness の自己検証を
/// 転用しない。
///
/// 戻り値は `(ok, evidence)` — `ok` は全 check の合取、`evidence`
/// は report の `expected`/`observed` にそのまま載る JSON。
pub fn dataset_workspace_cells(fixture: &DatasetWorkspace) -> (bool, Value) {
    let mut checks: Vec<Value> = Vec::new();
    let mut ok = true;
    let mut check = |name: &str, pass: bool, detail: Value| {
        ok &= pass;
        checks.push(json!({ "name": name, "pass": pass, "detail": detail }));
    };

    let Some(tabs) = workspace_tabs(&fixture.spec) else {
        check("root-is-tabs", false, json!(fixture.spec.root.kind()));
        return (false, json!({ "checks": checks }));
    };
    check(
        "tab-count",
        tabs.tabs.len() == WORKSPACE_TAB_COUNT,
        json!({ "count": tabs.tabs.len() }),
    );
    let titles: Vec<&str> = tabs.tabs.iter().map(|t| t.title.as_str()).collect();
    check(
        "tab-titles",
        titles == ["Source", "Spectrogram", "Analysis"],
        json!({ "titles": titles }),
    );
    check(
        "selection-target",
        tabs.selection
            .as_ref()
            .is_some_and(|s| s.target == "al.workspace"),
        json!({ "selection": tabs.selection.as_ref().map(|s| &s.target) }),
    );

    // Analysis tab(index 2)の typed cell。
    let analysis = tabs.tabs.get(2).map(|tab| tab.child.as_ref());
    let events = analysis.and_then(|node| find_table(node, EVENTS_KEY));
    let alignments = analysis.and_then(|node| find_table(node, ALIGNMENTS_KEY));
    let events_note = analysis.and_then(|node| find_label_text(node, EVENTS_NOTE_KEY));

    check(
        "events-table-mounted",
        events.is_some(),
        json!({ "present": events.is_some() }),
    );
    check(
        "alignments-table-mounted",
        alignments.is_some(),
        json!({ "present": alignments.is_some() }),
    );

    // events: 行がある場合、typed cell の規約を検査する
    // (strength の非有限値は欠損セル None、start_s/end_s は有限
    // f64、confidence/validity は既知 tag)。空 table も「0 件」と
    // して正直に記録する。
    let events_evidence = events.map(|table| {
        let note_total =
            events_note.and_then(|text| text.rsplit(" of ").next()?.parse::<u64>().ok());
        let rows: Vec<Value> = table
            .rows
            .iter()
            .map(|row| {
                json!({
                    "cells": row.iter().map(cell_json).collect::<Vec<_>>(),
                })
            })
            .collect();
        json!({
            "rows": rows.len(),
            "note_total": note_total,
            "rows_detail": rows,
        })
    });
    let mut events_ok = true;
    if let Some(table) = events {
        for (i, row) in table.rows.iter().enumerate() {
            // start_s(列3)は有限 f64 必須、strength(列5)は非有限なら欠損。
            let start_ok = matches!(row.get(3), Some(Some(CellValue::Number(n))) if n.is_finite());
            let strength_missing_ok = match row.get(5) {
                Some(None) => true, // 欠損セル — NaN/非有限の正直な写像
                Some(Some(CellValue::Number(n))) => n.is_finite(),
                _ => false,
            };
            if !(start_ok && strength_missing_ok) {
                events_ok = false;
                check(
                    &format!("events-row-{i}-typed-cells"),
                    false,
                    json!({ "row": i, "cells": row.iter().map(cell_json).collect::<Vec<_>>() }),
                );
            }
        }
    }
    check(
        "events-typed-cells",
        events_ok,
        events_evidence.clone().unwrap_or(Value::Null),
    );

    // alignments: fixture は source×capture の 1 組を埋め込むため、
    // 受理済み 1 行・capture 範囲が embed offset を含むことが
    // AL 側の独立期待値。
    let mut alignments_evidence = Value::Null;
    let mut alignments_ok = false;
    if let Some(table) = alignments {
        let row = table.rows.first();
        let capture_start = row
            .and_then(|r| r.get(7))
            .and_then(|c| c.as_ref())
            .and_then(|c| match c {
                CellValue::Number(n) => Some(*n),
                _ => None,
            });
        let capture_end = row
            .and_then(|r| r.get(8))
            .and_then(|c| c.as_ref())
            .and_then(|c| match c {
                CellValue::Number(n) => Some(*n),
                _ => None,
            });
        let status = row
            .and_then(|r| r.get(3))
            .and_then(|c| c.as_ref())
            .and_then(|c| match c {
                CellValue::Text(s) => Some(s.as_str()),
                _ => None,
            });
        let source_id = row
            .and_then(|r| r.first())
            .and_then(|c| c.as_ref())
            .and_then(|c| match c {
                CellValue::Text(s) => Some(s.as_str()),
                _ => None,
            });
        let capture_id = row
            .and_then(|r| r.get(1))
            .and_then(|c| c.as_ref())
            .and_then(|c| match c {
                CellValue::Text(s) => Some(s.as_str()),
                _ => None,
            });
        // embed 区間 [9600, 9600+4096) を含む受理 alignment を期待。
        // alignment の境界は手法依存のため、開始が offset 近傍
        // (± source 長)で終了が開始より後、という範囲で判定する。
        let range_ok = capture_start.is_some_and(|start| {
            capture_end.is_some_and(|end| {
                let start = start as i64;
                let end = end as i64;
                let offset = DATASET_EMBED_OFFSET as i64;
                let src_len = 4_096i64;
                end > start
                    && start >= offset - src_len
                    && start <= offset + src_len
                    && end >= offset
                    && end <= offset + 2 * src_len
            })
        });
        alignments_ok = table.rows.len() == 1
            && status == Some("accepted")
            && source_id == Some(fixture.source_input_id.as_str())
            && capture_id == Some(fixture.capture_input_id.as_str())
            && range_ok;
        alignments_evidence = json!({
            "rows": table.rows.len(),
            "row0": row.map(|r| r.iter().map(cell_json).collect::<Vec<_>>()),
            "expected": {
                "status": "accepted",
                "source_input_id": fixture.source_input_id,
                "capture_input_id": fixture.capture_input_id,
                "capture_start_near": DATASET_EMBED_OFFSET,
            },
        });
    }
    check(
        "alignments-accepted-embed-range",
        alignments_ok,
        alignments_evidence,
    );

    (ok, json!({ "checks": checks }))
}
