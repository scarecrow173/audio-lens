//! #57/#58 計測ファースト: cheap_analysis の支配要因を
//! 本番形状(50 s/48 kHz stereo、stft 2048/1024)で分解する。
//! envelope / matrix_to_tensor が支配的かを判定するための
//! kernel 別タイミング。
use std::hint::black_box;
use std::time::Instant;

use al_analysis::cheap::{CheapConfig, analyze_input};
use al_analysis::stft::{band_power, envelope, stft, third_octave_bands};
use al_locate::decode::DecodedInput;

const RATE: u32 = 48_000;
const SECS: usize = 50;

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

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let t = Instant::now();
    let r = f();
    println!("{label}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    r
}

fn main() {
    let n = RATE as usize * SECS;
    let cfg = CheapConfig::default();
    println!("input: {SECS}s stereo @{RATE} ({n} samples/ch), stft {}/{}",
        cfg.stft_window, cfg.stft_hop);

    // --- analyze_input 全体(実パイプラインと同じ処理) ---
    let dec = DecodedInput {
        input_id: "synthetic".into(),
        sample_rate: RATE,
        channels: 2,
        channel_samples: vec![noise(3, n), noise(5, n)],
        artifact_ids: Vec::new(),
    };
    let res = timed("analyze_input(total, 2ch)", || analyze_input(&dec, &cfg));
    println!("  records={} tensors={}", res.records.len(), res.tensors.len());

    // --- 単ch分の kernel 分解 ---
    let samples = &dec.channel_samples[0];
    let m = timed("stft(2048/1024, 1ch)", || {
        stft(samples, cfg.stft_window, cfg.stft_hop, RATE)
    });
    println!("  frames={} fft_size={}", m.frames(), m.fft_size);
    let pow = timed("m.power()", || m.power());
    let norm = (0.375 * m.window as f64) * m.sample_rate as f64;
    let inv = 1.0 / norm;
    let half = m.fft_size / 2;
    let mut mean_row = vec![0.0; half];
    let mag: Vec<Vec<f64>> = timed("mag derive + mean_row", || {
        pow.iter()
            .map(|row| {
                let r: Vec<f64> = row.iter().map(|&v| v.sqrt()).collect();
                for (k, &v) in r.iter().take(half).enumerate() {
                    mean_row[k] += v;
                }
                r
            })
            .collect()
    });
    timed("psd derive", || -> Vec<Vec<f64>> {
        pow.iter()
            .map(|row| row.iter().map(|&v| v * inv).collect())
            .collect()
    });
    timed("band_power cfg.bands", || -> Vec<Vec<f64>> {
        pow.iter()
            .map(|row| band_power(row, RATE, m.fft_size, &cfg.bands))
            .collect()
    });
    let third = third_octave_bands(20.0, (RATE as f64 / 2.0).min(20000.0));
    timed("band_power third_octave", || -> Vec<Vec<f64>> {
        pow.iter()
            .map(|row| band_power(row, RATE, m.fft_size, &third))
            .collect()
    });
    // #57 対象: envelope(mag 行、幅 8)
    timed("envelope(mag rows, w=8)", || -> Vec<Vec<f64>> {
        mag.iter().map(|r| envelope(r, 8)).collect()
    });
    // #58 対象: matrix_to_tensor 相当の flatten コピー
    for (label, mat) in [
        ("stft_re", &m.re),
        ("stft_im", &m.im),
        ("magnitude", &mag),
        ("power", &pow),
    ] {
        let bytes: usize = mat.iter().map(Vec::len).sum::<usize>() * 8;
        timed(&format!("flatten {label} ({:.1} MB)", bytes as f64 / 1e6), || {
            let mut data = Vec::with_capacity(mat.iter().map(Vec::len).sum());
            for row in mat {
                data.extend_from_slice(row);
            }
            black_box(data)
        });
    }
    // per-frame 形状ループ相当(centroid/flatness/rolloff/entropy/flux)
    timed("per-frame shape loop", || {
        let bin_hz = RATE as f64 / (half * 2) as f64;
        let mut prev: Option<Vec<f64>> = None;
        let mut acc = 0.0f64;
        for row in &mag {
            let row = &row[..half];
            let total: f64 = row.iter().sum();
            if total > 0.0 {
                acc += row
                    .iter()
                    .enumerate()
                    .map(|(b, &v)| b as f64 * bin_hz * v)
                    .sum::<f64>()
                    / total;
            }
            acc += (row.iter().map(|v| (v + 1e-12).ln()).sum::<f64>() / half as f64).exp();
            let h: f64 = row
                .iter()
                .filter(|&&v| v > 0.0)
                .map(|&v| (v / total.max(1e-30)) * (v / total.max(1e-30)).ln())
                .sum();
            acc += -h / (half as f64).ln();
            if let Some(p) = &prev {
                acc += row.iter().zip(p).map(|(&a, &b)| (a - b).max(0.0)).sum::<f64>();
            }
            prev = Some(row.to_vec());
        }
        black_box(acc)
    });
}
