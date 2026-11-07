//! AL-G3/AL-G4 GPUI parity: Source 画面の同一 `ViewSpec` を Fake 層(`ViewHost`
//! 直接評価)と GPUI Driver(`GpuiObserver` 経由の mount)で評価し、
//! mount 済みの key・control の projection・intent が一致することを検証する。
//!
//! GPUI 側は consumer 観測 seam(`ruviform-ui-gpui` の `GpuiObserver`)
//! だけを使い、GPUI 型は持たない。描画 plan は `GpuiObserver::drawn` で
//! Meter・TimeAxis・Waveform を観測し、mount した spec の payload から導いた
//! Fake 層の期待値と照合する。Spectrogram・Label・Table の描画 plan は seam
//! (`DrawnPlan`)に無いため対象外(NotRun、ruviform#436 の follow-up)。

#![cfg(feature = "gpui-test")]

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use al_gui::view::screen::{
    METER_KEY, RULER_KEY, SEEK_KEY, SEEK_TARGET, WAVEFORM_KEY, source_screen,
};
use common::{GAP, fixture_view};
use ruviform_ui::behavior::{CommandPhase, ControlInput, InputSource, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, AttachEpochAllocator, ComponentKey, ComponentSnapshot, ComponentSpec, MeterSpec,
    TimeAxisSpec, ViewEvent, ViewHost, ViewSpec, WaveformSpec,
};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{
    DrawnBar, DrawnMark, DrawnPlan, DrawnWaveformChannel, GpuiObserver, ViewMount,
};
use serde_json::Value;

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

fn child<'a>(spec: &'a ViewSpec, key: &str) -> &'a ComponentSpec {
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("root must be a panel");
    };
    panel
        .children
        .iter()
        .find(|c| c.key() == &ComponentKey::from(key))
        .expect("child component")
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
