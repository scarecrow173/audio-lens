//! 同一プロセス・入力で移行前の spectral mask と新しい実数 FFT 経路を測る。
use al_math::{c64, correlation, fft::Fft, filter};
use std::{hint::black_box, time::Instant};

// 移行直前の al-locate::dsp の split-complex FFT wrapper。
fn previous_fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let plan = Fft::new(re.len());
    let mut values: Vec<_> = re
        .iter()
        .zip(im.iter())
        .map(|(&r, &i)| c64::new(r, i))
        .collect();
    plan.transform(&mut values, inverse);
    for ((r, i), value) in re.iter_mut().zip(im.iter_mut()).zip(values) {
        *r = value.re;
        *i = value.im;
    }
}

// SIMD 基盤導入前の radix-2 FFT。応答確認と比較測定専用。
fn original_fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    for i in 1..n {
        let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = if inverse { 1.0 } else { -1.0 } * std::f64::consts::TAU / len as f64;
        let (wr, wi) = (angle.cos(), angle.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for j in i..i + len / 2 {
                let k = j + len / 2;
                let (tr, ti) = (re[k] * cr - im[k] * ci, re[k] * ci + im[k] * cr);
                re[k] = re[j] - tr;
                im[k] = im[j] - ti;
                re[j] += tr;
                im[j] += ti;
                let next = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = next;
            }
        }
        len *= 2;
    }
    if inverse {
        for (r, i) in re.iter_mut().zip(im) {
            *r /= n as f64;
            *i /= n as f64;
        }
    }
}

fn legacy_bandpass(input: &[f64], lo: f64, hi: f64, original: bool) -> Vec<f64> {
    let n = input.len().next_power_of_two();
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    re[..input.len()].copy_from_slice(input);
    let fft = if original { original_fft } else { previous_fft };
    fft(&mut re, &mut im, false);
    let bin_hz = 48000.0 / n as f64;
    for k in 0..n {
        let frequency = bin_hz * k.min(n - k) as f64;
        if frequency < lo || frequency >= hi {
            re[k] = 0.0;
            im[k] = 0.0;
        }
    }
    fft(&mut re, &mut im, true);
    re.truncate(input.len());
    re
}

fn bench<T>(name: &str, mut f: impl FnMut() -> T) -> f64 {
    black_box(f());
    let start = Instant::now();
    for _ in 0..3 {
        black_box(f());
    }
    let ms = start.elapsed().as_secs_f64() * 1000.0 / 3.0;
    println!("{name:44} {ms:9.1} ms");
    ms
}

fn main() {
    let mut state = 17u64;
    let input: Vec<_> = (0..2_400_000)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 32) as f64 / u32::MAX as f64 - 0.5
        })
        .collect();
    let reference = legacy_bandpass(&input, 200.0, 800.0, true);
    let actual = filter::bandpass(&input, 48000, 200.0, 800.0);
    let error = actual
        .iter()
        .zip(&reference)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(error < 1e-8, "response mismatch: {error}");
    println!("max absolute response error: {error:.3e}");
    drop((reference, actual));
    let original = bench("original radix-2 bandpass 2.4M", || {
        legacy_bandpass(&input, 200.0, 800.0, true)
    });
    let previous = bench("previous dispatched complex bandpass 2.4M", || {
        legacy_bandpass(&input, 200.0, 800.0, false)
    });
    let current = bench("real spectral-mask bandpass 2.4M", || {
        filter::bandpass(&input, 48000, 200.0, 800.0)
    });
    let bands = [
        (20.0, 250.0),
        (250.0, 1000.0),
        (1000.0, 4000.0),
        (4000.0, 16000.0),
        (16000.0, 24000.0),
    ];
    let previous_many = bench("previous five independent bands 2.4M", || {
        bands
            .iter()
            .map(|&(lo, hi)| legacy_bandpass(&input, lo, hi, false))
            .collect::<Vec<_>>()
    });
    let many = bench("shared forward FFT five bands 2.4M", || {
        filter::bandpass_many(&input, 48000, &bands)
    });
    println!(
        "speedup original/current: {:.2}x, previous/current: {:.2}x, five bands: {:.2}x",
        original / current,
        previous / current,
        previous_many / many
    );
    bench("NCC 2.4M x 8k", || {
        correlation::ncc(&input[8192..16384], &input, usize::MAX)
    });
    bench("GCC-PHAT 2.4M x 8k", || {
        correlation::gcc_phat(&input[8192..16384], &input, usize::MAX)
    });
}
