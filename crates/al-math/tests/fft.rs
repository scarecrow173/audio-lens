use al_math::fft::{Fft, RealFft};
use std::f64::consts::TAU;

// 既存 dsp::fft の radix-2 規則を独立した参照として保持する。
fn reference(re: &mut [f64], im: &mut [f64], inverse: bool) {
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
        let angle = if inverse { TAU } else { -TAU } / len as f64;
        let (step_re, step_im) = (angle.cos(), angle.sin());
        for (chunk_re, chunk_im) in re.chunks_exact_mut(len).zip(im.chunks_exact_mut(len)) {
            let (mut w_re, mut w_im) = (1.0, 0.0);
            for j in 0..len / 2 {
                let t_re = chunk_re[j + len / 2] * w_re - chunk_im[j + len / 2] * w_im;
                let t_im = chunk_re[j + len / 2] * w_im + chunk_im[j + len / 2] * w_re;
                let a_re = chunk_re[j];
                let a_im = chunk_im[j];
                chunk_re[j] = a_re + t_re;
                chunk_im[j] = a_im + t_im;
                chunk_re[j + len / 2] = a_re - t_re;
                chunk_im[j + len / 2] = a_im - t_im;
                // w *= step。複素乗算と同じ乗算・加算順を保つ。
                let next_re = w_re * step_re - w_im * step_im;
                w_im = w_re * step_im + w_im * step_re;
                w_re = next_re;
            }
        }
        len *= 2;
    }
    if inverse {
        for v in re.iter_mut() {
            *v /= n as f64;
        }
        for v in im.iter_mut() {
            *v /= n as f64;
        }
    }
}

fn close(
    actual_re: &[f64],
    actual_im: &[f64],
    expected_re: &[f64],
    expected_im: &[f64],
    tolerance: f64,
) {
    assert_eq!(actual_re.len(), actual_im.len());
    assert_eq!(actual_re.len(), expected_re.len());
    assert_eq!(actual_im.len(), expected_im.len());
    for (i, ((&ar, &ai), (&br, &bi))) in actual_re
        .iter()
        .zip(actual_im)
        .zip(expected_re.iter().zip(expected_im))
        .enumerate()
    {
        let diff = (ar - br).abs() + (ai - bi).abs();
        let norm = br.abs() + bi.abs();
        assert!(
            diff <= tolerance * (1.0 + norm),
            "bin {i}: ({ar}, {ai}) != ({br}, {bi})"
        );
    }
}

#[test]
fn complex_matches_legacy_forward_inverse_and_roundtrip() {
    for power in 0..=16 {
        let n = 1 << power;
        let input_re: Vec<_> = (0..n).map(|i| (i as f64 * 0.37).sin()).collect();
        let input_im: Vec<_> = (0..n).map(|i| (i as f64 * 0.71).cos()).collect();
        let plan = Fft::new(n);
        for inverse in [false, true] {
            let mut expected_re = input_re.clone();
            let mut expected_im = input_im.clone();
            reference(&mut expected_re, &mut expected_im, inverse);
            let mut actual_re = input_re.clone();
            let mut actual_im = input_im.clone();
            plan.transform(&mut actual_re, &mut actual_im, inverse);
            // 旧版の twiddle 逐次再帰は長さに応じて丸め誤差を蓄積する。
            close(
                &actual_re,
                &actual_im,
                &expected_re,
                &expected_im,
                1e-9_f64.max(n as f64 * 1e-12),
            );
        }
        let mut actual_re = input_re.clone();
        let mut actual_im = input_im.clone();
        plan.transform(&mut actual_re, &mut actual_im, false);
        plan.transform(&mut actual_re, &mut actual_im, true);
        close(&actual_re, &actual_im, &input_re, &input_im, 1e-10);
    }
}

#[test]
fn real_half_spectrum_matches_complex_and_roundtrips() {
    for power in 0..=16 {
        let n = 1 << power;
        let mut plan = RealFft::new(n);
        for input in [
            vec![0.0; n],
            vec![1.0; n],
            (0..n)
                .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
                .collect(),
            (0..n).map(|i| (i as f64 * 0.37).sin()).collect(),
        ] {
            let mut expected_re = input.clone();
            let mut expected_im = vec![0.0; n];
            reference(&mut expected_re, &mut expected_im, false);
            let mut spectrum_re = vec![0.0; n / 2 + 1];
            let mut spectrum_im = vec![0.0; n / 2 + 1];
            plan.forward(&input, &mut spectrum_re, &mut spectrum_im);
            close(
                &spectrum_re,
                &spectrum_im,
                &expected_re[..n / 2 + 1],
                &expected_im[..n / 2 + 1],
                1e-9_f64.max(n as f64 * 1e-12),
            );
            assert_eq!(spectrum_im[0], 0.0);
            if n > 1 {
                assert_eq!(spectrum_im[n / 2], 0.0);
            }
            let mut output = vec![0.0; n];
            plan.inverse(&spectrum_re, &spectrum_im, &mut output);
            for (a, b) in output.iter().zip(&input) {
                assert!((a - b).abs() < 1e-10);
            }
        }
    }
}

#[test]
#[should_panic]
fn rejects_non_power_of_two() {
    Fft::new(3);
}

#[test]
#[should_panic]
fn rejects_empty_transform() {
    Fft::new(0);
}

#[test]
#[should_panic]
fn rejects_wrong_buffer_length() {
    Fft::new(4).transform(&mut [0.0; 2], &mut [0.0; 2], false);
}

#[test]
#[should_panic]
fn rejects_wrong_half_spectrum_length() {
    RealFft::new(8).forward(&[0.0; 8], &mut [0.0; 8], &mut [0.0; 8]);
}

#[test]
fn matches_independent_dft_and_is_repeatable() {
    for n in [1, 2, 4, 8, 16, 32, 64] {
        let input_re: Vec<_> = (0..n).map(|i| (i as f64 * 0.37).sin()).collect();
        let input_im: Vec<_> = (0..n).map(|i| (i as f64 * 0.71).cos()).collect();
        let plan = Fft::new(n);
        for inverse in [false, true] {
            let (expected_re, expected_im): (Vec<_>, Vec<_>) = (0..n)
                .map(|k| {
                    let (mut sum_re, mut sum_im) = (0.0, 0.0);
                    for (j, (&xr, &xi)) in input_re.iter().zip(&input_im).enumerate() {
                        let a = (if inverse { TAU } else { -TAU }) * (j * k) as f64 / n as f64;
                        let (wr, wi) = (a.cos(), a.sin());
                        sum_re += xr * wr - xi * wi;
                        sum_im += xr * wi + xi * wr;
                    }
                    if inverse {
                        (sum_re / n as f64, sum_im / n as f64)
                    } else {
                        (sum_re, sum_im)
                    }
                })
                .unzip();
            let mut first_re = input_re.clone();
            let mut first_im = input_im.clone();
            let mut second_re = input_re.clone();
            let mut second_im = input_im.clone();
            plan.transform(&mut first_re, &mut first_im, inverse);
            plan.transform(&mut second_re, &mut second_im, inverse);
            close(&first_re, &first_im, &expected_re, &expected_im, 1e-11);
            assert_eq!(first_re, second_re);
            assert_eq!(first_im, second_im);
        }
    }
}

#[test]
fn real_inverse_ignores_endpoint_imaginary_parts() {
    let mut plan = RealFft::new(4);
    let mut actual = [0.0; 4];
    plan.inverse(&[4.0, 0.0, 0.0], &[12.0, 0.0, -7.0], &mut actual);
    assert_eq!(actual, [1.0; 4]);
}

#[test]
fn immutable_complex_plan_can_be_shared() {
    let plan = Fft::new(2048);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                let mut re = vec![0.0; 2048];
                let mut im = vec![0.0; 2048];
                re[0] = 1.0;
                plan.transform(&mut re, &mut im, false);
                assert_eq!(re, vec![1.0; 2048]);
                assert_eq!(im, vec![0.0; 2048]);
            });
        }
    });
}
