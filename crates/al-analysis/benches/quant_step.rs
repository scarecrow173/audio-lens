//! Criterion bench for the A12 `dig.quant_step_est` distinct-count
//! path (TASK-034EO484s25JM8bsb6zUCh, design T-baseline).
//!
//! Current shape: `uniq: Vec<u64>` sort + dedup is always built, then
//! the `2 <= uniq.len() <= 1_000_000` gate decides whether the f64
//! re-materialization + `total_cmp` sort runs. Candidate shape:
//! `HashSet<u64>` pre-count so the gate skips the u64 sort entirely.
//! Both shapes are written out inside this bench — no production
//! code change is required to measure them.
//!
//! Two fixtures: an in-gate signal (small level set) and a >1M
//! distinct signal where the gate skips everything after counting.

use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::HashSet;
use std::hint::black_box;

const IN_GATE_N: usize = 1_000_000;
const HI_DISTINCT_N: usize = 1_500_000;

/// Deterministic zero-mean pseudo-noise (LCG, same generator shape
/// as the in-tree test fixtures). The `>> 33` mapping keeps ~31 bits
/// of distinctness, so `HI_DISTINCT_N` samples carry >1M distinct
/// bit patterns.
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

/// Current double-sort shape (`cheap.rs` dig.quant_step_est): u64
/// sort + dedup always, f64 materialize + `total_cmp` sort inside
/// the in-gate branch. Returns `(quant_step, support)`.
fn quant_double_sort(samples: &[f64]) -> (f64, u64) {
    let mut uniq: Vec<u64> = samples.iter().map(|v| v.to_bits()).collect();
    uniq.sort_unstable();
    uniq.dedup();
    let quant = if uniq.len() >= 2 && uniq.len() <= 1_000_000 {
        let mut sorted: Vec<f64> = uniq.iter().map(|b| f64::from_bits(*b)).collect();
        sorted.sort_by(|a, b| a.total_cmp(b));
        sorted
            .windows(2)
            .map(|w| w[1] - w[0])
            .filter(|d| *d > 0.0)
            .fold(f64::INFINITY, f64::min)
    } else {
        f64::NAN
    };
    (quant, uniq.len() as u64)
}

/// Candidate HashSet pre-count shape: count first, run the two
/// sorts only inside the gate. Same value semantics — the sorted
/// set determines `quant`, and `support` is the distinct count.
fn quant_hashset_gate(samples: &[f64]) -> (f64, u64) {
    let distinct: HashSet<u64> = samples.iter().map(|v| v.to_bits()).collect();
    let quant = if distinct.len() >= 2 && distinct.len() <= 1_000_000 {
        let mut sorted: Vec<f64> = distinct.iter().map(|b| f64::from_bits(*b)).collect();
        sorted.sort_by(|a, b| a.total_cmp(b));
        sorted
            .windows(2)
            .map(|w| w[1] - w[0])
            .filter(|d| *d > 0.0)
            .fold(f64::INFINITY, f64::min)
    } else {
        f64::NAN
    };
    (quant, distinct.len() as u64)
}

fn bench_quant_step(c: &mut Criterion) {
    let mut group = c.benchmark_group("quant_step");

    // In-gate fixture: quantized to a 1/16 grid → ~33 distinct levels.
    let in_gate: Vec<f64> = noise(IN_GATE_N, 0x9e37)
        .iter()
        .map(|&v| (v * 16.0).round() / 16.0)
        .collect();
    group.bench_function("in_gate_double_sort", |b| {
        b.iter(|| black_box(quant_double_sort(black_box(&in_gate))))
    });
    group.bench_function("in_gate_hashset_gate", |b| {
        b.iter(|| black_box(quant_hashset_gate(black_box(&in_gate))))
    });

    // >1M distinct fixture: the gate skips the f64 materialization
    // in both shapes; the HashSet shape also skips the u64 sort.
    let hi = noise(HI_DISTINCT_N, 0x51ed);
    let (_, support) = quant_double_sort(&hi);
    assert!(
        support > 1_000_000,
        "fixture must exceed the gate: {support}"
    );
    group.bench_function("over_1m_double_sort", |b| {
        b.iter(|| black_box(quant_double_sort(black_box(&hi))))
    });
    group.bench_function("over_1m_hashset_gate", |b| {
        b.iter(|| black_box(quant_hashset_gate(black_box(&hi))))
    });
    group.finish();
}

criterion_group!(benches, bench_quant_step);
criterion_main!(benches);
