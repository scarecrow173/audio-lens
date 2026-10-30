//! Issue #98 baseline: NaN/Inf・無音・短入力に対する現行 API の
//! 観測挙動を pin する。正しさの主張ではなく「現行の意味」を固定する
//! (移行後に観測が変われば検出できるようにするため)。

use al_math::correlation::{gcc_phat, ncc};
use al_math::fft::{Fft, RealFftPlan, RealFftWorkspace};
use al_math::filter::bandpass;
use al_math::resample::{ResamplePlan, resample};

fn sig(n: usize) -> Vec<f64> {
    (0..n).map(|i| (i as f64 * 0.13).sin()).collect()
}

#[test]
fn fft_nan_propagates_except_structural_zero_im_bins() {
    // 現行観測: NaN 入力は re 全 bin を NaN にし、im は DC/Nyquist
    // (構造的に実数の bin)のみ 0.0 を保持する。
    let fft = Fft::new(8);
    let mut re = sig(8);
    re[3] = f64::NAN;
    let mut im = vec![0.0; 8];
    fft.transform(&mut re, &mut im, false);
    assert!(re.iter().all(|v| v.is_nan()));
    assert_eq!(im[0], 0.0);
    assert_eq!(im[4], 0.0);
    assert!(im[1..4].iter().all(|v| v.is_nan()));
    assert!(im[5..8].iter().all(|v| v.is_nan()));

    // ±Inf も同じ伝搬形
    for bad in [f64::INFINITY, f64::NEG_INFINITY] {
        let mut re = sig(8);
        re[3] = bad;
        let mut im = vec![0.0; 8];
        fft.transform(&mut re, &mut im, false);
        assert!(re.iter().all(|v| !v.is_finite()));
    }
}

#[test]
fn real_fft_nan_keeps_dc_nyquist_im_zero() {
    let plan = RealFftPlan::new(8);
    let mut ws = RealFftWorkspace::new(&plan);
    let mut input = sig(8);
    input[1] = f64::NAN;
    let mut re = vec![0.0; 5];
    let mut im = vec![0.0; 5];
    plan.forward(&mut ws, &input, &mut re, &mut im);
    assert!(re.iter().all(|v| v.is_nan()));
    assert_eq!(im[0], 0.0);
    assert_eq!(im[4], 0.0);
    assert!(im[1..4].iter().all(|v| v.is_nan()));
}

#[test]
fn ncc_and_gcc_phat_observed_edge_contracts() {
    // 現行観測: 無音同士の NCC/GCC-PHAT は [0.0] 長 1 の curve
    let silent = vec![0.0; 64];
    assert_eq!(ncc(&silent, &silent, 8), vec![0.0]);
    assert_eq!(gcc_phat(&silent, &silent, 4), vec![0.0]);

    // template > signal は空 curve
    assert!(ncc(&sig(32), &sig(8), 8).is_empty());

    // 現行観測: NaN を含む template への GCC-PHAT は全 0 curve
    // (energy gate で潰れる)
    let mut t = sig(16);
    t[2] = f64::NAN;
    assert_eq!(gcc_phat(&t, &sig(64), 4), vec![0.0; 5]);
}

#[test]
fn bandpass_and_resample_edge_inputs() {
    // 空入力は空を返す
    assert!(bandpass(&[], 48_000, 100.0, 1_000.0).is_empty());

    // 無音は有限値のまま
    let out = bandpass(&vec![0.0; 256], 48_000, 100.0, 1_000.0);
    assert!(out.iter().all(|v| v.is_finite()));

    // 非有限値を含む入力でも panic しない(長は維持)
    let mut x = sig(256);
    x[10] = f64::INFINITY;
    let out = bandpass(&x, 48_000, 100.0, 1_000.0);
    assert_eq!(out.len(), x.len());

    // resample: ゼロ長は Ok(空) か Err のいずれかを pin
    let plan = ResamplePlan::new(48_000, 96_000).unwrap();
    let empty = resample(&[], &plan);
    assert!(empty.is_err() || empty.unwrap().is_empty());
}
