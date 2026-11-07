//! Waveform/Seek 画面の `ViewSpec` 射影 (AL-G3)。
//!
//! [`WaveformView`] が返す解決済み `ViewBin` を RF#387 の
//! `WaveformSpec` payload へ 1 対 1 で写す。bin の統計・gap
//! (`NaN`/`count == 0`)は変換せず、描画・操作の意味論は
//! ruviform-ui の `ViewHost`(driver-neutral)へ委譲する。GPUI 型は
//! この層に現れない。

use ruviform_ui::behavior::{DetachPolicy, ValueDescriptor};
use ruviform_ui::view::{
    A11yRole, A11ySpec, ComponentKey, ComponentSpec, ComponentVisualState, ControlSpec,
    Orientation, PanelSpec, ViewSpec, WaveformBin, WaveformSpec,
};

use super::ViewError;
use super::source::WaveformView;

/// 画面 Panel の key。
pub const PANEL_KEY: &str = "audio-view";
/// Waveform component の key。
pub const WAVEFORM_KEY: &str = "waveform";
/// Seek component の key。
pub const SEEK_KEY: &str = "seek";
/// Seek が発行する command target(`CommandIntent::Commit` の target)。
pub const SEEK_TARGET: &str = "transport.seek";

/// `channel` の `[start_s, end_s)` を最大 `bins` 個の Waveform bin へ
/// 集約し、source 全長 `[0, duration]` を範囲とする Seek を並べた
/// Panel の `ViewSpec` を返す。
///
/// # Errors
/// 範囲外の channel、source の読み出しエラー、または frame を持たない
/// source の場合。
pub fn waveform_seek(
    source: &WaveformView,
    channel: usize,
    start_s: f64,
    end_s: f64,
    bins: usize,
) -> Result<ViewSpec, ViewError> {
    if source.frames == 0 {
        return Err(ViewError::Invalid("waveform source has no frames".into()));
    }
    let viewport = source.frame_range(start_s, end_s);
    let duration_s = source.frames as f64 / f64::from(source.sample_rate);
    let channel_bins = source
        .window(channel, start_s, end_s, bins)?
        .into_iter()
        .map(|b| WaveformBin {
            start: b.start,
            end: b.end,
            min: b.bucket.min,
            max: b.bucket.max,
            rms: b.bucket.rms,
            count: b.bucket.count,
        })
        .collect();
    let waveform = ComponentSpec::Waveform(WaveformSpec {
        key: ComponentKey::from(WAVEFORM_KEY),
        sample_rate: source.sample_rate,
        channels: 1,
        frames: source.frames,
        viewport,
        channel_bins: vec![channel_bins],
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
    Ok(ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey::from(PANEL_KEY),
            title: None,
            children: vec![waveform, seek],
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    })
}
