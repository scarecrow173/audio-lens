//! Alloc bench for Issue #98 pre-migration baseline.
//!
//! bench_reference.rs と同じ代表経路を、counting allocator 付きで
//! 計測する。alloc 回数・bytes は「参考値の記録」(mean)。
//! 出力は JSON 1 行で、tools/baseline/capture が bench/results.json
//! へ転記する。

use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use al_analysis::reference::{Aligned, ReferenceConfig, analyze_pair};
use al_core::{Confidence, Validity};

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(l.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

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

fn bench(name: &str, reps: usize, f: &mut dyn FnMut()) {
    f();
    let mut allocs = 0u64;
    let mut bytes = 0u64;
    let mut ms = 0.0;
    for _ in 0..reps {
        ALLOCS.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        let t0 = Instant::now();
        f();
        ms += t0.elapsed().as_secs_f64() * 1000.0;
        allocs += ALLOCS.load(Ordering::Relaxed);
        bytes += BYTES.load(Ordering::Relaxed);
    }
    println!(
        "{{\"name\":{:?},\"reps\":{},\"mean_ms\":{:.3},\"mean_allocs\":{},\"mean_bytes\":{}}}",
        name,
        reps,
        ms / reps as f64,
        allocs / reps as u64,
        bytes / reps as u64,
    );
}

fn main() {
    let rate = 48_000u32;

    let n = 96_000usize;
    let src = noise(n, 0xbeef);
    let cap: Vec<f64> = (0..n)
        .map(|i| if i >= 5 { 0.7 * src[i - 5] } else { 0.0 })
        .collect();
    let aligned = Aligned {
        src: vec![src.as_slice()],
        cap: vec![cap.as_slice()],
        rate,
        upstream_confidence: Confidence::High,
    };
    let cfg = ReferenceConfig::default();
    let alignment_id = al_core::AlignmentId::new();
    bench("analyze_pair 2s mono (R0-R3, fir_model)", 3, &mut || {
        black_box(analyze_pair(
            "src__cap",
            &alignment_id,
            &aligned,
            Validity::Valid,
            &cfg,
        ));
    });

    let src2: Vec<Vec<f64>> = vec![noise(n, 0x1), noise(n, 0x2)];
    let cap2: Vec<Vec<f64>> = (0..2usize)
        .map(|c| {
            let w = [0.6 + 0.1 * c as f64, -0.3 + 0.2 * c as f64];
            let nz = noise(n, 0x100 + c as u64);
            (0..n)
                .map(|i| w[0] * src2[0][i] + w[1] * src2[1][i] + 0.01 * nz[i])
                .collect()
        })
        .collect();
    bench("prepare_residual 2s 2src x 2cap", 3, &mut || {
        black_box(al_locate::prepare_residual(
            black_box(&src2),
            black_box(&cap2),
            1e-6,
        ));
    });
}
