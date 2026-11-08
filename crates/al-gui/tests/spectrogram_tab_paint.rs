//! AL#180 回帰: Spectrogram tab 選択時の UI thread hang。
//!
//! ruviform pin `a5b1db1` では spectrogram の paint canvas が 0 px 高さに
//! なり(RF#453 の既知不具合)、`MAX_SPECTROGRAM_CELLS` 上限いっぱいの
//! 65,536 cell が全て同一位置の重なり quad として gpui `BoundsTree` の
//! O(n²) 挿入を引き起こし、初回 paint で UI thread が復帰しなかった。
//! pin `863a7a9d` は RF#453(canvas 0 サイズ修正)と RF#461(spec サイズ
//! 比例 canvas painter の `paint_layer` batch 化)の両方を含み、
//! 65,536 cell の paint は debug build でも数十 ms で完了する。
//!
//! 本テストは workspace と同じ selectable Tabs 構造に本番解像度
//! (512 cols × 128 rows)の `spectrogram_screen` を載せ、tab 選択 →
//! `settle`(実 layout/paint)→ `drawn` で paint 完了を検証する。
//! pin が退行すると `settle` が戻らずテストが hang する。

#![cfg(feature = "gpui-test")]

use std::sync::Arc;

use al_gui::view::screen::{SPECTROGRAM_KEY, spectrogram_screen};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState, PanelSpec,
    TabSpec, TabsSelectionSpec, TabsSpec, ViewHost, ViewSpec,
};
use ruviform_ui::{WindowId, WindowSpec};
use ruviform_ui_gpui::{GpuiObserver, ViewMount};
use ruviform_viz::{
    Aggregate, BinAxis, FrameGrid, RowSpec, Spectrogram, SpectrumData, SpectrumInfo, TileRequest,
    VecSpectrum,
};

/// workspace の Tabs key・tab 数と同じ構造の selectable Tabs に、
/// 本番 `workspace_screen` と同じ解像度 (512 cols × 128 rows、
/// `SPECTROGRAM_MAX_FRAMES` = 4096 frame 分の tile) の spectrogram を載せる。
const WORKSPACE_KEY: &str = "workspace";
const SPECTROGRAM_TAB: usize = 1;

fn label_panel(key: &str, text: &str) -> ComponentSpec {
    ComponentSpec::Panel(PanelSpec {
        key: ComponentKey::from(key),
        title: None,
        children: vec![ComponentSpec::Label(ruviform_ui::view::LabelSpec {
            key: ComponentKey(format!("{key}-label")),
            text: text.to_string(),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        })],
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    })
}

/// 全 frame・全 bin が有限値の spectrogram を作り、本番と同じ
/// `TileRequest`(frames 0..4096、rows 128、cols 512、Aggregate::Max)
/// で tile → `spectrogram_screen` まで通す。空列が無いため
/// `spectrogram_cell_plan` は 65,536 cell 全てを paint 対象にする。
fn production_spectrogram_screen() -> ViewSpec {
    const FRAMES: usize = 8_192;
    const BINS: usize = 256;
    let info = SpectrumInfo {
        frames: FRAMES as u64,
        bins: BINS,
        grid: FrameGrid::new(256, 128, 0).expect("grid"),
        axis: BinAxis::Uniform {
            first_hz: 0.0,
            step_hz: 1.0,
        },
        sample_rate: 48_000,
    };
    let magnitudes: Vec<f64> = (0..FRAMES * BINS)
        .map(|i| ((i % 977) as f64) / 977.0)
        .collect();
    let source =
        Arc::new(VecSpectrum::new(info, SpectrumData::Magnitude(magnitudes)).expect("source"));
    let spectrogram = Spectrogram::new(source);
    let tile = spectrogram
        .tile(&TileRequest {
            frames: 0..4096,
            rows: RowSpec::uniform(BINS, 128),
            cols: 512,
            aggregate: Aggregate::Max,
        })
        .expect("tile");
    spectrogram_screen(&tile, 0.0..1.0, FRAMES as u64).expect("screen")
}

fn workspace_like_spec() -> ViewSpec {
    let spectrogram = production_spectrogram_screen();
    ViewSpec {
        root: ComponentSpec::Tabs(TabsSpec {
            key: ComponentKey::from(WORKSPACE_KEY),
            active: 0,
            tabs: vec![
                TabSpec {
                    key: ComponentKey::from("tab-source"),
                    title: "Source".to_string(),
                    child: Box::new(label_panel("source-view", "source")),
                },
                TabSpec {
                    key: ComponentKey::from("tab-spectrogram"),
                    title: "Spectrogram".to_string(),
                    child: Box::new(spectrogram.root),
                },
                TabSpec {
                    key: ComponentKey::from("tab-analysis"),
                    title: "Analysis".to_string(),
                    child: Box::new(label_panel("analysis-view", "analysis")),
                },
            ],
            selection: Some(TabsSelectionSpec {
                target: "al.workspace".to_string(),
            }),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn mount(spec: ViewSpec) -> (GpuiObserver, WindowId) {
    let epoch = AttachEpochAllocator::new().mint();
    let mut host = ViewHost::new(ViewInstanceId::new(7).expect("nonzero"), epoch);
    host.mount(spec).expect("mount");
    host.drain_events();
    let mut observer = GpuiObserver::new();
    let window = observer
        .driver_mut()
        .create_view_window(&WindowSpec::default(), ViewMount { host, theme: None })
        .expect("create_view_window");
    (observer, window)
}

/// Spectrogram tab を選択しても `settle` が戻り、spectrogram が
/// paint 対象へ遷移すること。pin 退行時は settle が hang する。
#[test]
fn select_spectrogram_tab_paint_completes() {
    let (mut observer, window) = mount(workspace_like_spec());
    observer.settle(|_, _| {});

    observer
        .select_tab(window, &ComponentKey::from(WORKSPACE_KEY), SPECTROGRAM_TAB)
        .expect("select_tab");
    observer.settle(|_, _| {});

    let drawn = observer.drawn(window).expect("drawn view");
    let keys: Vec<&str> = drawn.components.iter().map(|c| c.key.0.as_str()).collect();
    assert!(
        keys.contains(&SPECTROGRAM_KEY),
        "spectrogram must be painted after tab selection: {keys:?}"
    );
    let cells = drawn
        .components
        .iter()
        .find_map(|c| match &c.plan {
            ruviform_ui_gpui::DrawnPlan::Spectrogram { cells } => Some(cells.len()),
            _ => None,
        })
        .expect("spectrogram drawn plan");
    assert_eq!(
        cells,
        512 * 128,
        "production resolution must be fully painted"
    );
}
