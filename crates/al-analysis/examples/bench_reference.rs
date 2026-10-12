//! Instant bench for the reference-stage hot paths
//! (TASK-034EJgkPQJmqrMLNdP7lVh, design T-baseline).
//!
//! Same conventions as the al-locate `bench_*` examples: warmup +
//! `reps` iterations, mean ms per line, output redirected to
//! `target/bench-baseline/` for before/after comparison.
//!
//! Measured paths:
//! - `stft` at the production setting (4096/2048) — A3 baseline.
//! - `analyze_pair` over a bounded synthetic pair (~2 s @48k,
//!   LCG noise + a composed delay/gain mix) covering the R0–R3
//!   staged-residual path including `fir_model` — A1 baseline.
//! - `prepare_residual` (al-locate pub, `&[Vec<f64>]` signature is
//!   stable across A4) which exercises `fit_mix` internally —
//!   A4 baseline. `fit_mix` itself is not measured directly because
//!   its signature changes under A4.

use std::hint::black_box;
use std::time::Instant;

use al_analysis::reference::{Aligned, ReferenceConfig, analyze_pair};
use al_analysis::stft::stft;
use al_core::{Confidence, Validity};

/// Deterministic zero-mean pseudo-noise (LCG, same generator shape
/// as the in-tree test fixtures).
fn noise(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed;
    (0..n)
        .map(|_| {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((s >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        })
        .collect()
}

fn bench(name: &str, reps: usize, f: &mut dyn FnMut()) {
    f();
    let t0 = Instant::now();
    for _ in 0..reps {
        f();
    }
    println!(
        "{:48} {:9.1} ms",
        name,
        t0.elapsed().as_secs_f64() * 1000.0 / reps as f64
    );
}

fn main() {
    let rate = 48_000u32;

    // A3: production stft over the ~1M-sample criterion point.
    let long = noise(1_048_576, 0x5eed);
    bench("stft 1M samples (4096/2048)", 3, &mut || {
        black_box(stft(black_box(&long), 4096, 2048, rate));
    });

    // A1: bounded synthetic pair — mono source, capture = 0.7x
    // delayed by 5 samples (FIR-recoverable mix), ~2 s @48k.
    let n = 96_000usize;
    let src = noise(n, 0xbeef);
    let cap: Vec<f64> = (0..n)
        .map(|i| if i >= 5 { 0.7 * src[i - 5] } else { 0.0 })
        .collect();
    let aligned = Aligned {
        src: vec![src.as_slice()],
        cap: vec![cap.as_slice()],
        rate,
        upstream_confidence: Confidence::High,
    };
    let cfg = ReferenceConfig::default();
    let alignment_id = al_core::AlignmentId::new();
    bench("analyze_pair 2s mono (R0-R3, fir_model)", 3, &mut || {
        black_box(analyze_pair(
            "src__cap",
            &alignment_id,
            &aligned,
            Validity::Valid,
            &cfg,
        ));
    });

    // A4: prepare_residual = caller-level fit_mix measure. Public
    // signature `&[Vec<f64>]` is stable across the A4 column-API
    // migration, so this harness stays identical before/after.
    let src2: Vec<Vec<f64>> = vec![noise(n, 0x1), noise(n, 0x2)];
    let cap2: Vec<Vec<f64>> = (0..2usize)
        .map(|c| {
            let w = [0.6 + 0.1 * c as f64, -0.3 + 0.2 * c as f64];
            let nz = noise(n, 0x100 + c as u64);
            (0..n)
                .map(|i| w[0] * src2[0][i] + w[1] * src2[1][i] + 0.01 * nz[i])
                .collect()
        })
        .collect();
    bench("prepare_residual 2s 2src x 2cap", 3, &mut || {
        black_box(al_locate::prepare_residual(
            black_box(&src2),
            black_box(&cap2),
            1e-6,
        ));
    });
}
