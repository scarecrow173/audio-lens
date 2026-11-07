//! Fake 層・GPUI 層で共有する Source fixture(AL-G3)。

use std::sync::Arc;

use al_gui::view::source::WaveformView;
use ruviform_viz::{SliceWaveform, WaveformSource};

pub const SAMPLE_RATE: u32 = 48_000;
/// 0.1 秒。`[1200, 1440)` frame を NaN gap にする。
pub const FRAMES: usize = 4_800;
pub const GAP: std::ops::Range<usize> = 1_200..1_440;

pub fn fixture_view() -> WaveformView {
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
