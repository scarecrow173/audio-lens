//! Temporary microbenchmark comparing al-locate DSP kernels against
//! the NumPy/SciPy reference numbers from the original toolchain.

use std::time::Instant;

/// 単発呼出の計測形状を旧 `dsp::fft` と揃えるためのローカル束線。
/// plan 生成と transform を同一計時区間に含める。
fn ruviform_fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let mut fft =
        ruviform_numeric::fft::Fft::new(re.len()).expect("fft length must be a power of two");
    fft.transform(re, im, inverse)
        .expect("fft buffer lengths must match the plan");
}

// 移行前の演算を同じプロセス・CPU・入力で比較するための参照。
// 製品の DSP 入口には使用しない。
fn legacy_fft(re: &mut [f64], im: &mut [f64]) {
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
        let ang = -2.0 * std::f64::consts::PI / len as f64;
        let (wr, wi) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for j in i..i + len / 2 {
                let k = j + len / 2;
                let (tr, ti) = (re[k] * cr - im[k] * ci, re[k] * ci + im[k] * cr);
                re[k] = re[j] - tr;
                im[k] = im[j] - ti;
                re[j] += tr;
                im[j] += ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len *= 2;
    }
}

fn main() {
    let n_sig = 2_400_000usize;
    let n_tpl = 8192usize;
    let mut rng: u64 = 0x9e3779b97f4a7c15;
    let mut rand = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng as f64 / u64::MAX as f64) * 2.0 - 1.0
    };
    let sig: Vec<f64> = (0..n_sig).map(|_| rand()).collect();
    let tpl: Vec<f64> = (0..n_tpl).map(|_| rand()).collect();

    let bench = |name: &str, reps: usize, f: &mut dyn FnMut()| {
        f();
        let t0 = Instant::now();
        for _ in 0..reps {
            f();
        }
        println!(
            "{:40} {:9.1} ms",
            name,
            t0.elapsed().as_secs_f64() * 1000.0 / reps as f64
        );
    };

    bench("legacy fft 4M (same process)", 3, &mut || {
        let mut re = sig.clone();
        re.resize(1 << 22, 0.0);
        let mut im = vec![0.0; 1 << 22];
        legacy_fft(&mut re, &mut im);
        std::hint::black_box(&re);
    });
    bench("fft 4M (next_pow2)", 3, &mut || {
        let mut re = sig.clone();
        re.resize(1 << 22, 0.0);
        let mut im = vec![0.0; 1 << 22];
        ruviform_fft(&mut re, &mut im, false);
        std::hint::black_box(&re);
    });

    // 同じ実数入力を、処理コンテキストの係数/scratch 再利用で測る。
    let n = 1 << 22;
    let mut real = sig.clone();
    real.resize(n, 0.0);
    let mut plan = ruviform_numeric::fft::Fft::new(n).unwrap();
    let mut cre = vec![0.0; n];
    let mut cim = vec![0.0; n];
    bench("complex fft 4M (reused plan)", 7, &mut || {
        cre.copy_from_slice(&real);
        cim.iter_mut().for_each(|v| *v = 0.0);
        plan.transform(&mut cre, &mut cim, false).unwrap();
        std::hint::black_box(&cre);
    });
    let mut real_plan = ruviform_numeric::fft::RealFft::new(n).unwrap();
    let mut spec_re = vec![0.0; n / 2 + 1];
    let mut spec_im = vec![0.0; n / 2 + 1];
    bench("rfft 4M (reused plan/scratch)", 7, &mut || {
        real_plan
            .forward(&real, &mut spec_re, &mut spec_im)
            .unwrap();
        std::hint::black_box(&spec_re);
    });
    drop((plan, cre, cim, real_plan, real, spec_re, spec_im));

    bench("NCC 2.4M x 8k (fft corr)", 3, &mut || {
        // Same FFT-correlation the locate methods use: gcc_phat's
        // inner transform without the whitening, i.e. conv via FFT.
        let n = (n_sig + n_tpl).next_power_of_two();
        let mut re = vec![0.0; n];
        let mut im = vec![0.0; n];
        let mut sr = vec![0.0; n];
        let mut si = vec![0.0; n];
        re[..n_sig].copy_from_slice(&sig);
        sr[..n_tpl].copy_from_slice(&tpl);
        ruviform_fft(&mut re, &mut im, false);
        ruviform_fft(&mut sr, &mut si, false);
        for k in 0..n {
            let pr = re[k] * sr[k] + im[k] * si[k];
            let pi = im[k] * sr[k] - re[k] * si[k];
            re[k] = pr;
            im[k] = pi;
        }
        ruviform_fft(&mut re, &mut im, true);
        std::hint::black_box(&re);
    });

    bench("bandpass 2.4M (spectral mask)", 3, &mut || {
        let b = ruviform_numeric::bands::bandpass(&sig, 48000, 200.0, 800.0)
            .expect("band settings and input must be valid");
        std::hint::black_box(&b);
    });

    bench("stft-equivalent: welch-ish psd 2.4M", 3, &mut || {
        // 4096-window hop 2048 frames, fft each.
        let win = 4096usize;
        let hop = 2048usize;
        let mut acc = 0.0f64;
        let mut fr = vec![0.0f64; win];
        let mut fi = vec![0.0f64; win];
        let mut start = 0usize;
        while start + win <= n_sig {
            fr[..win].copy_from_slice(&sig[start..start + win]);
            fi.iter_mut().for_each(|v| *v = 0.0);
            ruviform_fft(&mut fr, &mut fi, false);
            acc += fr[0];
            start += hop;
        }
        std::hint::black_box(acc);
    });

    bench("gcc_phat 2.4M x 8k", 3, &mut || {
        let p = ruviform_numeric::corr::gcc_phat(&tpl, &sig, n_sig - n_tpl)
            .expect("correlation template/signal shapes must be valid");
        std::hint::black_box(&p);
    });

    // ==== mechanical-caller-cleanup A15 (TASK-034EO484s25JM8bsb6zUCh) ====
    // deinterleave/interleave の Instant 計時区間 — 既存測定経路に
    // 載っていなかったため追加(plan T1)。A15-opt の対称腕:
    // `decode.rs` の現行 element-loop と channel-block strided gather
    // (`chunks_exact`)、normalize/reference の frame-major push
    // loop と `n*c` 事前確保 indexed write / staged-row
    // `extend_from_slice`。byte/element 完全一致は
    // `src/decode.rs`/`tests/locate.rs` の characterization で gate。
    let channels = 2usize;
    let frames = 1_000_000usize;
    let interleaved: Vec<f64> = (0..frames * channels).map(|_| rand()).collect();
    let chans: Vec<Vec<f64>> = (0..channels)
        .map(|c| (0..frames).map(|i| interleaved[i * channels + c]).collect())
        .collect();

    bench("deinterleave 2ch x 1M (current)", 5, &mut || {
        let d = al_locate::deinterleave(&interleaved, channels as u16);
        std::hint::black_box(&d);
    });
    bench(
        "deinterleave 2ch x 1M (channel-block gather)",
        5,
        &mut || {
            // A15 候補形状: チャンネル毎に chunks_exact ストライド gather。
            let fr = interleaved.len() / channels;
            let mut out: Vec<Vec<f64>> = (0..channels).map(|_| Vec::with_capacity(fr)).collect();
            for (c, o) in out.iter_mut().enumerate() {
                for frame in interleaved.chunks_exact(channels) {
                    o.push(frame[c]);
                }
            }
            std::hint::black_box(&out);
        },
    );

    bench("interleave 2ch x 1M (frame-major loop)", 5, &mut || {
        // normalize.rs:168-171 / reference.rs:1163-1170 の現行形状。
        let mut v = Vec::with_capacity(frames * channels);
        for i in 0..frames {
            for ch in &chans {
                v.push(ch[i]);
            }
        }
        std::hint::black_box(&v);
    });
    bench("interleave 2ch x 1M (indexed write)", 5, &mut || {
        // A15 候補形状: `n*c` 事前確保 + indexed write。
        let mut v = vec![0.0; frames * channels];
        for (c, ch) in chans.iter().enumerate() {
            for (i, &x) in ch.iter().enumerate() {
                v[i * channels + c] = x;
            }
        }
        std::hint::black_box(&v);
    });
    bench("interleave 2ch x 1M (staged-row extend)", 5, &mut || {
        // A15 候補形状: frame row buffer → extend_from_slice。
        let mut row = vec![0.0; channels];
        let mut v = Vec::with_capacity(frames * channels);
        for i in 0..frames {
            for (c, ch) in chans.iter().enumerate() {
                row[c] = ch[i];
            }
            v.extend_from_slice(&row);
        }
        std::hint::black_box(&v);
    });
}
