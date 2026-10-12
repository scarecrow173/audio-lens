//! Criterion bench for the A9/A16a/A16b spectral-pass shapes
//! (TASK-034EO484s25JM8bsb6zUCh, design T-baseline).
//!
//! All arms run over one deterministic `StftMatrix`:
//! - A9: `magnitude()` + `power()` + `psd()` (three `mag_sq` passes)
//!   vs `power()` once + caller-side `sqrt` / `* inv` derivation.
//! - A16a: column-scan mean (`(0..bins).map(|k| rows.map(|r| r[k])
//!   .sum() / frames)`, the current `mean_psd_*`/`mean_power` shape)
//!   vs the equivalent row-major scalar accumulation — same
//!   row-ascending addition order, so bit-identical by construction.
//! - A16b (`dom()`): `magnitude()` full matrix + row-mean argmax vs
//!   a reused `fft_size` scratch fed by `mag_sq` per frame +
//!   `scratch[k].sqrt()` accumulation. `mag_sq` goes through the
//!   `al_locate::dsp` facade (same kernel `magnitude()` calls).

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use al_analysis::stft::{StftMatrix, stft};
use al_locate::dsp::mag_sq;

const RATE: u32 = 48_000;
const WINDOW: usize = 4096;
const HOP: usize = 2048;
/// ~5.5 s @48 kHz → 127 frames × 4096 bins.
const N: usize = 262_144;

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

/// Row-major scalar accumulation — same 0.0-start, row-ascending,
/// k-ascending order and `/ frames` divisor as the column scan.
fn mean_row_major(rows: &[Vec<f64>], bins: usize, frames: usize) -> Vec<f64> {
    let mut mean = vec![0.0; bins];
    for row in rows.iter().take(frames) {
        for (k, &v) in row.iter().enumerate() {
            mean[k] += v;
        }
    }
    for v in &mut mean {
        *v /= frames as f64;
    }
    mean
}

fn bench_spectral_pass(c: &mut Criterion) {
    let mut group = c.benchmark_group("spectral_pass");
    let m: StftMatrix = stft(&noise(N, 0x5eed), WINDOW, HOP, RATE);
    let frames = m.frames();
    let bins = m.fft_size;
    let half = bins / 2;
    let bin_hz = RATE as f64 / bins as f64;

    // A9 — three `mag_sq` passes vs one `power()` + derivations.
    group.bench_function("three_passes_mag_pow_psd", |b| {
        b.iter(|| {
            let mag = m.magnitude();
            let pow = m.power();
            let psd = m.psd();
            black_box((mag, pow, psd))
        })
    });
    group.bench_function("single_power_derived", |b| {
        b.iter(|| {
            let pow = m.power();
            let norm = (0.375 * m.window as f64) * f64::from(m.sample_rate);
            let inv = 1.0 / norm;
            let mag: Vec<Vec<f64>> = pow
                .iter()
                .map(|row| row.iter().map(|&v| v.sqrt()).collect())
                .collect();
            let psd: Vec<Vec<f64>> = pow
                .iter()
                .map(|row| row.iter().map(|&v| v * inv).collect())
                .collect();
            black_box((mag, pow, psd))
        })
    });

    // A16a — column-scan mean (current) vs row-major accumulation.
    let psd = m.psd();
    group.bench_function("mean_column_scan", |b| {
        b.iter(|| {
            black_box(
                (0..bins)
                    .map(|k| psd.iter().take(frames).map(|r| r[k]).sum::<f64>() / frames as f64)
                    .collect::<Vec<f64>>(),
            )
        })
    });
    group.bench_function("mean_row_major", |b| {
        b.iter(|| black_box(mean_row_major(&psd, bins, frames)))
    });

    // A16b — dom() full magnitude matrix vs per-frame mag_sq scratch.
    group.bench_function("dom_magnitude_matrix", |b| {
        b.iter(|| {
            let mag = m.magnitude();
            let mut mean = vec![0.0; half];
            for row in &mag {
                for (k, v) in mean.iter_mut().enumerate() {
                    *v += row[k];
                }
            }
            black_box(
                mean.iter()
                    .enumerate()
                    .skip(1)
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(b, _)| b as f64 * bin_hz)
                    .unwrap_or(0.0),
            )
        })
    });
    group.bench_function("dom_mag_sq_scratch", |b| {
        b.iter(|| {
            let mut mean = vec![0.0; half];
            let mut scratch = vec![0.0; m.fft_size];
            for f in 0..m.frames() {
                mag_sq(&m.re[f], &m.im[f], &mut scratch);
                for (k, v) in mean.iter_mut().enumerate() {
                    *v += scratch[k].sqrt();
                }
            }
            black_box(
                mean.iter()
                    .enumerate()
                    .skip(1)
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(b, _)| b as f64 * bin_hz)
                    .unwrap_or(0.0),
            )
        })
    });
    group.finish();
}

criterion_group!(benches, bench_spectral_pass);
criterion_main!(benches);
