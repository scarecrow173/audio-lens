//! Criterion bench for the `fir_model` convolution path
//! (TASK-034EJgkPQJmqrMLNdP7lVh, design T-baseline;
//! TASK-034EJgOK1cNKnai60q1zOd added the `dsp::conv` arm).
//!
//! `conv_direct` is the verbatim reference implementation kept in
//! `reference.rs` tests (no shared helper, per ADR). `conv`
//! (overlap-save via `RealFft`) runs on the same fixtures for a
//! direct A/B against the `pre` baseline.
//!
//! Direct convolution is O(n·m), so `conv_direct` sizes are bounded
//! to n <= 262_144 with `sample_size(10)` and a short
//! `measurement_time` (n = 262_144, m = 4_096 is ~1G MACs/iter).
//! The `conv` arm additionally covers n = 1_048_576 to observe
//! FFT-path scaling where the direct path is too slow to bench.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;

const SIGNAL_SIZES: [usize; 2] = [65_536, 262_144];
/// `conv` adds the 1M scaling point the direct path cannot
/// afford.
const FFT_SIGNAL_SIZES: [usize; 3] = [65_536, 262_144, 1_048_576];
const KERNEL_SIZES: [usize; 2] = [256, 4_096];

/// Direct linear convolution returning the first `x.len()` outputs.
/// Verbatim copy of the reference implementation in
/// `reference.rs` tests — keep in sync by hand.
fn conv_direct(x: &[f64], h: &[f64]) -> Vec<f64> {
    let mut y = vec![0.0; x.len()];
    for (i, &xv) in x.iter().enumerate() {
        let maxk = h.len().min(x.len() - i);
        for (k, &hv) in h.iter().take(maxk).enumerate() {
            y[i + k] += xv * hv;
        }
    }
    y
}

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

fn bench_conv(c: &mut Criterion) {
    let mut group = c.benchmark_group("conv");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(2));
    for m in KERNEL_SIZES {
        let kernel = noise(m, 0xa11ce);
        for n in SIGNAL_SIZES {
            let signal = noise(n, 0x5eed);
            group.bench_with_input(
                BenchmarkId::new("conv_direct", format!("n{n}_m{m}")),
                &(n, m),
                |b, _| {
                    b.iter(|| conv_direct(black_box(&signal), black_box(&kernel)));
                },
            );
        }
        for n in FFT_SIGNAL_SIZES {
            let signal = noise(n, 0x5eed);
            group.bench_with_input(
                BenchmarkId::new("dsp_conv", format!("n{n}_m{m}")),
                &(n, m),
                |b, _| {
                    b.iter(|| ruviform_numeric::conv::conv(black_box(&signal), black_box(&kernel)));
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_conv);
criterion_main!(benches);
