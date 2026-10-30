//! Ruviform Git 依存(AL#101 で消費する解析/STFT・帯域カーネル)の
//! import smoke-test。基礎型の構築とリンクのみを検証する
//! (docs/designs/ruviform-dependency-ci.md §1)。
//! カーネル呼出の実切替は AL#101 のスコープ。
use ruviform_audio_analysis::{ChannelLevel, LevelMeter};
use ruviform_dsp::Window;

#[test]
fn audio_analysis_level_meter() {
    let meter = LevelMeter::new(2).unwrap();
    assert_eq!(meter.channels(), 2);
    assert_eq!(meter.levels(), &[ChannelLevel::default(); 2]);
}

#[test]
fn dsp_window() {
    let rect = Window::Rectangular.coeffs(8);
    assert!(rect.iter().all(|&v| (v - 1.0).abs() < 1e-7));
}
