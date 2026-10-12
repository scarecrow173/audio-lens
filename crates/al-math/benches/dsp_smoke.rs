//! Smoke bench: exercises the public dispatch entry points
//! (`dispatch` → `WithSimd`) through two existing kernels so
//! `cargo bench -p al-math` verifies the criterion wiring end to end.
//!
//! Production benches for the vector primitives (measurement targets
//! and metrics) are deferred to the `al-math-simd-reduction` chain;
//! this target intentionally measures only `correlation::ncc` and
//! `filter::spectral_envelope` at one fixed size.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn bench_dsp_dispatch(c: &mut Criterion) {
    let signal: Vec<f64> = (0..8192).map(|i| (i as f64 * 0.01).sin()).collect();
    let template = signal[1024..2048].to_vec();
    let max_lag = signal.len() - template.len();

    c.bench_function("correlation::ncc", |b| {
        b.iter(|| {
            al_math::correlation::ncc(black_box(&template), black_box(&signal), black_box(max_lag))
        });
    });
    c.bench_function("filter::spectral_envelope", |b| {
        b.iter(|| al_math::filter::spectral_envelope(black_box(&signal), 1024, 32));
    });
}

criterion_group!(benches, bench_dsp_dispatch);
criterion_main!(benches);
