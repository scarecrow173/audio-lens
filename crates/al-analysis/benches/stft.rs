//! Criterion bench for `al_analysis::stft::stft`
//! (TASK-034EJgkPQJmqrMLNdP7lVh, design T-baseline).
//!
//! Group `stft` measures the production STFT configuration
//! (window 4096 / hop 2048, fixed) over deterministic LCG noise at
//! ~0.5M and ~1M samples. This is the A3 (FFT plan hoist) baseline:
//! re-run with `--baseline <name>` after the optimization lands.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const WINDOW: usize = 4096;
const HOP: usize = 2048;
const RATE: u32 = 48_000;
const SIZES: [usize; 2] = [524_288, 1_048_576];

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

fn bench_stft(c: &mut Criterion) {
    let mut group = c.benchmark_group("stft");
    for n in SIZES {
        let samples = noise(n, 0x5eed);
        group.bench_with_input(BenchmarkId::new("4096/2048", n), &samples, |b, s| {
            b.iter(|| al_analysis::stft::stft(black_box(s), WINDOW, HOP, RATE));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_stft);
criterion_main!(benches);
