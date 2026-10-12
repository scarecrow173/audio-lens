//! Criterion bench for `al_math::linalg::LeastSquares`
//! (TASK-034EJgkPQJmqrMLNdP7lVh, design T-baseline).
//!
//! Baseline `lstsq-rows` was measured on the former rows API
//! (`LeastSquares::new`); the A4 column-oriented signature landed in
//! TASK-034EJgyjQ0EutoP1M6RqoQ and this file follows it. The
//! benchmark IDs (`new` = factor construction, `solve`) are kept so
//! `--baseline lstsq-rows` reports a direct delta against the rows
//! API. Factor construction and `solve` are measured separately at
//! column counts {1, 2, 4} and observation counts {65_536, 262_144,
//! 1_048_576}. Inputs are generated once outside `b.iter` so
//! generation is not part of the measurement. Separate target from
//! `vec_primitives.rs` (different concern).

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;

const COLUMN_COUNTS: [usize; 3] = [1, 2, 4];
const OBSERVATION_COUNTS: [usize; 3] = [65_536, 262_144, 1_048_576];
const LAMBDA: f64 = 1e-6;

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

/// `cols` deterministic source columns of `obs` observations each
/// (same values as the rows-API baseline, transposed).
fn columns(obs: usize, cols: usize) -> Vec<Vec<f64>> {
    (0..cols).map(|j| noise(obs, 0x5eed + j as u64)).collect()
}

fn bench_lstsq(c: &mut Criterion) {
    let mut group = c.benchmark_group("lstsq");
    for cols in COLUMN_COUNTS {
        for obs in OBSERVATION_COUNTS {
            let columns = columns(obs, cols);
            let refs: Vec<&[f64]> = columns.iter().map(Vec::as_slice).collect();
            let rhs = noise(obs, 0xca9);
            let id = format!("{cols}x{obs}");
            group.bench_with_input(BenchmarkId::new("new", &id), &refs, |b, r| {
                b.iter(|| {
                    al_math::linalg::LeastSquares::from_columns(black_box(r), black_box(LAMBDA))
                });
            });
            let factor = al_math::linalg::LeastSquares::from_columns(&refs, LAMBDA).unwrap();
            group.bench_with_input(BenchmarkId::new("solve", &id), &rhs, |b, rhs| {
                b.iter(|| factor.solve(black_box(rhs)));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_lstsq);
criterion_main!(benches);
