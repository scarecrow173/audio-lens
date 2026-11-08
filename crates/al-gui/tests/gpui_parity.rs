//! AL-G3/AL-G4 GPUI parity: Source 画面の同一 `ViewSpec` を Fake 層(`ViewHost`
//! 直接評価)と GPUI Driver(`GpuiObserver` 経由の mount)で評価し、
//! mount 済みの key・control の projection・intent が一致することを検証する。
//!
//! GPUI 側は consumer 観測 seam(`ruviform-ui-gpui` の `GpuiObserver`)
//! だけを使い、GPUI 型は持たない。描画 plan は `GpuiObserver::drawn` で
//! Meter・TimeAxis・Waveform・Spectrogram・Label・Table を観測し、mount した
//! spec の payload から導いた Fake 層の期待値と照合する(`DrawnPlan` の
//! Spectrogram・Label・Table は ruviform#442)。

#![cfg(feature = "gpui-test")]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::sync::Arc;

use al_gui::view::desktop::{DESKTOP_KEY, RESUME_KEY, desktop_screen};
use al_gui::view::job::{JobRow, RunOutcome};
use al_gui::view::run_screen::{JOBS_KEY, RUN_STATUS_KEY, STAGES_KEY, job_list, run_screen};
use al_gui::view::screen::{
    METER_KEY, RULER_KEY, SEEK_KEY, SEEK_TARGET, SPECTROGRAM_KEY, SPECTROGRAM_RANGE_KEY,
    WAVEFORM_KEY, source_screen, spectrogram_screen,
};
use al_gui::view::workspace::WORKSPACE_KEY;
use al_testkit::temp::TempDir;
use common::{GAP, fixture_view, make_job};
use ruviform_ui::behavior::{CommandPhase, ControlInput, InputSource, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11ySpec, AttachEpoch, AttachEpochAllocator, ComponentKey, ComponentSnapshot, ComponentSpec,
    ComponentVisualState, MeterSpec, PanelSpec, SpectrogramSpec, TableSpec, TimeAxisSpec,
    ViewEvent, ViewHost, ViewSpec, WaveformSpec,
};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{
    DrawnBar, DrawnCell, DrawnMark, DrawnPlan, DrawnWaveformChannel, GpuiObserver, ViewMount,
};
use ruviform_viz::{
    Aggregate, BinAxis, FrameGrid, RowSpec, Spectrogram, SpectrumData, SpectrumInfo, TileRequest,
    VecSpectrum,
};
use serde_json::{Value, json};

const COMMAND_OP: &str = "rvf.ui.command.v1";
const SEEK_PLAIN: f64 = 0.05;

/// 両 driver で比較する control/component の projection。bit 同一で比べる。
#[derive(Debug, PartialEq)]
struct Projection {
    mounted: bool,
    phase: CommandPhase,
    display: Option<(u64, u64)>,
    committed: Option<(u64, u64)>,
}

fn project(s: &ComponentSnapshot) -> Projection {
    let bits =
        |v: &ruviform_ui::behavior::ControlValue| (v.plain.to_bits(), v.normalized.to_bits());
    Projection {
        mounted: s.mounted,
        phase: s.phase,
        display: s.display.as_ref().map(bits),
        committed: s.committed.as_ref().map(bits),
    }
}

fn seek_gesture(plain: f64) -> [ControlInput; 3] {
    let gesture_id = "s1".to_string();
    [
        ControlInput::Begin {
            gesture_id: gesture_id.clone(),
            source: InputSource::from("pointer"),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        },
        ControlInput::Preview {
            gesture_id: gesture_id.clone(),
            value: ValueInput::Plain(plain),
            modifiers: None,
            seq: None,
        },
        ControlInput::Commit {
            gesture_id,
            value: None,
            modifiers: None,
            seq: None,
        },
    ]
}

fn spec() -> ViewSpec {
    let view = fixture_view();
    source_screen(&view, 0, 0.0, 0.1, 64).expect("screen spec")
}

/// Fake 層: `ViewHost` を直接駆動する。
struct Fake {
    host: ViewHost,
}

impl Fake {
    fn new(spec: ViewSpec) -> Self {
        let epoch = AttachEpochAllocator::new().mint();
        let mut host = ViewHost::new(ViewInstanceId::new(429).expect("nonzero"), epoch);
        host.mount(spec).expect("mount");
        host.drain_events();
        Self { host }
    }

    fn keys(&self) -> BTreeSet<String> {
        self.host.mounted_keys().into_iter().map(|k| k.0).collect()
    }

    fn projections(&self) -> BTreeMap<String, Projection> {
        self.keys()
            .into_iter()
            .map(|k| {
                let snap = self
                    .host
                    .observe(&ComponentKey(k.clone()))
                    .expect("mounted snapshot");
                (k, project(&snap))
            })
            .collect()
    }

    fn gesture(&mut self, plain: f64) -> Vec<Value> {
        let key = ComponentKey::from(SEEK_KEY);
        for input in seek_gesture(plain) {
            let generation = self.host.generation();
            self.host
                .dispatch_input(generation, &key, input)
                .expect("dispatch");
        }
        self.host
            .drain_events()
            .into_iter()
            .filter_map(|e| match e {
                ViewEvent::Intent { intent, .. } => {
                    Some(serde_json::to_value(intent).expect("json"))
                }
                _ => None,
            })
            .collect()
    }
}

/// GPUI 層: `GpuiObserver` で window に mount し、seam 経由で観測する。
struct Gpui {
    observer: GpuiObserver,
    window: WindowId,
}

impl Gpui {
    fn new(spec: ViewSpec) -> Self {
        let epoch: AttachEpoch = AttachEpochAllocator::new().mint();
        let mut host = ViewHost::new(ViewInstanceId::new(429).expect("nonzero"), epoch);
        host.mount(spec).expect("mount");
        host.drain_events();
        let mut observer = GpuiObserver::new();
        let window = observer
            .driver_mut()
            .create_view_window(&WindowSpec::default(), ViewMount { host, theme: None })
            .expect("create_view_window");
        observer.settle(|_, _| {});
        Self { observer, window }
    }

    fn keys(&self) -> BTreeSet<String> {
        self.observer
            .observe(self.window)
            .expect("observed view")
            .components
            .into_iter()
            .map(|c| c.key.0)
            .collect()
    }

    fn projections(&self) -> BTreeMap<String, Projection> {
        self.observer
            .observe(self.window)
            .expect("observed view")
            .components
            .into_iter()
            .map(|c| (c.key.0, project(&c.snapshot)))
            .collect()
    }

    /// gesture の間に新しく届いた command intent の `params.intent` を返す。
    fn gesture(&mut self, plain: f64) -> Vec<Value> {
        let before = self.command_intents().len();
        let key = ComponentKey::from(SEEK_KEY);
        for input in seek_gesture(plain) {
            self.observer
                .dispatch_input(self.window, &key, input)
                .expect("dispatch");
        }
        self.observer.settle(|_, _| {});
        self.command_intents()[before..].to_vec()
    }

    fn command_intents(&self) -> Vec<Value> {
        self.observer
            .intents()
            .into_iter()
            .filter(|i| i.op == COMMAND_OP)
            .filter_map(|i| i.params.get("intent").cloned())
            .collect()
    }
}

fn expected_keys() -> BTreeSet<String> {
    ["audio-view", "waveform", SEEK_KEY, RULER_KEY, "meter"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

#[test]
fn mounted_components_match_between_fake_and_gpui() {
    let fake = Fake::new(spec());
    let gpui = Gpui::new(spec());
    assert_eq!(fake.keys(), expected_keys());
    assert_eq!(gpui.keys(), fake.keys());
}

#[test]
fn projections_match_before_any_gesture() {
    let fake = Fake::new(spec());
    let gpui = Gpui::new(spec());
    assert_eq!(gpui.projections(), fake.projections());
}

#[test]
fn seek_gesture_intents_and_projections_match() {
    let mut fake = Fake::new(spec());
    let mut gpui = Gpui::new(spec());

    let fake_intents = fake.gesture(SEEK_PLAIN);
    let gpui_intents = gpui.gesture(SEEK_PLAIN);

    assert!(
        fake_intents
            .iter()
            .any(|i| { i["kind"] == "commit" && i["target"] == SEEK_TARGET }),
        "fake must commit the seek target: {fake_intents:?}"
    );
    assert_eq!(gpui_intents, fake_intents);
    assert_eq!(gpui.projections(), fake.projections());

    let seek = &gpui.projections()[SEEK_KEY];
    assert_eq!(
        seek.committed.map(|(plain, _)| f64::from_bits(plain)),
        Some(SEEK_PLAIN)
    );
}

/// viewport 内の相対位置。contract の描画規則(`at - start` を span で割る、
/// span は 1 以上)をそのまま写す。
fn viewport_x(at: f64, viewport: &Range<u64>) -> f64 {
    let span = (viewport.end - viewport.start).max(1) as f64;
    (at - viewport.start as f64) / span
}

fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}

/// `key` の component を Panel・Tabs の子を辿って探す。
fn find<'a>(node: &'a ComponentSpec, key: &str) -> Option<&'a ComponentSpec> {
    if node.key() == &ComponentKey::from(key) {
        return Some(node);
    }
    match node {
        ComponentSpec::Panel(panel) => panel.children.iter().find_map(|c| find(c, key)),
        ComponentSpec::Tabs(tabs) => tabs.tabs.iter().find_map(|t| find(&t.child, key)),
        _ => None,
    }
}

fn child<'a>(spec: &'a ViewSpec, key: &str) -> &'a ComponentSpec {
    find(&spec.root, key).expect("child component")
}

/// Meter の描画 bar の期待値。gap(`count == 0`・非有限 peak)、範囲外の bin は描かない。
/// `(channel, x0, x1, peak, rms)`。
fn expected_bars(m: &MeterSpec) -> Vec<(usize, f64, f64, f64, Option<f64>)> {
    m.channel_levels
        .iter()
        .enumerate()
        .flat_map(|(channel, bins)| {
            bins.iter().filter_map(move |b| {
                if b.count == 0 || !b.peak.is_finite() {
                    return None;
                }
                let f0 = viewport_x(b.start as f64, &m.viewport);
                let f1 = viewport_x(b.end as f64, &m.viewport);
                if f1 <= f0 || f1 <= 0.0 || f0 >= 1.0 {
                    return None;
                }
                let rms = b.rms.is_finite().then(|| b.rms.clamp(0.0, 1.0));
                Some((
                    channel,
                    f0.max(0.0),
                    f1.min(1.0),
                    b.peak.clamp(0.0, 1.0),
                    rms,
                ))
            })
        })
        .collect()
}

/// TimeAxis の期待 mark。viewport 外の tick は描かない。`(x, major, label)`。
fn expected_marks(t: &TimeAxisSpec) -> Vec<(f64, bool, &str)> {
    t.ticks
        .iter()
        .map(|k| (viewport_x(k.at, &t.viewport), k.major, k.label.as_str()))
        .filter(|(x, _, _)| (0.0..=1.0).contains(x))
        .collect()
}

/// Waveform の描画 bin(非 gap)の frame 範囲。
fn expected_bins(w: &WaveformSpec, channel: usize) -> Vec<Range<u64>> {
    w.channel_bins[channel]
        .iter()
        .filter(|b| b.count != 0 && b.min.is_finite() && b.max.is_finite())
        .map(|b| b.start..b.end)
        .collect()
}

fn assert_bars(bars: &[DrawnBar], m: &MeterSpec) {
    let want = expected_bars(m);
    assert!(!want.is_empty(), "fixture must draw meter bars");
    assert_eq!(bars.len(), want.len(), "drawn meter bar count");
    for (bar, (channel, x0, x1, peak, rms)) in bars.iter().zip(want) {
        assert_eq!(bar.channel, channel);
        assert_close(bar.x0, x0);
        assert_close(bar.x1, x1);
        assert_close(bar.peak, peak);
        match (bar.rms, rms) {
            (Some(actual), Some(expected)) => assert_close(actual, expected),
            (None, None) => {}
            other => panic!("rms mismatch: {other:?}"),
        }
    }
}

fn assert_marks(marks: &[DrawnMark], t: &TimeAxisSpec) {
    let want = expected_marks(t);
    assert!(!want.is_empty(), "fixture must draw tick marks");
    assert_eq!(marks.len(), want.len(), "drawn tick mark count");
    for (mark, (x, major, label)) in marks.iter().zip(want) {
        assert_close(mark.x, x);
        assert_eq!(mark.major, major);
        assert_eq!(mark.label, label);
    }
}

fn assert_waveform(channels: &[DrawnWaveformChannel], w: &WaveformSpec) {
    assert_eq!(channels.len(), w.channels, "drawn channel count");
    for (drawn, channel) in channels.iter().zip(0..w.channels) {
        assert_eq!(drawn.channel, channel);
        assert_eq!(
            drawn.bins,
            expected_bins(w, channel),
            "channel {channel} bins"
        );
    }
}

#[test]
fn drawn_plans_match_fake_expectations_for_meter_ruler_and_waveform() {
    let spec = spec();
    let gpui = Gpui::new(spec.clone());
    let drawn = gpui.observer.drawn(gpui.window).expect("drawn view");
    assert_eq!(drawn.window, gpui.window);

    let keys: Vec<&str> = drawn.components.iter().map(|c| c.key.0.as_str()).collect();
    assert_eq!(keys, [WAVEFORM_KEY, RULER_KEY, METER_KEY], "paint order");
    for component in &drawn.components {
        match (&component.plan, child(&spec, &component.key.0)) {
            (DrawnPlan::Waveform { channels }, ComponentSpec::Waveform(w)) => {
                assert_waveform(channels, w);
            }
            (DrawnPlan::TimeAxis { marks }, ComponentSpec::TimeAxis(t)) => assert_marks(marks, t),
            (DrawnPlan::Meter { bars }, ComponentSpec::Meter(m)) => assert_bars(bars, m),
            (plan, other) => panic!("{plan:?} drawn for {}", other.kind()),
        }
    }
}

#[test]
fn drawn_waveform_omits_the_nan_gap_range() {
    let gpui = Gpui::new(spec());
    let drawn = gpui.observer.drawn(gpui.window).expect("drawn view");
    let DrawnPlan::Waveform { channels } = &drawn.components[0].plan else {
        panic!("waveform is the first paint component");
    };
    let gap = GAP.start as u64..GAP.end as u64;
    let painted = &channels[0].bins;
    assert!(
        painted
            .iter()
            .all(|b| !(b.start >= gap.start && b.end <= gap.end)),
        "no painted bin lies entirely inside the gap {gap:?}: {painted:?}"
    );
    assert!(
        painted.iter().any(|b| b.start < gap.end && b.end > gap.end),
        "a bin straddling the gap end is painted (it holds finite samples)"
    );
}

/// Spectrogram 画面の fixture。8 frame × 4 bin。frame 2・3 は全 bin が NaN
/// (gap 列)、frame 4・5 の bin 1 は NaN(gap 列ではない非有限セル)。
fn spectrogram_view() -> ViewSpec {
    let mut magnitudes: Vec<f64> = (0..32).map(|i| (i % 4) as f64 + 1.0).collect();
    magnitudes[8..16].fill(f64::NAN);
    magnitudes[17] = f64::NAN;
    magnitudes[21] = f64::NAN;
    let info = SpectrumInfo {
        frames: 8,
        bins: 4,
        grid: FrameGrid::new(8, 4, 0).expect("grid"),
        axis: BinAxis::Uniform {
            first_hz: 0.0,
            step_hz: 1.0,
        },
        sample_rate: 48_000,
    };
    let source = VecSpectrum::new(info, SpectrumData::Magnitude(magnitudes)).expect("source");
    let tile = Spectrogram::new(Arc::new(source))
        .tile(&TileRequest {
            frames: 0..8,
            rows: RowSpec::uniform(4, 4),
            cols: 4,
            aggregate: Aggregate::Max,
        })
        .expect("tile");
    spectrogram_screen(&tile, 0.0..10.0, 8).expect("spectrogram screen")
}

/// Run 画面の fixture。`dataset` の duration と `locate` の error が欠損セル。
fn run_view() -> ViewSpec {
    let run = RunOutcome::from_json(&json!({
        "job_id": "job-1",
        "stages": [
            {"stage": "ingest", "status": "completed", "duration_ms": 12},
            {"stage": "locate", "status": "failed", "duration_ms": null, "error": "no match"},
            {"stage": "dataset", "status": "skipped", "duration_ms": null}
        ]
    }))
    .expect("run outcome");
    run_screen(&run, Vec::new()).expect("run screen")
}

/// job 一覧の fixture(一列の Table)。
fn jobs_view() -> ViewSpec {
    let job = |id: &str| JobRow {
        job_id: id.to_string(),
        inputs: Vec::new(),
        stages: Vec::new(),
        errors: Vec::new(),
        dataset_ready: true,
    };
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey::from("jobs-view"),
            title: None,
            children: vec![job_list(&[job("job-1"), job("job-2")]).expect("job list")],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

/// Table の期待格子。欠損セルは `None` のまま位置を保つ。
fn expected_grid(t: &TableSpec) -> Vec<Vec<Option<String>>> {
    t.rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| c.as_ref().map(ToString::to_string))
                .collect()
        })
        .collect()
}

/// Spectrogram の期待 cell `(x0, x1, y0, y1, intensity)`。gap 列(`count == 0`)・
/// viewport 外の列・非有限の magnitude は描かない。y は上から下へ数え、
/// row 0 が最下段に来る。
fn expected_cells(s: &SpectrogramSpec) -> Vec<(f64, f64, f64, f64, f64)> {
    let rows = s.rows as f64;
    let lo = s.magnitude_range.start;
    let span = s.magnitude_range.end - lo;
    let mut out = Vec::new();
    for column in s.columns.iter().filter(|c| c.count != 0) {
        let f0 = viewport_x(column.start as f64, &s.viewport);
        let f1 = viewport_x(column.end as f64, &s.viewport);
        if f1 <= f0 || f1 <= 0.0 || f0 >= 1.0 {
            continue;
        }
        for (row, &m) in column.magnitudes.iter().enumerate() {
            if !m.is_finite() {
                continue;
            }
            out.push((
                f0.max(0.0),
                f1.min(1.0),
                1.0 - (row + 1) as f64 / rows,
                1.0 - row as f64 / rows,
                ((m - lo) / span).clamp(0.0, 1.0),
            ));
        }
    }
    out
}

fn assert_cells(cells: &[DrawnCell], s: &SpectrogramSpec) {
    let want = expected_cells(s);
    assert!(!want.is_empty(), "fixture must draw spectrogram cells");
    assert!(
        want.len() < s.rows * s.columns.len(),
        "fixture must omit gap and non-finite cells"
    );
    assert_eq!(cells.len(), want.len(), "drawn spectrogram cell count");
    for (cell, (x0, x1, y0, y1, intensity)) in cells.iter().zip(want) {
        assert_close(cell.x0, x0);
        assert_close(cell.x1, x1);
        assert_close(cell.y0, y0);
        assert_close(cell.y1, y1);
        assert_close(cell.intensity, intensity);
    }
}

/// `view` を mount した窓の drawn plan を、同じ spec の payload から導いた
/// 期待値と照合し、描画順の key を返す。
fn assert_drawn_matches(view: &ViewSpec) -> Vec<String> {
    let gpui = Gpui::new(view.clone());
    let drawn = gpui.observer.drawn(gpui.window).expect("drawn view");
    for component in &drawn.components {
        match (&component.plan, child(view, &component.key.0)) {
            (DrawnPlan::Spectrogram { cells }, ComponentSpec::Spectrogram(s)) => {
                assert_cells(cells, s);
            }
            (DrawnPlan::Label { text }, ComponentSpec::Label(l)) => assert_eq!(text, &l.text),
            (DrawnPlan::Table { header, rows }, ComponentSpec::Table(t)) => {
                assert_eq!(header, &t.columns);
                assert_eq!(rows, &expected_grid(t));
            }
            (plan, other) => panic!("{plan:?} drawn for {}", other.kind()),
        }
    }
    drawn.components.iter().map(|c| c.key.0.clone()).collect()
}

#[test]
fn drawn_spectrogram_and_range_label_match_fake_expectations() {
    let keys = assert_drawn_matches(&spectrogram_view());
    assert_eq!(
        keys,
        [SPECTROGRAM_RANGE_KEY, SPECTROGRAM_KEY],
        "paint order"
    );
}

#[test]
fn drawn_run_status_and_stage_table_match_fake_expectations() {
    let view = run_view();
    let keys = assert_drawn_matches(&view);
    assert_eq!(keys, [RUN_STATUS_KEY, STAGES_KEY], "paint order");
    let ComponentSpec::Table(stages) = child(&view, STAGES_KEY) else {
        panic!("stages must be a table");
    };
    assert!(
        expected_grid(stages).iter().flatten().any(Option::is_none),
        "fixture must carry a missing cell"
    );
}

#[test]
fn drawn_job_list_matches_fake_expectations() {
    assert_eq!(assert_drawn_matches(&jobs_view()), [JOBS_KEY]);
}

#[test]
fn drawn_desktop_jobs_tab_matches_fake_expectations() {
    let dir = TempDir::new("al-gui-parity-desktop");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    let view = desktop_screen(Some(&base), Some(&job_id));
    assert_eq!(assert_drawn_matches(&view), [JOBS_KEY]);
}

fn drain_intents(host: &mut ViewHost) -> Vec<Value> {
    host.drain_events()
        .into_iter()
        .filter_map(|e| match e {
            ViewEvent::Intent { intent, .. } => Some(serde_json::to_value(intent).expect("json")),
            _ => None,
        })
        .collect()
}

#[test]
fn jobs_selection_and_resume_button_match_between_fake_and_gpui() {
    assert_eq!(COMMAND_OP, al_gui::command::COMMAND_INTENT_OP);
    let dir = TempDir::new("al-gui-parity-command");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    let view = desktop_screen(Some(&base), Some(&job_id));
    let jobs = ComponentKey::from(JOBS_KEY);
    let resume = ComponentKey::from(RESUME_KEY);

    let mut fake = Fake::new(view.clone());
    fake.host
        .select_row(fake.host.generation(), &jobs, 0)
        .expect("fake select");
    let fake_select = drain_intents(&mut fake.host);
    fake.host
        .invoke(fake.host.generation(), &resume)
        .expect("fake invoke");
    let fake_invoke = drain_intents(&mut fake.host);
    let fake_selected = fake.host.observe(&jobs).expect("jobs mounted").selected;

    let mut gpui = Gpui::new(view);
    gpui.observer
        .select_row(gpui.window, &jobs, 0)
        .expect("gpui select");
    gpui.observer.settle(|_, _| {});
    let gpui_select = gpui.command_intents();
    gpui.observer
        .invoke(gpui.window, &resume)
        .expect("gpui invoke");
    gpui.observer.settle(|_, _| {});
    let gpui_invoke = gpui.command_intents()[gpui_select.len()..].to_vec();
    let gpui_selected = gpui
        .observer
        .observe(gpui.window)
        .expect("observed view")
        .components
        .into_iter()
        .find(|c| c.key == jobs)
        .expect("jobs observed")
        .snapshot
        .selected;

    assert_eq!(fake_select.len(), 1, "select emits one intent");
    assert_eq!(fake_select[0]["kind"], "action");
    assert_eq!(fake_select[0]["target"], al_gui::command::JOB_TARGET);
    assert_eq!(fake_select[0]["action"], al_gui::command::ACTION_SELECT);
    assert_eq!(fake_select[0]["params"], json!({"row": 0}));
    assert_eq!(gpui_select, fake_select);

    assert_eq!(fake_invoke.len(), 1, "resume emits one intent");
    assert_eq!(fake_invoke[0]["action"], al_gui::command::ACTION_RESUME);
    assert_eq!(fake_invoke[0]["params"], json!({"job_id": job_id.as_str()}));
    assert_eq!(gpui_invoke, fake_invoke);

    assert_eq!(fake_selected, Some(0));
    assert_eq!(gpui_selected, fake_selected);
}

#[test]
fn tab_selection_intent_and_selected_tab_match_between_fake_and_gpui() {
    let dir = TempDir::new("al-gui-parity-tab-select");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    let view = desktop_screen(Some(&base), Some(&job_id));
    let tabs = ComponentKey::from(DESKTOP_KEY);

    let mut fake = Fake::new(view.clone());
    fake.host
        .select_tab(fake.host.generation(), &tabs, 1)
        .expect("fake select tab");
    let fake_intents = drain_intents(&mut fake.host);
    let fake_selected = fake.host.observe(&tabs).expect("tabs mounted").selected;

    let mut gpui = Gpui::new(view);
    gpui.observer
        .select_tab(gpui.window, &tabs, 1)
        .expect("gpui select tab");
    gpui.observer.settle(|_, _| {});
    let gpui_intents = gpui.command_intents();
    let gpui_selected = gpui
        .observer
        .observe(gpui.window)
        .expect("observed view")
        .components
        .into_iter()
        .find(|c| c.key == tabs)
        .expect("tabs observed")
        .snapshot
        .selected;

    assert_eq!(fake_intents.len(), 1, "a tab press emits one intent");
    assert_eq!(fake_intents[0]["target"], al_gui::command::DESKTOP_TARGET);
    assert_eq!(
        fake_intents[0]["action"],
        al_gui::command::ACTION_TAB_SELECT
    );
    assert_eq!(fake_intents[0]["params"], json!({"index": 1}));
    assert_eq!(gpui_intents, fake_intents);

    assert_eq!(fake_selected, Some(1));
    assert_eq!(gpui_selected, fake_selected);
}

#[test]
fn workspace_tab_selection_intent_and_selected_tab_match_between_fake_and_gpui() {
    let dir = TempDir::new("al-gui-parity-ws-tab-select");
    let base = dir.join("jobs");
    let job_id = make_job(&base, &dir.join("cap.wav"));
    let view = desktop_screen(Some(&base), Some(&job_id));
    let root = ComponentKey::from(DESKTOP_KEY);
    let workspace = ComponentKey::from(WORKSPACE_KEY);

    // Dataset tab を表示してから workspace の Spectrogram tab を選ぶ、
    // 実際の pointer 操作に相当する流れ。
    let mut fake = Fake::new(view.clone());
    fake.host
        .select_tab(fake.host.generation(), &root, 2)
        .expect("fake select the dataset tab");
    let fake_root_intents = drain_intents(&mut fake.host);
    fake.host
        .select_tab(fake.host.generation(), &workspace, 1)
        .expect("fake select the spectrogram tab");
    let fake_ws_intents = drain_intents(&mut fake.host);
    let fake_ws_selected = fake
        .host
        .observe(&workspace)
        .expect("workspace mounted")
        .selected;

    let mut gpui = Gpui::new(view);
    gpui.observer
        .select_tab(gpui.window, &root, 2)
        .expect("gpui select the dataset tab");
    gpui.observer.settle(|_, _| {});
    let gpui_root_intents = gpui.command_intents();
    gpui.observer
        .select_tab(gpui.window, &workspace, 1)
        .expect("gpui select the spectrogram tab");
    gpui.observer.settle(|_, _| {});
    let gpui_ws_intents = gpui.command_intents()[gpui_root_intents.len()..].to_vec();
    let gpui_ws_selected = gpui
        .observer
        .observe(gpui.window)
        .expect("observed view")
        .components
        .into_iter()
        .find(|c| c.key == workspace)
        .expect("workspace observed")
        .snapshot
        .selected;

    assert_eq!(
        fake_root_intents.len(),
        1,
        "the dataset tab press emits one intent"
    );
    assert_eq!(
        fake_root_intents[0]["target"],
        al_gui::command::DESKTOP_TARGET
    );
    assert_eq!(gpui_root_intents, fake_root_intents);

    assert_eq!(
        fake_ws_intents.len(),
        1,
        "a workspace tab press emits one intent"
    );
    assert_eq!(
        fake_ws_intents[0]["target"],
        al_gui::command::WORKSPACE_TARGET
    );
    assert_eq!(
        fake_ws_intents[0]["action"],
        al_gui::command::ACTION_TAB_SELECT
    );
    assert_eq!(fake_ws_intents[0]["params"], json!({"index": 1}));
    assert_eq!(gpui_ws_intents, fake_ws_intents);

    assert_eq!(fake_ws_selected, Some(1));
    assert_eq!(gpui_ws_selected, fake_ws_selected);
}
