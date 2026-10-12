//! Targeted Instant measurement for the residual subtraction loop
//! (`residual.rs` `subtract_fit`), per the T4 conditional-reorganization
//! protocol (design docs/designs/al-math-vec-primitives.md).
//!
//! Measures both loop variants on the same representative fixture in one
//! process — the current transposed-access `*v -= Σ_s m·s[t]` gather, and
//! the per-source `dsp::sub_scaled(cap, m, s)` reorganization — then checks
//! the elementwise equivalence predicate |a-b| <= 1e-12*max(1,|a|,|b|).

use std::hint::black_box;
use std::time::Instant;

const REPS: usize = 15;

fn noise(seed: u64, n: usize) -> Vec<f64> {
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

/// Current residual.rs loop: `cap[t] -= Σ_s m_s·src_s[t]` (strided gather).
fn legacy_subtract(src: &[Vec<f64>], capture: &[Vec<f64>], matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut residual = capture.to_vec();
    for (cap, row) in residual.iter_mut().zip(matrix) {
        for (t, v) in cap.iter_mut().enumerate() {
            let pred: f64 = src.iter().zip(row).map(|(s, &m)| s[t] * m).sum();
            *v -= pred;
        }
    }
    residual
}

/// Reorganized form: per-source `sub_scaled` passes (contiguous streams).
fn reorg_subtract(src: &[Vec<f64>], capture: &[Vec<f64>], matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut residual = capture.to_vec();
    for (cap, row) in residual.iter_mut().zip(matrix) {
        for (s, &m) in src.iter().zip(row) {
            al_locate::dsp::sub_scaled(cap, m, s);
        }
    }
    residual
}

/// Interleaved A/B measurement: alternates both variants across reps so
/// slow drift (thermal, allocator, cache state) hits them symmetrically.
fn measure_pair(
    mut legacy: impl FnMut() -> Vec<Vec<f64>>,
    mut reorg: impl FnMut() -> Vec<Vec<f64>>,
) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    black_box(legacy());
    black_box(reorg());
    let (mut legacy_times, mut reorg_times) = (Vec::new(), Vec::new());
    let (mut legacy_out, mut reorg_out) = (Vec::new(), Vec::new());
    for _ in 0..REPS {
        let start = Instant::now();
        legacy_out = legacy();
        legacy_times.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(&legacy_out);
        let start = Instant::now();
        reorg_out = reorg();
        reorg_times.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(&reorg_out);
    }
    for (label, times) in [
        ("legacy-gather-subtract", &mut legacy_times),
        ("per-source-sub_scaled", &mut reorg_times),
    ] {
        times.sort_by(f64::total_cmp);
        println!(
            "{label}: median_ms={:.3} min_ms={:.3} p95_ms={:.3}",
            times[REPS / 2],
            times[0],
            times[REPS - 1]
        );
    }
    (legacy_out, reorg_out)
}

fn run_shape(frames: usize, source_ch: usize) {
    // Representative channel layouts from stage.rs residual search:
    // stereo capture, mono or stereo parent source aligned to the
    // capture window, ~35 s @ 48 kHz frames.
    let src: Vec<Vec<f64>> = (0..source_ch)
        .map(|c| noise(11 + 13 * c as u64, frames))
        .collect();
    let capture: Vec<Vec<f64>> = vec![noise(37, frames), noise(53, frames)];
    let matrix: Vec<Vec<f64>> = (0..capture.len())
        .map(|c| {
            (0..source_ch)
                .map(|s| ((c * 3 + s * 5) as f64 * 0.17).sin() * 0.8)
                .collect()
        })
        .collect();
    println!(
        "frames={frames} capture_ch={} source_ch={source_ch} reps={REPS}",
        capture.len()
    );

    let (legacy, reorg) = measure_pair(
        || legacy_subtract(&src, &capture, &matrix),
        || reorg_subtract(&src, &capture, &matrix),
    );

    // Correctness gate: elementwise predicate on the same fixture.
    let mut max_diff = 0.0f64;
    let mut max_ratio = 0.0f64;
    for (a, b) in legacy.iter().flatten().zip(reorg.iter().flatten()) {
        let diff = (a - b).abs();
        max_diff = max_diff.max(diff);
        max_ratio = max_ratio.max(diff / (1.0f64.max(a.abs()).max(b.abs())));
    }
    println!("elementwise: max_diff={max_diff:.3e} max_ratio={max_ratio:.3e}");
    assert!(
        max_ratio <= 1e-12,
        "reorg violates the equivalence predicate: ratio {max_ratio}"
    );
    println!("predicate |a-b| <= 1e-12*max(1,|a|,|b|): PASS");
}

fn main() {
    let frames = 1_680_000usize;
    run_shape(frames, 1);
    run_shape(frames, 2);
}
