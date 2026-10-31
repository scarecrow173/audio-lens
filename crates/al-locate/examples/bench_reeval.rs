//! perf-reeval-2026-10(perf Issue #76/#71 再評価)用の計測 example。
//! CorrelationPlan の plan build コストと ncc transform コストを分離し、
//! window_anchors_in / band_gain windows で使われる代表 shape を測る。
//!
//! 使い方: `cargo run --release -p al-locate --example bench_reeval`
//! 結果: `docs/impl/ir/perf-reeval-2026-10/bench-reeval.txt`

use std::time::Instant;

use ruviform_numeric::corr::{CorrelationPlan, CorrelationWorkspace, ncc_direct};

fn signal_vec(n: usize, phase: f64) -> Vec<f64> {
    (0..n)
        .map(|i| ((i as f64) * 0.0017 + phase).sin())
        .collect()
}

fn bench_plan(template: usize, signal: usize, label: &str) {
    let iters = if signal > 1_000_000 { 3 } else { 7 };
    // plan build
    let t0 = Instant::now();
    let mut build_ms = f64::MAX;
    let mut plan_opt = None;
    for _ in 0..iters {
        let tb = Instant::now();
        let p = CorrelationPlan::new_ncc(template, signal).unwrap();
        build_ms = build_ms.min(tb.elapsed().as_secs_f64() * 1e3);
        if plan_opt.is_none() {
            plan_opt = Some(p);
        }
    }
    let plan = plan_opt.unwrap();
    let _ = t0;
    // workspace alloc
    let mut ws_ms = f64::MAX;
    let mut ws = None;
    for _ in 0..iters {
        let tb = Instant::now();
        let w = CorrelationWorkspace::new(&plan);
        ws_ms = ws_ms.min(tb.elapsed().as_secs_f64() * 1e3);
        if ws.is_none() {
            ws = Some(w);
        }
    }
    let mut ws = ws.unwrap();
    let t = signal_vec(template, 0.0);
    let s = signal_vec(signal, 0.4);
    // ncc run (min of iters)
    let mut run_ms = f64::MAX;
    let mut curve_len = 0usize;
    for _ in 0..iters {
        let tr = Instant::now();
        let c = plan.ncc(&mut ws, &t, &s, usize::MAX).unwrap();
        run_ms = run_ms.min(tr.elapsed().as_secs_f64() * 1e3);
        curve_len = c.values.len();
    }
    println!(
        "{label:<14} tpl={template:>6} sig={signal:>8} fft={:>8} lags={:>8} build={:>8.2}ms ws={:>8.2}ms ncc={:>8.2}ms",
        plan.fft_len(),
        curve_len,
        build_ms,
        ws_ms,
        run_ms
    );
}

fn bench_direct(template: usize, signal: usize, label: &str) {
    let t = signal_vec(template, 0.0);
    let s = signal_vec(signal, 0.4);
    let iters = 3;
    let mut run_ms = f64::MAX;
    let mut out_len = 0usize;
    for _ in 0..iters {
        let tr = Instant::now();
        let c = ncc_direct(&t, &s, usize::MAX).unwrap();
        out_len = c.len();
        run_ms = run_ms.min(tr.elapsed().as_secs_f64() * 1e3);
    }
    println!(
        "{label:<14} tpl={template:>6} sig={signal:>8} ncc_direct lags={out_len} run={run_ms:>8.2}ms"
    );
}

fn main() {
    println!("=== window_anchors_in shapes (template=512) ===");
    bench_plan(512, 64_000, "narrow_lo");
    bench_plan(512, 132_000, "narrow_hi");
    bench_plan(512, 580_000, "widen_mid");
    bench_plan(512, 1_050_000, "widen_hi");
    bench_plan(512, 2_400_240, "full");
    println!("=== band_gain windows shape ===");
    bench_plan(4_800, 8_640, "band_win");
    bench_direct(4_800, 8_640, "band_win");
    println!("=== full-rate method shapes ===");
    bench_plan(2_400_000, 2_400_240, "method");
}
