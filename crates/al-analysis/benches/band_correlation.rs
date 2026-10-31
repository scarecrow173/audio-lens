//! Criterion bench for the A6 `band_correlation` path
//! (TASK-034EO484s25JM8bsb6zUCh, design T-baseline).
//!
//! Current shape: `bands.iter().take(8)` × `bandpass` per band per
//! signal (one forward FFT per band). Candidate shape:
//! `bandpass_many` once per signal over `&bands[..bands.len().min(8)]`.
//! Both arms are self-contained on existing public API — `bandpass`
//! is already a `bandpass_many(.., &[(lo, hi)])` `.pop()` wrapper,
//! so the outputs are bit-identical by construction and only the
//! plan/forward count differs (16 → 2 per signal).

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use al_analysis::stft::third_octave_bands;
use ruviform_numeric::bands::{bandpass, bandpass_many};
use ruviform_numeric::vec::corr0;

const RATE: u32 = 48_000;
/// ~2 s @48 kHz — same pair scale as `examples/bench_reference.rs`.
const N: usize = 96_000;

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

fn bench_band_correlation(c: &mut Criterion) {
    let mut group = c.benchmark_group("band_correlation");
    let src = noise(N, 0xbeef);
    let cap = noise(N, 0xcafe);
    // Same band set `analyze_pair` uses for `ref.band_correlation`.
    let bands = third_octave_bands(40.0, (RATE as f64 / 2.0).min(16_000.0));

    // Current: one `bandpass` call per band per signal.
    group.bench_function("sequential_8x_bandpass", |b| {
        b.iter(|| {
            let mut corrs = Vec::with_capacity(8);
            for &(lo, hi) in bands.iter().take(8) {
                let xs = bandpass(black_box(&src), RATE, lo, hi)
                    .expect("band settings and input must be valid");
                let ys = bandpass(black_box(&cap), RATE, lo, hi)
                    .expect("band settings and input must be valid");
                corrs.push(corr0(&xs, &ys).expect("equal slice lengths"));
            }
            black_box(corrs)
        })
    });

    // Candidate: one forward FFT per signal over the capped slice.
    group.bench_function("batched_bandpass_many", |b| {
        b.iter(|| {
            let capped = &bands[..bands.len().min(8)];
            let xs = bandpass_many(black_box(&src), RATE, capped)
                .expect("band settings and input must be valid");
            let ys = bandpass_many(black_box(&cap), RATE, capped)
                .expect("band settings and input must be valid");
            black_box(
                xs.iter()
                    .zip(&ys)
                    .map(|(x, y)| corr0(x, y).expect("equal slice lengths"))
                    .collect::<Vec<f64>>(),
            )
        })
    });
    group.finish();
}

criterion_group!(benches, bench_band_correlation);
criterion_main!(benches);
