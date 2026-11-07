//! AL-G3: Waveform/Seek 画面の `ViewSpec` 射影を Fake 層(`ViewHost`
//! headless projection)で評価する。Theme・Source・View・Interaction
//! の同一 fixture を使い、GPUI 側は composition root からのみ接続する。

use std::sync::Arc;

use al_gui::theme;
use al_gui::view::screen::{SEEK_KEY, SEEK_TARGET, WAVEFORM_KEY, waveform_seek};
use al_gui::view::source::WaveformView;
use ruviform_ui::behavior::{CommandIntent, ControlInput, InputSource, ValueInput};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, MAX_WAVEFORM_BINS, ProjectionOp, ViewEvent, ViewHost,
    ViewSpec, WaveformSpec,
};
use ruviform_viz::{SliceWaveform, WaveformSource};

const SAMPLE_RATE: u32 = 48_000;
/// 0.1 秒。`[1200, 1440)` frame を NaN gap にする。
const FRAMES: usize = 4_800;
const GAP: std::ops::Range<usize> = 1_200..1_440;

fn fixture_view() -> WaveformView {
    let samples: Vec<f64> = (0..FRAMES)
        .map(|i| {
            if GAP.contains(&i) {
                f64::NAN
            } else {
                (i as f64 * 0.01).sin() * 0.75
            }
        })
        .collect();
    let source: Arc<dyn WaveformSource> =
        Arc::new(SliceWaveform::new(SAMPLE_RATE, 1, samples).expect("slice waveform"));
    WaveformView::try_from_source(source).expect("waveform view")
}

fn waveform_of(spec: &ViewSpec) -> &WaveformSpec {
    let ComponentSpec::Panel(panel) = &spec.root else {
        panic!("root must be the screen panel");
    };
    match &panel.children[0] {
        ComponentSpec::Waveform(w) => w,
        other => panic!("first child must be waveform, got {}", other.kind()),
    }
}

fn host(spec: ViewSpec) -> ViewHost {
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpoch::new(1),
    );
    let ops = host.mount(spec).expect("mount");
    for k in ["audio-view", WAVEFORM_KEY, SEEK_KEY] {
        assert!(
            ops.contains(&ProjectionOp::Mounted {
                key: ComponentKey::from(k)
            }),
            "{k} not mounted"
        );
    }
    host.drain_events();
    host
}

fn seek_gesture(host: &mut ViewHost, plain: f64) -> Vec<ViewEvent> {
    let generation = host.generation();
    let key = ComponentKey::from(SEEK_KEY);
    host.dispatch_input(
        generation,
        &key,
        ControlInput::Begin {
            gesture_id: "s1".into(),
            source: InputSource::from("pointer"),
            origin_view: None,
            target: None,
            at: None,
            modifiers: None,
            seq: None,
        },
    )
    .expect("begin");
    host.dispatch_input(
        generation,
        &key,
        ControlInput::Preview {
            gesture_id: "s1".into(),
            value: ValueInput::Plain(plain),
            modifiers: None,
            seq: None,
        },
    )
    .expect("preview");
    host.dispatch_input(
        generation,
        &key,
        ControlInput::Commit {
            gesture_id: "s1".into(),
            value: None,
            modifiers: None,
            seq: None,
        },
    )
    .expect("commit");
    host.drain_events()
}

#[test]
fn projected_bins_are_bit_exact_to_resolved_view_bins() {
    let view = fixture_view();
    let spec = waveform_seek(&view, 0, 0.0, 0.1, 16).expect("spec");
    let wave = waveform_of(&spec);
    let resolved = view.window(0, 0.0, 0.1, 16).expect("window");
    assert_eq!(wave.channel_bins[0].len(), resolved.len());
    for (projected, source) in wave.channel_bins[0].iter().zip(&resolved) {
        assert_eq!(projected.start, source.start);
        assert_eq!(projected.end, source.end);
        assert_eq!(projected.count, source.bucket.count);
        assert_eq!(projected.min.to_bits(), source.bucket.min.to_bits());
        assert_eq!(projected.max.to_bits(), source.bucket.max.to_bits());
        assert_eq!(projected.rms.to_bits(), source.bucket.rms.to_bits());
    }
}

#[test]
fn gap_bins_stay_nan_with_zero_count() {
    let view = fixture_view();
    let spec = waveform_seek(&view, 0, 0.0, 0.1, 64).expect("spec");
    let wave = waveform_of(&spec);
    let gaps: Vec<_> = wave.channel_bins[0]
        .iter()
        .filter(|b| b.count == 0)
        .collect();
    assert!(!gaps.is_empty(), "fixture gap must surface as empty bins");
    for b in gaps {
        assert!(
            b.min.is_nan() && b.max.is_nan() && b.rms.is_nan(),
            "gap bin {b:?}"
        );
    }
}

#[test]
fn viewport_and_seek_range_come_from_source_frames() {
    let view = fixture_view();
    let spec = waveform_seek(&view, 0, 0.0, 0.1, 16).expect("spec");
    let wave = waveform_of(&spec);
    assert_eq!(wave.frames, FRAMES as u64);
    assert_eq!(wave.sample_rate, SAMPLE_RATE);
    assert_eq!(wave.viewport, 0..FRAMES as u64);
    let ComponentSpec::Panel(panel) = &spec.root else {
        unreachable!()
    };
    let seek = panel.children[1].control().expect("seek control");
    assert_eq!(seek.target.as_deref(), Some(SEEK_TARGET));
    assert_eq!(seek.descriptor.min, 0.0);
    assert_eq!(seek.descriptor.max, FRAMES as f64 / f64::from(SAMPLE_RATE));
}

#[test]
fn zero_width_window_keeps_bins_inside_viewport() {
    let view = fixture_view();
    let spec = waveform_seek(&view, 0, 0.05, 0.05, 16).expect("spec");
    let wave = waveform_of(&spec);
    assert!(
        !wave.viewport.is_empty(),
        "viewport must cover the read frame"
    );
    assert_eq!(wave.viewport, view.read_frames(0.05, 0.05));
    assert!(!wave.channel_bins[0].is_empty());
    for b in &wave.channel_bins[0] {
        assert!(
            wave.viewport.start <= b.start && b.end <= wave.viewport.end,
            "bin {}..{} outside viewport {:?}",
            b.start,
            b.end,
            wave.viewport
        );
    }
}

#[test]
fn bins_beyond_limit_are_rejected_before_reading() {
    let view = fixture_view();
    assert!(waveform_seek(&view, 0, 0.0, 0.1, MAX_WAVEFORM_BINS + 1).is_err());
    assert!(waveform_seek(&view, 0, 0.0, 0.1, MAX_WAVEFORM_BINS).is_ok());
}

#[test]
fn seek_gesture_commits_transport_intent_within_source_range() {
    let view = fixture_view();
    let spec = waveform_seek(&view, 0, 0.0, 0.1, 16).expect("spec");
    let mut host = host(spec);
    let events = seek_gesture(&mut host, 0.05);
    let intents: Vec<CommandIntent> = events
        .into_iter()
        .filter_map(|e| match e {
            ViewEvent::Intent { intent, .. } => Some(intent),
            _ => None,
        })
        .collect();
    assert!(
        intents.iter().any(|i| matches!(
            i,
            CommandIntent::Commit { target, .. } if target == SEEK_TARGET
        )),
        "expected transport.seek commit, got {intents:?}"
    );
    let snap = host
        .observe(&ComponentKey::from(SEEK_KEY))
        .expect("seek snapshot");
    assert_eq!(snap.committed.map(|v| v.plain), Some(0.05));
}

#[test]
fn product_theme_modes_do_not_change_semantic_event_stream() {
    let view = fixture_view();
    let mut streams = Vec::new();
    for mode in theme::MODES {
        let resolved = theme::resolve(mode).unwrap_or_else(|e| panic!("resolve {mode}: {e}"));
        assert!(
            resolved.color("color.waveform").is_some(),
            "{mode} lacks waveform"
        );
        let spec = waveform_seek(&view, 0, 0.0, 0.1, 16).expect("spec");
        let mut host = host(spec);
        streams.push(format!("{:?}", seek_gesture(&mut host, 0.05)));
    }
    assert!(!streams[0].is_empty());
    assert_eq!(
        streams[0], streams[1],
        "light/dark must yield identical semantics"
    );
}

#[test]
fn channel_out_of_range_is_rejected() {
    let view = fixture_view();
    assert!(waveform_seek(&view, 1, 0.0, 0.1, 16).is_err());
}
