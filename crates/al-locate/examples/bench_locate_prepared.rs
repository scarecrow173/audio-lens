//! TASK-035g: 同一 CPU・入力で準備費用と探索費用を分けて測る。
use std::hint::black_box;
use std::time::Instant;

use al_locate::methods::{BandNcc, FullNcc, LocateMethod};
use al_locate::prepared::InputCache;

const RATE: u32 = 48_000;
const REPS: usize = 7;

fn measure(label: &str, mut work: impl FnMut()) {
    for _ in 0..2 {
        work();
    }
    let mut times = Vec::new();
    for _ in 0..REPS {
        let start = Instant::now();
        work();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{label}: median_ms={:.3} p95_ms={:.3}",
        times[REPS / 2],
        times[REPS - 1]
    );
}

fn main() {
    let mut state = 901u64;
    let capture: Vec<_> = (0..RATE as usize * 35)
        .map(|_| {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        })
        .collect();
    let sources = [&capture[48_001..1_008_001], &capture[480_003..1_440_003]];
    println!(
        "rate={RATE} capture_seconds=35 source_seconds=20 pairs=2 seed=901 warmups=2 reps={REPS}"
    );
    measure("prepare-cold-two-sources-and-capture", || {
        let mut cache = InputCache::default();
        black_box(
            cache
                .prepare("capture", 0, RATE, "original", &capture)
                .unwrap(),
        );
        for (i, source) in sources.iter().enumerate() {
            black_box(
                cache
                    .prepare(&format!("source-{i}"), 0, RATE, "original", source)
                    .unwrap(),
            );
        }
    });
    let mut cache = InputCache::default();
    let signal = cache
        .prepare("capture", 0, RATE, "original", &capture)
        .unwrap();
    let templates: Vec<_> = sources
        .iter()
        .enumerate()
        .map(|(i, source)| {
            cache
                .prepare(&format!("source-{i}"), 0, RATE, "original", source)
                .unwrap()
        })
        .collect();
    // 比較前に全証拠 (位置・順位・背景比・半値幅・競合比) の一致を確認する。
    for (source, prepared) in sources.iter().zip(&templates) {
        assert_eq!(
            BandNcc.locate(source, &capture, RATE),
            BandNcc.locate_prepared(source, &capture, RATE, prepared, &signal)
        );
        let (evidence, trace) =
            FullNcc.locate_coarse_to_fine(source, &capture, RATE, prepared, &signal);
        assert_eq!(evidence, FullNcc.locate(source, &capture, RATE));
        let trace = trace.unwrap();
        println!(
            "coarse_frames={} radius={} refined={:?} fallback={}",
            prepared.coarse_samples().unwrap().len(),
            trace.neighborhood_radius,
            trace.refined_positions,
            trace.fallback_reason
        );
    }
    measure("band-ncc-uncached-two-pairs", || {
        for source in sources {
            black_box(BandNcc.locate(source, &capture, RATE));
        }
    });
    measure("band-ncc-prepared-two-pairs", || {
        for (source, prepared) in sources.iter().zip(&templates) {
            black_box(BandNcc.locate_prepared(source, &capture, RATE, prepared, &signal));
        }
    });
    measure("coarse-ncc-two-pairs-proposal-only", || {
        for prepared in &templates {
            let t = prepared.coarse_samples().unwrap();
            let s = signal.coarse_samples().unwrap();
            let corr = al_locate::dsp::ncc(t, s, s.len() - t.len());
            black_box(al_locate::methods::peaks_from_correlation(&corr, 8));
        }
    });
    measure("full-ncc-two-pairs", || {
        for source in sources {
            black_box(FullNcc.locate(source, &capture, RATE));
        }
    });
    measure("coarse-fine-and-global-evidence-two-pairs", || {
        for (source, prepared) in sources.iter().zip(&templates) {
            black_box(FullNcc.locate_coarse_to_fine(source, &capture, RATE, prepared, &signal));
        }
    });
    // 準備と探索を同じ計測内で含める。再利用時の値と混同しない。
    measure("cold-prepared-band-and-full-two-pairs", || {
        let mut cache = InputCache::default();
        let signal = cache
            .prepare("capture", 0, RATE, "original", &capture)
            .unwrap();
        for (i, source) in sources.iter().enumerate() {
            let prepared = cache
                .prepare(&format!("source-{i}"), 0, RATE, "original", source)
                .unwrap();
            black_box(BandNcc.locate_prepared(source, &capture, RATE, &prepared, &signal));
            black_box(FullNcc.locate_coarse_to_fine(source, &capture, RATE, &prepared, &signal));
        }
    });
    measure("uncached-band-and-full-two-pairs", || {
        for source in sources {
            black_box(BandNcc.locate(source, &capture, RATE));
            black_box(FullNcc.locate(source, &capture, RATE));
        }
    });
}
