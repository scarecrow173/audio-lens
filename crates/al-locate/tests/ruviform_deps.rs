//! Ruviform Git 依存(AL#101 で消費する DSP/数値カーネル)の import
//! smoke-test。基礎型の構築とリンクのみを検証する
//! (docs/designs/ruviform-dependency-ci.md §1)。
//! カーネル呼出の実切替は AL#101 のスコープ。
use ruviform_dsp::{Fft, Window};
use ruviform_math::dsp::SimdToken;

#[test]
fn dsp_window_and_fft() {
    let hann = Window::Hann.coeffs(5);
    assert!((hann[0] - 0.0).abs() < 1e-6);
    assert!((hann[1] - 0.5).abs() < 1e-6);
    assert!((hann[2] - 1.0).abs() < 1e-6);

    let fft = Fft::new(8).unwrap();
    assert_eq!(fft.len(), 8);
}

#[test]
fn math_simd_detection() {
    assert_ne!(ruviform_math::version(), "");
    let token = SimdToken::detect();
    assert!(!token.level().is_empty());
}
