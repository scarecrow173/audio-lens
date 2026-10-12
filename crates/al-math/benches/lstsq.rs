//! Criterion bench for `al_math::linalg::LeastSquares`
//! (TASK-034EJgkPQJmqrMLNdP7lVh, design T-baseline).
//!
//! Baseline is measured on the **rows API** (`LeastSquares::new` +
//! `solve`) — the A4 column-oriented signature lands in a later
//! task and this file follows it then. Factor construction and
//! `solve` are measured separately at column counts {1, 2, 4} and
//! observation counts {65_536, 262_144, 1_048_576}. Inputs are
//! generated once outside `b.iter` so generation is not part of the
//! measurement. Separate target from `vec_primitives.rs` (different
//! concern).

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

/// `obs` observation rows of `cols` deterministic source values.
fn rows(obs: usize, cols: usize) -> Vec<Vec<f64>> {
    let columns: Vec<Vec<f64>> = (0..cols).map(|j| noise(obs, 0x5eed + j as u64)).collect();
    (0..obs)
        .map(|t| (0..cols).map(|j| columns[j][t]).collect())
        .collect()
}

fn bench_lstsq(c: &mut Criterion) {
    let mut group = c.benchmark_group("lstsq");
    for cols in COLUMN_COUNTS {
        for obs in OBSERVATION_COUNTS {
            let rows = rows(obs, cols);
            let rhs = noise(obs, 0xca9);
            let id = format!("{cols}x{obs}");
            group.bench_with_input(BenchmarkId::new("new", &id), &rows, |b, r| {
                b.iter(|| al_math::linalg::LeastSquares::new(black_box(r), black_box(LAMBDA)));
            });
            let factor = al_math::linalg::LeastSquares::new(&rows, LAMBDA).unwrap();
            group.bench_with_input(BenchmarkId::new("solve", &id), &rhs, |b, rhs| {
                b.iter(|| factor.solve(black_box(rhs)));
            });
        }
    }
    group.finish();
}

criterion_group!(benches, bench_lstsq);
criterion_main!(benches);
