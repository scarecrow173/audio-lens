//! Re-baseline bench for the `RealFft` kernel after the plan/workspace
//! split (#52). Separates plan construction (`RealFft::new`) from
//! transform execution (`forward`/`inverse` on a reused plan) so the
//! kernel cost can be evaluated independently, and quantifies the
//! pow2-only padding overhead a mixed-radix engine would avoid.
//!
//! Sizes cover the hot callers: NCC lag search (`dsp::conv`,
//! `analyze_pair`) and spectral analysis paths. `pad_*` arms measure
//! the realized FFT length for representative non-pow2 inputs
//! (signal.len() + kernel.len() rounded up to pow2), which is the
//! term a mixed-radix FFT would reduce.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::time::Duration;

/// Pow2 transform sizes spanning small STFT frames to 1M+ conv blocks.
const FFT_SIZES: [usize; 7] = [1_024, 4_096, 16_384, 65_536, 262_144, 1_048_576, 4_194_304];
/// Representative non-pow2 payloads whose realized FFT length is the
/// next power of two of `n + m` (overlap-save / NCC-style sizing).
const PAD_CASES: [(usize, usize); 4] = [
    (65_536, 256),
    (262_144, 4_096),
    (800_000, 4_096),
    (1_500_000, 8_192),
];

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

fn bench_fft_kernel(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_kernel");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(2));
    for &n in &FFT_SIZES {
        let input = noise(n, 0xf11);
        let bins = n / 2 + 1;
        let mut out_re = vec![0.0; bins];
        let mut out_im = vec![0.0; bins];
        let mut back = vec![0.0; n];

        group.bench_with_input(BenchmarkId::new("plan", n), &n, |b, &n| {
            b.iter(|| al_math::fft::RealFft::new(black_box(n)));
        });
        let mut fft = al_math::fft::RealFft::new(n);
        group.bench_with_input(BenchmarkId::new("forward", n), &n, |b, _| {
            b.iter(|| {
                fft.forward(
                    black_box(&input),
                    black_box(&mut out_re),
                    black_box(&mut out_im),
                )
            });
        });
        group.bench_with_input(BenchmarkId::new("inverse", n), &n, |b, _| {
            b.iter(|| fft.inverse(black_box(&out_re), black_box(&out_im), black_box(&mut back)));
        });
    }
    group.finish();
}

/// Padding cost: the realized pow2 FFT length for non-pow2 payloads
/// and the waste fraction vs the ideal mixed-radix length.
fn bench_pad_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_pad");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(2));
    for &(n, m) in &PAD_CASES {
        let realized = (n + m).next_power_of_two();
        let waste = realized as f64 / (n + m) as f64;
        eprintln!("pad {n}+{m}: realized={realized} waste={waste:.3}x");
        let input = noise(realized, 0xf11);
        let bins = realized / 2 + 1;
        let mut out_re = vec![0.0; bins];
        let mut out_im = vec![0.0; bins];
        let mut fft = al_math::fft::RealFft::new(realized);
        group.bench_with_input(
            BenchmarkId::new("forward", format!("{n}+{m}->{realized}")),
            &realized,
            |b, _| {
                b.iter(|| {
                    fft.forward(
                        black_box(&input),
                        black_box(&mut out_re),
                        black_box(&mut out_im),
                    )
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_fft_kernel, bench_pad_overhead);
criterion_main!(benches);
