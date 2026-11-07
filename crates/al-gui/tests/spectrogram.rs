//! AL-G4: Spectrogram 画面の `ViewSpec` 射影。`ruviform-viz` の解決済み
//! `Tile` を `SpectrogramSpec` へ 1 対 1 で写し、列の frame 区間・gap・
//! 上限を Fake 層(`ViewHost`)の mount 検証で確かめる。

use std::sync::Arc;

use al_gui::view::screen::{SPECTROGRAM_KEY, spectrogram_screen};
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    AttachEpoch, ComponentKey, ComponentSpec, SpectrogramSpec, ViewHost, ViewSpec,
};
use ruviform_viz::{
    Aggregate, BinAxis, FrameGrid, RowSpec, Spectrogram, SpectrumData, SpectrumInfo, Tile,
    TileRequest, VecSpectrum,
};

/// `frames × bins` の magnitude を持つ `Spectrogram` を作る。
fn spectrogram(frames: u64, bins: usize, magnitudes: Vec<f64>) -> Spectrogram {
    let info = SpectrumInfo {
        frames,
        bins,
        grid: FrameGrid::new(8, 4, 0).expect("grid"),
        axis: BinAxis::Uniform {
            first_hz: 0.0,
            step_hz: 1.0,
        },
        sample_rate: 48_000,
    };
    let source = VecSpectrum::new(info, SpectrumData::Magnitude(magnitudes)).expect("source");
    Spectrogram::new(Arc::new(source))
}

fn tile(spec: &Spectrogram, frames: std::ops::Range<u64>, rows: usize, cols: usize) -> Tile {
    let bins = spec.info().bins;
    spec.tile(&TileRequest {
        frames,
        rows: RowSpec::uniform(bins, rows),
        cols,
        aggregate: Aggregate::Max,
    })
    .expect("tile")
}

fn spectrogram_of(view: &ViewSpec) -> &SpectrogramSpec {
    let ComponentSpec::Panel(panel) = &view.root else {
        panic!("root must be the spectrogram panel");
    };
    match &panel.children[1] {
        ComponentSpec::Spectrogram(s) => s,
        other => panic!("second child must be spectrogram, got {}", other.kind()),
    }
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// 8 frame × 4 bin、frame 2・3 は全 bin が NaN(gap)。
fn gap_fixture() -> Spectrogram {
    let mut m: Vec<f64> = (0..32).map(|i| (i % 4) as f64 + 1.0).collect();
    for v in &mut m[2 * 4..4 * 4] {
        *v = f64::NAN;
    }
    spectrogram(8, 4, m)
}

#[test]
fn columns_map_one_to_one_to_tile_values_and_frame_spans() {
    let t = tile(&gap_fixture(), 0..8, 2, 8);
    let view = spectrogram_screen(&t, 0.0..8.0, 8).expect("screen");
    let s = spectrogram_of(&view);

    assert_eq!(s.rows, 2);
    assert_eq!(s.viewport, 0..8);
    assert_eq!(s.magnitude_range, 0.0..8.0);
    assert_eq!(s.columns.len(), 8);
    for (c, column) in s.columns.iter().enumerate() {
        assert_eq!((column.start, column.end), (c as u64, c as u64 + 1));
        let want: Vec<f64> = (0..t.rows).map(|r| t.values[r * t.cols + c]).collect();
        assert_eq!(bits(&column.magnitudes), bits(&want), "column {c}");
    }
}

#[test]
fn all_nan_columns_keep_count_zero_and_nan_cells() {
    let t = tile(&gap_fixture(), 0..8, 2, 8);
    let view = spectrogram_screen(&t, 0.0..8.0, 8).expect("screen");
    let s = spectrogram_of(&view);

    let counts: Vec<u64> = s.columns.iter().map(|c| c.count).collect();
    assert_eq!(counts, [2, 2, 0, 0, 2, 2, 2, 2]);
    for c in [2, 3] {
        assert!(s.columns[c].magnitudes.iter().all(|m| m.is_nan()));
    }
}

#[test]
fn columns_split_span_like_ruviform_tile_when_not_divisible() {
    let spec = spectrogram(10, 2, (0..20).map(f64::from).collect());
    let t = tile(&spec, 0..10, 1, 4);
    let view = spectrogram_screen(&t, 0.0..20.0, 10).expect("screen");
    let spans: Vec<_> = spectrogram_of(&view)
        .columns
        .iter()
        .map(|c| c.start..c.end)
        .collect();
    assert_eq!(spans, [0..2, 2..5, 5..7, 7..10]);
}

#[test]
fn offset_request_shifts_column_frames_to_absolute_frames() {
    let spec = spectrogram(10, 2, (0..20).map(f64::from).collect());
    let t = tile(&spec, 4..8, 1, 2);
    let view = spectrogram_screen(&t, 0.0..20.0, 10).expect("screen");
    let s = spectrogram_of(&view);
    assert_eq!(s.viewport, 4..8);
    let spans: Vec<_> = s.columns.iter().map(|c| c.start..c.end).collect();
    assert_eq!(spans, [4..6, 6..8]);
}

#[test]
fn mounts_in_fake_host_and_observes_spectrogram_key() {
    let t = tile(&gap_fixture(), 0..8, 2, 8);
    let view = spectrogram_screen(&t, 0.0..8.0, 8).expect("screen");
    let mut host = ViewHost::new(
        ViewInstanceId::new(1).expect("nonzero"),
        AttachEpoch::new(1),
    );
    host.mount(view).expect("mount");
    let snap = host
        .observe(&ComponentKey::from(SPECTROGRAM_KEY))
        .expect("spectrogram mounted");
    assert!(snap.mounted);
}

#[test]
fn rejects_tile_whose_values_do_not_match_its_shape() {
    let mut t = tile(&gap_fixture(), 0..8, 2, 8);
    t.values.pop();
    assert!(spectrogram_screen(&t, 0.0..8.0, 8).is_err());
}
