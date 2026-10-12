//! Criterion bench for the `fir_model` convolution path
//! (TASK-034EJgkPQJmqrMLNdP7lVh, design T-baseline).
//!
//! T0 contains only the `conv_direct` baseline arm — `dsp::conv`
//! does not exist until the A1 optimization commit, which adds its
//! arm on the same fixtures for a direct A/B. `conv_direct` is the
//! verbatim reference implementation kept in `reference.rs` tests
//! and `dsp.rs` tests (no shared helper, per ADR).
//!
//! Direct convolution is O(n·m), so sizes are bounded to
//! n <= 262_144 with `sample_size(10)` and a short
//! `measurement_time` (n = 262_144, m = 4_096 is ~1G MACs/iter).
//! The FFT-path-only 1M scaling point arrives with the `dsp::conv`
//! arm, not here.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;

const SIGNAL_SIZES: [usize; 2] = [65_536, 262_144];
const KERNEL_SIZES: [usize; 2] = [256, 4_096];

/// Direct linear convolution returning the first `x.len()` outputs.
/// Verbatim copy of the reference implementation in
/// `reference.rs`/`dsp.rs` tests — keep in sync by hand.
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
    }
    group.finish();
}

criterion_group!(benches, bench_conv);
criterion_main!(benches);
