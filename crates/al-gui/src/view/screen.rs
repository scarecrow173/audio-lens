//! Source 画面(Waveform/Seek/TimeAxis/Meter)と Spectrogram 画面の
//! `ViewSpec` 射影 (AL-G3/AL-G4)。
//!
//! [`WaveformView`] が返す解決済み `ViewBin` を RF#387 の
//! `WaveformSpec`/`MeterSpec` payload へ 1 対 1 で写す。bin の統計・
//! gap(`NaN`/`count == 0`)は変換せず、描画・操作の意味論は
//! ruviform-ui の `ViewHost`(driver-neutral)へ委譲する。GPUI 型は
//! この層に現れない。
//!
//! Meter の `peak` は同じ bin の `min`/`max` から `max(|min|, |max|)`
//! で導く唯一の写像で、他の値は再計算しない。TimeAxis の tick は
//! 表示 viewport の秒範囲に対する刻みのみを決め、解析値には触れない。
//!
//! Spectrogram は `ruviform-viz` の解決済み [`Tile`] を列ごとに写す。
//! 列の frame 区間・有限セル数(`count`)・magnitude は [`Tile::columns`]
//! の規則をそのまま使い、値・gap は再計算しない。

use std::ops::Range;

use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
use ruviform_ui::view::{
    A11yRole, A11ySpec, AxisTick, ComponentKey, ComponentSpec, ComponentVisualState, ControlSpec,
    LabelSpec, MAX_METER_BINS, MeterBin, MeterSpec, Orientation, PanelSpec, SpectrogramColumn,
    SpectrogramSpec, TimeAxisSpec, ViewSpec, WaveformBin, WaveformSpec,
};
use ruviform_viz::{Tile, ViewBin};

use super::ViewError;
use super::source::WaveformView;

/// 画面 Panel の key。
pub const PANEL_KEY: &str = "audio-view";
/// Waveform component の key。
pub const WAVEFORM_KEY: &str = "waveform";
/// Seek component の key。
pub const SEEK_KEY: &str = "seek";
/// TimeAxis component の key。
pub const RULER_KEY: &str = "ruler";
/// Meter component の key。
pub const METER_KEY: &str = "meter";
/// Seek が発行する command target(`CommandIntent::Commit` の target)。
pub const SEEK_TARGET: &str = "transport.seek";
/// Spectrogram 画面 Panel の key。
pub const SPECTROGRAM_PANEL_KEY: &str = "spectrogram-view";
/// Spectrogram component の key。
pub const SPECTROGRAM_KEY: &str = "spectrogram";
/// 表示 frame 範囲を示す Label の key。
pub const SPECTROGRAM_RANGE_KEY: &str = "spectrogram-range";

/// tick 刻みの候補 `(step_s, decimals)`。小さい順。最大刻みの 10 区間分まで
/// (約 10 日)の span を扱える。
const TICK_STEPS: [(f64, usize); 23] = [
    (0.001, 3),
    (0.002, 3),
    (0.005, 3),
    (0.01, 2),
    (0.02, 2),
    (0.05, 2),
    (0.1, 1),
    (0.2, 1),
    (0.5, 1),
    (1.0, 0),
    (2.0, 0),
    (5.0, 0),
    (10.0, 0),
    (30.0, 0),
    (60.0, 0),
    (300.0, 0),
    (600.0, 0),
    (1800.0, 0),
    (3600.0, 0),
    (7200.0, 0),
    (14400.0, 0),
    (28800.0, 0),
    (86400.0, 0),
];
/// 表示 span を分割する区間数の上限(刻みは区間数がこれ以下に収まる最小)。
const MAX_TICK_INTERVALS: f64 = 10.0;
/// 大きい刻みの major tick 間隔(tick index の剰余)。
const MAJOR_EVERY: u64 = 5;

/// `channel` の `[start_s, end_s)` を最大 `bins` 個の bin へ集約し、
/// Waveform・Seek(source 全長)・TimeAxis・Meter を並べた Panel の
/// `ViewSpec` を返す。Waveform と Meter は同じ解決済み bin を共有する。
///
/// # Errors
/// 範囲外の channel、source の読み出しエラー、frame を持たない source、
/// または `bins` が `MAX_METER_BINS` を超える場合。
pub fn source_screen(
    source: &WaveformView,
    channel: usize,
    start_s: f64,
    end_s: f64,
    bins: usize,
) -> Result<ViewSpec, ViewError> {
    if source.frames == 0 {
        return Err(ViewError::Invalid("waveform source has no frames".into()));
    }
    // Meter は Waveform と同じ bin を共有するため、より小さい上限を画面全体の上限とする。
    if bins > MAX_METER_BINS {
        return Err(ViewError::Invalid(format!(
            "screen bins {bins} exceed {MAX_METER_BINS}"
        )));
    }
    let viewport = source.read_frames(start_s, end_s);
    let duration_s = source.frames as f64 / f64::from(source.sample_rate);
    let resolved = source.window(channel, start_s, end_s, bins)?;
    let waveform = ComponentSpec::Waveform(WaveformSpec {
        key: ComponentKey::from(WAVEFORM_KEY),
        sample_rate: source.sample_rate,
        channels: 1,
        frames: source.frames,
        viewport: viewport.clone(),
        channel_bins: vec![resolved.iter().map(waveform_bin).collect()],
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Img),
            label: Some("Waveform".to_string()),
            ..A11ySpec::default()
        },
    });
    let seek = ComponentSpec::Seek(ControlSpec {
        key: ComponentKey::from(SEEK_KEY),
        control_id: None,
        target: Some(SEEK_TARGET.to_string()),
        descriptor: ValueDescriptor::ranged(0.0, duration_s),
        orientation: Orientation::Horizontal,
        detach_policy: DetachPolicy::Cancel,
        unsupported: false,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Slider),
            label: Some("Seek".to_string()),
            orientation: Some(Orientation::Horizontal),
            value_min: Some(0.0),
            value_max: Some(duration_s),
            value_text: None,
        },
    });
    let ruler = ComponentSpec::TimeAxis(TimeAxisSpec {
        key: ComponentKey::from(RULER_KEY),
        viewport: viewport.clone(),
        ticks: ruler_ticks(source.sample_rate, &viewport)?,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Img),
            label: Some("timeline".to_string()),
            ..A11ySpec::default()
        },
    });
    let meter = ComponentSpec::Meter(MeterSpec {
        key: ComponentKey::from(METER_KEY),
        channels: 1,
        channel_levels: vec![resolved.iter().map(meter_bin).collect()],
        viewport,
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Img),
            label: Some("level".to_string()),
            ..A11ySpec::default()
        },
    });
    Ok(ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey::from(PANEL_KEY),
            title: None,
            children: vec![waveform, seek, ruler, meter],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    })
}

/// 解決済み `Tile` を Spectrogram 画面(表示範囲の Label と Spectrogram)の
/// `ViewSpec` へ写す。`tile.frames` は STFT frame 単位の表示範囲で、
/// 列の frame 区間は [`Tile::columns`] が定める。
/// `magnitude_range` は consumer が決める表示範囲で、値は変換しない。
///
/// # Errors
/// `tile` の `values` 長が `rows × cols` と一致しない場合。
pub fn spectrogram_screen(
    tile: &Tile,
    magnitude_range: Range<f64>,
    total_frames: u64,
) -> Result<ViewSpec, ViewError> {
    if tile.rows == 0 || tile.cols == 0 || tile.values.len() != tile.rows * tile.cols {
        return Err(ViewError::Invalid(format!(
            "tile {}x{} does not match {} values",
            tile.rows,
            tile.cols,
            tile.values.len()
        )));
    }
    let range = ComponentSpec::Label(LabelSpec {
        key: ComponentKey::from(SPECTROGRAM_RANGE_KEY),
        text: format!(
            "frames {}..{} of {total_frames}",
            tile.frames.start, tile.frames.end
        ),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec::default(),
    });
    let spectrogram = ComponentSpec::Spectrogram(SpectrogramSpec {
        key: ComponentKey::from(SPECTROGRAM_KEY),
        viewport: tile.frames.clone(),
        rows: tile.rows,
        magnitude_range,
        columns: tile
            .columns()
            .map(|c| SpectrogramColumn {
                start: c.frames.start,
                end: c.frames.end,
                count: c.count,
                magnitudes: c.magnitudes,
            })
            .collect(),
        visual: ComponentVisualState::default(),
        a11y: A11ySpec {
            role: Some(A11yRole::Img),
            label: Some("Spectrogram".to_string()),
            ..A11ySpec::default()
        },
    });
    Ok(ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey::from(SPECTROGRAM_PANEL_KEY),
            title: None,
            children: vec![range, spectrogram],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    })
}

fn waveform_bin(b: &ViewBin) -> WaveformBin {
    WaveformBin {
        start: b.start,
        end: b.end,
        min: b.bucket.min,
        max: b.bucket.max,
        rms: b.bucket.rms,
        count: b.bucket.count,
    }
}

/// gap bin(`count == 0`)は peak も NaN のまま残す。
fn meter_bin(b: &ViewBin) -> MeterBin {
    let peak = if b.bucket.count == 0 {
        f64::NAN
    } else {
        b.bucket.min.abs().max(b.bucket.max.abs())
    };
    MeterBin {
        start: b.start,
        end: b.end,
        peak,
        rms: b.bucket.rms,
        count: b.bucket.count,
    }
}

/// viewport の範囲を区間数が `MAX_TICK_INTERVALS` 以下に収まる最小の刻みで
/// 区切る。tick 位置は viewport の frame 単位(絶対 frame)で、丸め後の
/// 位置が viewport の外に出るものは落とす。
///
/// # Errors
/// 表示 span が最大刻みでも `MAX_TICK_INTERVALS` に収まらない場合。
fn ruler_ticks(sample_rate: u32, viewport: &Range<u64>) -> Result<Vec<AxisTick>, ViewError> {
    let rate = f64::from(sample_rate);
    let (lo, hi) = (viewport.start as f64, viewport.end as f64);
    let span_s = (hi - lo) / rate;
    let (step, decimals) = TICK_STEPS
        .iter()
        .copied()
        .find(|(step, _)| span_s / step <= MAX_TICK_INTERVALS)
        .ok_or_else(|| ViewError::Invalid(format!("ruler span {span_s}s too large")))?;
    let stride = step * rate;
    let first = (lo / stride).floor() as u64;
    let last = (hi / stride).ceil() as u64;
    Ok((first..=last)
        .filter_map(|k| {
            let at = k as f64 * stride;
            (lo <= at && at <= hi).then(|| AxisTick {
                at,
                label: format!("{:.decimals$}", k as f64 * step),
                major: k % MAJOR_EVERY == 0,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_ui::view::MAX_TIME_AXIS_TICKS;

    #[test]
    fn long_span_gets_coarse_ticks_within_the_bound() {
        let rate = 48_000;
        let ticks = ruler_ticks(rate, &(0..u64::from(rate) * 3 * 3600)).expect("ticks");
        let labels: Vec<_> = ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(
            labels,
            ["0", "1800", "3600", "5400", "7200", "9000", "10800"]
        );
        assert!(ticks.len() <= MAX_TIME_AXIS_TICKS);
    }

    #[test]
    fn span_beyond_the_largest_step_is_rejected() {
        let rate = 48_000;
        assert!(ruler_ticks(rate, &(0..u64::from(rate) * 11 * 86_400)).is_err());
    }
}
