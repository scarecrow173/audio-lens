//! Criterion bench for the A7/A8 `csd` caller patterns
//! (TASK-034EO484s25JM8bsb6zUCh, design T-baseline).
//!
//! A7: per-frame `csd` returns a `bins`-length `Vec<(f64, f64)>` per
//! frame which callers immediately accumulate bin-by-bin — the bench
//! compares the allocating call against the same `conj(x)·y` element
//! expression inlined into the accumulation loop.
//!
//! A8 (hft): the current code calls `csd` on length-1 slices per
//! `(frame, bin)` — a tiny heap allocation per element — vs the same
//! scalar expression inline. Both arms are self-contained on the
//! existing public API (`csd` stays public).

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

use al_analysis::stft::{csd, stft};

const RATE: u32 = 48_000;
const WINDOW: usize = 4096;
const HOP: usize = 2048;
/// ~2 s @48 kHz → 45 frames at 4096/2048.
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

fn bench_csd_accum(c: &mut Criterion) {
    let mut group = c.benchmark_group("csd_accum");
    let ms = stft(&noise(N, 0x5eed), WINDOW, HOP, RATE);
    let mc = stft(&noise(N, 0xc0ffee), WINDOW, HOP, RATE);
    let frames = ms.frames().min(mc.frames());
    let bins = ms.fft_size;
    let half = bins / 2;

    // A7 — Welch accumulation shape used at reference.rs (pxy/pab),
    // band_gain.rs (xy), cheap.rs stereo (csd_re/csd_im) and
    // expensive/channel.rs (sxy).
    group.bench_function("per_frame_csd_call", |b| {
        b.iter(|| {
            let mut pxy = vec![(0.0f64, 0.0f64); bins];
            for f in 0..frames {
                let c = csd(&ms.re[f], &ms.im[f], &mc.re[f], &mc.im[f]);
                for (p, &(cr, ci)) in pxy.iter_mut().zip(&c) {
                    p.0 += cr;
                    p.1 += ci;
                }
            }
            black_box(pxy)
        })
    });
    group.bench_function("per_frame_inline", |b| {
        b.iter(|| {
            let mut pxy = vec![(0.0f64, 0.0f64); bins];
            for f in 0..frames {
                for (p, ((&xr, &xi), (&yr, &yi))) in pxy.iter_mut().zip(
                    ms.re[f]
                        .iter()
                        .zip(&ms.im[f])
                        .zip(mc.re[f].iter().zip(&mc.im[f])),
                ) {
                    p.0 += xr * yr + xi * yi;
                    p.1 += xr * yi - xi * yr;
                }
            }
            black_box(pxy)
        })
    });

    // A8 — hft construction: length-1 `csd` per (frame, bin) vs the
    // same scalar product inline.
    group.bench_function("hft_len1_csd", |b| {
        b.iter(|| {
            let mut hft = Vec::with_capacity(frames);
            for f in 0..frames {
                let mut row = Vec::with_capacity(half);
                for k in 0..half {
                    let xs = ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
                    row.push(if xs > 0.0 {
                        let c = csd(
                            &ms.re[f][k..k + 1],
                            &ms.im[f][k..k + 1],
                            &mc.re[f][k..k + 1],
                            &mc.im[f][k..k + 1],
                        );
                        ((c[0].0 * c[0].0 + c[0].1 * c[0].1).sqrt()) / xs
                    } else {
                        0.0
                    });
                }
                hft.push(row);
            }
            black_box(hft)
        })
    });
    group.bench_function("hft_scalar_inline", |b| {
        b.iter(|| {
            let mut hft = Vec::with_capacity(frames);
            for f in 0..frames {
                let mut row = Vec::with_capacity(half);
                for k in 0..half {
                    let xs = ms.re[f][k] * ms.re[f][k] + ms.im[f][k] * ms.im[f][k];
                    row.push(if xs > 0.0 {
                        let (xr, xi) = (ms.re[f][k], ms.im[f][k]);
                        let (yr, yi) = (mc.re[f][k], mc.im[f][k]);
                        let cr = xr * yr + xi * yi;
                        let ci = xr * yi - xi * yr;
                        ((cr * cr + ci * ci).sqrt()) / xs
                    } else {
                        0.0
                    });
                }
                hft.push(row);
            }
            black_box(hft)
        })
    });
    group.finish();
}

criterion_group!(benches, bench_csd_accum);
criterion_main!(benches);
