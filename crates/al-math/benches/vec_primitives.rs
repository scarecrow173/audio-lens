//! Criterion bench for the `al_math::vec` primitives (TASK-034EpPfi3haBLwJwM4xl66).
//!
//! Group `vec` measures all 8 primitives at sizes 32 (small vectors such as
//! spectral_envelope bins, exposing per-call dispatch cost), 1_024, 65_536,
//! and 1_048_576 (methods.rs preflight full-length energy scale). Inputs are
//! deterministic (sin/cos pattern, same shape as the vec.rs tests) and
//! `black_box` guards against optimization elision. `Throughput::Bytes` is
//! reported so reductions can be read as GB/s.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

const SIZES: [usize; 4] = [32, 1_024, 65_536, 1_048_576];

fn inputs(n: usize) -> (Vec<f64>, Vec<f64>) {
    let x: Vec<f64> = (0..n).map(|i| (i as f64 * 0.71).sin() - 0.25).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 1.37).cos() + 0.5).collect();
    (x, y)
}

fn bench_vec_primitives(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec");
    for n in SIZES {
        let (x, y) = inputs(n);
        let a = -2.5f64;

        // Reductions (scalar output).
        group.throughput(Throughput::Bytes((n * 16) as u64));
        group.bench_with_input(BenchmarkId::new("dot", n), &n, |b, _| {
            b.iter(|| al_math::vec::dot(black_box(&x), black_box(&y)));
        });
        group.throughput(Throughput::Bytes((n * 8) as u64));
        group.bench_with_input(BenchmarkId::new("sum_sq", n), &n, |b, _| {
            b.iter(|| al_math::vec::sum_sq(black_box(&x)));
        });
        group.throughput(Throughput::Bytes((n * 16) as u64));
        group.bench_with_input(BenchmarkId::new("corr0", n), &n, |b, _| {
            b.iter(|| al_math::vec::corr0(black_box(&x), black_box(&y)));
        });
        group.throughput(Throughput::Bytes((n * 8) as u64));
        group.bench_with_input(BenchmarkId::new("rms", n), &n, |b, _| {
            b.iter(|| al_math::vec::rms(black_box(&x)));
        });

        // Elementwise (vector output). Bytes = read + written traffic.
        group.throughput(Throughput::Bytes((n * 16) as u64));
        let mut out = vec![0.0; n];
        group.bench_with_input(BenchmarkId::new("scale", n), &n, |b, _| {
            b.iter(|| al_math::vec::scale(black_box(&x), black_box(a), black_box(&mut out)));
        });
        group.throughput(Throughput::Bytes((n * 24) as u64));
        let mut acc = y.clone();
        group.bench_with_input(BenchmarkId::new("axpy", n), &n, |b, _| {
            b.iter(|| al_math::vec::axpy(black_box(&mut acc), black_box(a), black_box(&x)));
        });
        group.throughput(Throughput::Bytes((n * 24) as u64));
        group.bench_with_input(BenchmarkId::new("sub_scaled", n), &n, |b, _| {
            b.iter(|| al_math::vec::sub_scaled(black_box(&mut acc), black_box(a), black_box(&x)));
        });
        group.throughput(Throughput::Bytes((n * 24) as u64));
        group.bench_with_input(BenchmarkId::new("mag_sq", n), &n, |b, _| {
            b.iter(|| al_math::vec::mag_sq(black_box(&x), black_box(&y), black_box(&mut out)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_vec_primitives);
criterion_main!(benches);
