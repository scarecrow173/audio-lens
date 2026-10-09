//! Temporary microbenchmark comparing al-locate DSP kernels against
//! the NumPy/SciPy reference numbers from the original toolchain.

use std::time::Instant;

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
        println!("{:40} {:9.1} ms", name, t0.elapsed().as_secs_f64() * 1000.0 / reps as f64);
    };

    bench("fft 4M (next_pow2)", 3, &mut || {
        let mut re = sig.clone();
        re.resize(1 << 22, 0.0);
        let mut im = vec![0.0; 1 << 22];
        al_locate::dsp::fft(&mut re, &mut im, false);
        std::hint::black_box(&re);
    });

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
        al_locate::dsp::fft(&mut re, &mut im, false);
        al_locate::dsp::fft(&mut sr, &mut si, false);
        for k in 0..n {
            let pr = re[k] * sr[k] + im[k] * si[k];
            let pi = im[k] * sr[k] - re[k] * si[k];
            re[k] = pr;
            im[k] = pi;
        }
        al_locate::dsp::fft(&mut re, &mut im, true);
        std::hint::black_box(&re);
    });

    bench("bandpass 2.4M (spectral mask)", 3, &mut || {
        let b = al_locate::dsp::bandpass(&sig, 48000, 200.0, 800.0);
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
            al_locate::dsp::fft(&mut fr, &mut fi, false);
            acc += fr[0];
            start += hop;
        }
        std::hint::black_box(acc);
    });

    bench("gcc_phat 2.4M x 8k", 3, &mut || {
        let p = al_locate::dsp::gcc_phat(&tpl, &sig, n_sig - n_tpl);
        std::hint::black_box(&p);
    });
}
