//! AL-G3 GPUI parity: Source 画面の同一 `ViewSpec` を Fake 層(`ViewHost`
//! 直接評価)と GPUI Driver(`GpuiObserver` 経由の mount)で評価し、
//! mount 済みの key・control の projection・intent が一致することを検証する。
//!
//! GPUI 側は consumer 観測 seam(`ruviform-ui-gpui` の `GpuiObserver`)
//! だけを使い、GPUI 型は持たない。Meter/TimeAxis の描画 plan は seam に
//! 公開されていないため対象外(NotRun)。

#![cfg(feature = "gpui-test")]

mod common;

use std::collections::{BTreeMap, BTreeSet};

use al_gui::view::screen::{RULER_KEY, SEEK_KEY, SEEK_TARGET, source_screen};
use common::fixture_view;
use ruviform_ui::behavior::{CommandPhase, ControlInput, InputSource, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, AttachEpochAllocator, ComponentKey, ComponentSnapshot, ViewEvent, ViewHost,
    ViewSpec,
};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{GpuiObserver, ViewMount};
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
