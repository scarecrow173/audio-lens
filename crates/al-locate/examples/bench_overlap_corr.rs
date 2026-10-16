//! local_lag の直接 corr0 走査と OverlapCorr(FFT)の窓サイズ別比較。
//! 小窓 dispatch 閾値の要否を実測で判定するための計測。
use std::hint::black_box;
use std::time::Instant;

use al_locate::dsp::{OverlapCorrPlan, OverlapCorrWorkspace, corr0};

fn noise(seed: u64, n: usize) -> Vec<f64> {
    let mut state = seed;
    (0..n)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((state >> 33) as f64 / (1u64 << 31) as f64) * 2.0 - 1.0
        })
        .collect()
}

/// 現行 local_lag と同じ ±max_lag の corr0 全走査。
fn direct_scan(x: &[f64], y: &[f64], max_lag: usize) -> Vec<f64> {
    let l = x.len();
    (-(max_lag as isize)..=max_lag as isize)
        .map(|lag| {
            let a = lag.unsigned_abs();
            if a >= l {
                return 0.0;
            }
            let (xs, ys) = if lag >= 0 {
                (&x[..l - a], &y[a..])
            } else {
                (&x[a..], &y[..l - a])
            };
            corr0(xs, ys)
        })
        .collect()
}

fn main() {
    println!("window,max_lag,direct_us,fft_us,speedup");
    for (window, lag_div) in [
        (128usize, 4usize),
        (256, 4),
        (512, 4),
        (1024, 4),
        (2048, 4),
        (4096, 8),
        (8192, 16), // lag_max = 512 (既定)
        (16384, 32),
    ] {
        let max_lag = (window / lag_div).max(1);
        let x = noise(3, window);
        let y = noise(5, window);
        let reps = (200_000_000usize / (window * max_lag).max(1)).clamp(3, 2000);
        // warm
        black_box(direct_scan(&x, &y, max_lag));
        let t = Instant::now();
        for _ in 0..reps {
            black_box(direct_scan(&x, &y, max_lag));
        }
        let direct = t.elapsed().as_secs_f64() * 1e6 / reps as f64;

        let plan = OverlapCorrPlan::new(window);
        let mut ws = OverlapCorrWorkspace::new(&plan);
        black_box(plan.curve(&mut ws, &x, &y, max_lag));
        let t = Instant::now();
        for _ in 0..reps {
            black_box(plan.curve(&mut ws, &x, &y, max_lag));
        }
        let fft = t.elapsed().as_secs_f64() * 1e6 / reps as f64;
        println!(
            "{window},{max_lag},{direct:.1},{fft:.1},{:.1}x",
            direct / fft
        );
    }
}
