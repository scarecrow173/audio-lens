use al_math::{correlation, filter};

fn noise(n: usize) -> Vec<f64> {
    let mut state = 17u64;
    (0..n)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 32) as f64 / u32::MAX as f64 - 0.5
        })
        .collect()
}

#[test]
fn ncc_matches_direct_signed_window_correlation() {
    for len in [1, 3, 8, 17, 64] {
        let signal = noise(len * 3 + 1);
        let template: Vec<_> = signal[len..len * 2].iter().map(|x| -x).collect();
        let corr = correlation::ncc(&template, &signal, usize::MAX);
        assert_eq!(corr.len(), signal.len() - len + 1);
        let te: f64 = template.iter().map(|x| x * x).sum();
        for (lag, &actual) in corr.iter().enumerate() {
            let window = &signal[lag..lag + len];
            let se: f64 = window.iter().map(|x| x * x).sum();
            let dot: f64 = template.iter().zip(window).map(|(a, b)| a * b).sum();
            assert!((actual - dot / (te * se).sqrt()).abs() < 1e-10);
        }
        assert!((corr[len] + 1.0).abs() < 1e-10);
    }
    assert!(correlation::ncc(&[], &[1.0], 0).is_empty());
    assert!(correlation::ncc(&[1.0, 2.0], &[1.0], 0).is_empty());
    assert_eq!(correlation::ncc(&[0.0; 3], &[1.0; 8], 2), vec![0.0; 3]);
    assert_eq!(correlation::ncc(&[1.0; 3], &[0.0; 8], 2), vec![0.0; 3]);
}

#[test]
fn gcc_phat_retains_polarity_and_lag_direction() {
    let signal = noise(257);
    let template: Vec<_> = signal[123..156].iter().map(|x| -x).collect();
    let corr = correlation::gcc_phat(&template, &signal, usize::MAX);
    let best = corr
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
        .unwrap();
    assert_eq!(best.0, 123);
    assert!(*best.1 < 0.0);
    assert_eq!(correlation::gcc_phat(&[0.0; 3], &[0.0; 8], 2), vec![0.0; 3]);
}

#[test]
fn gcc_phat_does_not_apply_ncc_energy_gate() {
    // PHAT の非ゼロ cross-spectrum を、素材の二乗 underflow で棄却しない。
    assert_eq!(correlation::gcc_phat(&[1e-200], &[1e200], 0), vec![1.0]);
    assert_eq!(correlation::gcc_phat(&[-1e-200], &[1e200], 0), vec![-1.0]);
}

#[test]
fn shared_bandpass_preserves_mask_boundaries_and_endpoints() {
    let n = 256;
    let input: Vec<_> = (0..n)
        .map(|i| {
            let phase = std::f64::consts::TAU * i as f64 / n as f64;
            1.0 + (10.0 * phase).sin() + (20.0 * phase).sin() + if i % 2 == 0 { 1.0 } else { -1.0 }
        })
        .collect();
    let bands = [(10.0, 20.0), (0.0, 10.0), (128.0, 129.0)];
    let output = filter::bandpass_many(&input, 256, &bands);
    for (i, band) in output.iter().enumerate() {
        assert_eq!(band.len(), n);
        assert_eq!(*band, filter::bandpass(&input, 256, bands[i].0, bands[i].1));
        for (j, &sample) in band.iter().enumerate() {
            let expected = match i {
                0 => (std::f64::consts::TAU * 10.0 * j as f64 / n as f64).sin(),
                1 => 1.0,
                _ => {
                    if j % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
            };
            assert!((sample - expected).abs() < 1e-10);
        }
    }
    assert_eq!(
        filter::bandpass_many(&[], 48000, &bands),
        vec![Vec::<f64>::new(); 3]
    );
    assert!(filter::bandpass_many(&input, 256, &[]).is_empty());
    assert_eq!(filter::bandpass(&[2.0], 256, 0.0, 1.0), vec![2.0]);
}

fn complex_spectrum(input: &[f64], n: usize) -> (Vec<f64>, Vec<f64>) {
    let mut re = vec![0.0; n];
    let mut im = vec![0.0; n];
    for (z, &value) in re.iter_mut().zip(input) {
        *z = value;
    }
    al_math::fft::Fft::new(n).transform(&mut re, &mut im, false);
    (re, im)
}

#[test]
fn real_kernels_preserve_previous_complex_spectrum_contract() {
    for len in [1, 3, 17, 67, 256] {
        let input = noise(len);
        let n = len.next_power_of_two();
        for (lo, hi) in [(0.0, 128.0), (10.0, 60.0), (128.0, 129.0), (70.0, 60.0)] {
            let (mut spectrum_re, mut spectrum_im) = complex_spectrum(&input, n);
            for k in 0..n {
                let f = 256.0 / n as f64 * k.min(n - k) as f64;
                if f < lo || f >= hi {
                    spectrum_re[k] = 0.0;
                    spectrum_im[k] = 0.0;
                }
            }
            al_math::fft::Fft::new(n).transform(&mut spectrum_re, &mut spectrum_im, true);
            let actual = filter::bandpass(&input, 256, lo, hi);
            for (a, b) in actual.iter().zip(&spectrum_re) {
                assert!((a - b).abs() < 1e-10);
            }
        }
        let template = &input[..len.min(7)];
        let n = (input.len() + template.len() - 1).next_power_of_two();
        let (mut a_re, mut a_im) = complex_spectrum(template, n);
        let (b_re, b_im) = complex_spectrum(&input, n);
        for k in 0..n {
            // cross = conj(a) * b = (ar·br + ai·bi, ar·bi − ai·br)
            let cross_re = a_re[k] * b_re[k] + a_im[k] * b_im[k];
            let cross_im = a_re[k] * b_im[k] - a_im[k] * b_re[k];
            let mag = (cross_re * cross_re + cross_im * cross_im).sqrt();
            if mag > 0.0 {
                a_re[k] = cross_re / mag;
                a_im[k] = cross_im / mag;
            } else {
                a_re[k] = 0.0;
                a_im[k] = 0.0;
            }
        }
        al_math::fft::Fft::new(n).transform(&mut a_re, &mut a_im, true);
        let actual = correlation::gcc_phat(template, &input, usize::MAX);
        for (a, b) in actual.iter().zip(&a_re) {
            assert!((a - b).abs() < 1e-10);
        }

        let (spectrum_re, spectrum_im) = complex_spectrum(&input, 256);
        let actual = filter::spectral_envelope(&input, 256, 17);
        for (bin, &a) in actual.iter().enumerate() {
            let lo = bin * 128 / 17;
            let hi = ((bin + 1) * 128 / 17).max(lo + 1).min(128);
            let mean = (lo..hi)
                .map(|k| (spectrum_re[k] * spectrum_re[k] + spectrum_im[k] * spectrum_im[k]).sqrt())
                .sum::<f64>()
                / (hi - lo) as f64;
            assert!((a - (mean + 1e-12).ln()).abs() < 1e-10);
        }
    }
}
