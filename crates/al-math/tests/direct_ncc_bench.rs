//! P3 形状ベンチ: Full FFT plan vs 適応 `new_ncc` plan vs 素の直接
//! カーネルの実測(計測専用、`cargo test --release -- --ignored`)。
//! `new_ncc` は形状で NccDirect/NccValid を選ぶため、ここでの
//! "adaptive" 列は実運用経路そのものの時間を示す。
use al_math::{
    correlation::{CorrelationPlan, CorrelationWorkspace},
    vec::{axpy, dot},
};
use std::time::Instant;

fn noise(len: usize, mut seed: u64) -> Vec<f64> {
    (0..len)
        .map(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            (seed >> 33) as f64 / (1u64 << 30) as f64 - 1.0
        })
        .collect()
}

/// 生の分子のみ: num[k] = Σ_i a[i]·b[k+i]、normalize は別途共通。
fn direct_outer_lag(a: &[f64], b: &[f64], max_lag: usize) -> Vec<f64> {
    (0..=max_lag).map(|k| dot(a, &b[k..k + a.len()])).collect()
}

fn direct_outer_i(a: &[f64], b: &[f64], max_lag: usize) -> Vec<f64> {
    let mut acc = vec![0.0; max_lag + 1];
    for (i, &ai) in a.iter().enumerate() {
        axpy(&mut acc, ai, &b[i..i + max_lag + 1]);
    }
    acc
}

/// ncc と同じ正規化を直接法の分子へ適用(契約確認用)。
fn normalize_like_ncc(num: &mut [f64], b: &[f64], len: usize, template_energy: f64) {
    let mut cum = vec![0.0; b.len() + 1];
    for (i, &v) in b.iter().enumerate() {
        cum[i + 1] = cum[i] + v * v;
    }
    let tn = template_energy.sqrt();
    for (lag, v) in num.iter_mut().enumerate() {
        let e = (cum[lag + len] - cum[lag]).max(0.0);
        *v = if e > 0.0 { *v / (tn * e.sqrt()) } else { 0.0 };
    }
}

fn max_err(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f64::max)
}

#[test]
#[ignore]
fn direct_vs_fft_shapes() {
    // 実形状: (名前, L, S)  — 全呼出で K = S−L。
    let shapes: &[(&str, usize, usize)] = &[
        ("band_ncc/full_ncc 44.1k", 2_205_000, 2_205_236),
        ("coarse_ncc 1/4rate", 551_250, 551_309),
        ("band_gain window", 4_410, 7_938),
        ("narrow anchor 小", 4_410, 13_230),
        ("narrow anchor 中", 4_410, 66_150),
        ("tiny 8kHz", 800, 1_438),
    ];
    for &(name, l, s) in shapes {
        let max_lag = s - l;
        let a = noise(l, 7);
        let b = noise(s, 11);

        // 実運用経路: new_ncc は形状で direct/valid を選ぶ。
        let plan = CorrelationPlan::new_ncc(l, s);
        let mut ws = CorrelationWorkspace::new(&plan);
        let t = Instant::now();
        let adaptive = plan.ncc(&mut ws, &a, &b, max_lag).to_vec();
        let adaptive_t = t.elapsed();

        // Full FFT plan(変更前の一律経路)との対比。
        let full = CorrelationPlan::new(l, s);
        let mut full_ws = CorrelationWorkspace::new(&full);
        let t = Instant::now();
        let fft_curve = full.ncc(&mut full_ws, &a, &b, max_lag).to_vec();
        let full_t = t.elapsed();

        let t = Instant::now();
        let mut num_a = direct_outer_lag(&a, &b, max_lag);
        let ta = t.elapsed();
        normalize_like_ncc(&mut num_a, &b, l, al_math::vec::sum_sq(&a));

        let t = Instant::now();
        let mut num_b = direct_outer_i(&a, &b, max_lag);
        let tb = t.elapsed();
        normalize_like_ncc(&mut num_b, &b, l, al_math::vec::sum_sq(&a));

        eprintln!(
            "{name:>22}: L={l} S={s} K={max_lag} | adaptive={adaptive_t:?} | full-fft={full_t:?} \
             | lag-outer={ta:?} | i-outer={tb:?} | err: adaptive={:.2e} lag-outer={:.2e} \
             i-outer={:.2e}",
            max_err(&adaptive, &fft_curve),
            max_err(&fft_curve, &num_a),
            max_err(&fft_curve, &num_b)
        );
    }
}
