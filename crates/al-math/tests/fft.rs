use al_math::{
    c64,
    fft::{Fft, RealFft},
};
use std::f64::consts::TAU;

// 既存 dsp::fft の radix-2 規則を独立した参照として保持する。
fn reference(values: &mut [c64], inverse: bool) {
    let n = values.len();
    for i in 1..n {
        let j = i.reverse_bits() >> (usize::BITS - n.trailing_zeros());
        if i < j {
            values.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = if inverse { TAU } else { -TAU } / len as f64;
        let step = c64::new(angle.cos(), angle.sin());
        for chunk in values.chunks_exact_mut(len) {
            let mut w = c64::new(1.0, 0.0);
            for j in 0..len / 2 {
                let t = chunk[j + len / 2] * w;
                let a = chunk[j];
                chunk[j] = a + t;
                chunk[j + len / 2] = a - t;
                w *= step;
            }
        }
        len *= 2;
    }
    if inverse {
        for v in values {
            *v /= n as f64;
        }
    }
}

fn close(a: &[c64], b: &[c64], tolerance: f64) {
    assert_eq!(a.len(), b.len());
    for (i, (a, b)) in a.iter().zip(b).enumerate() {
        assert!(
            (*a - *b).l1_norm() <= tolerance * (1.0 + b.l1_norm()),
            "bin {i}: {a:?} != {b:?}"
        );
    }
}

#[test]
fn complex_matches_legacy_forward_inverse_and_roundtrip() {
    for power in 0..=16 {
        let n = 1 << power;
        let input: Vec<_> = (0..n)
            .map(|i| c64::new((i as f64 * 0.37).sin(), (i as f64 * 0.71).cos()))
            .collect();
        let plan = Fft::new(n);
        for inverse in [false, true] {
            let mut expected = input.clone();
            reference(&mut expected, inverse);
            let mut actual = input.clone();
            plan.transform(&mut actual, inverse);
            // 旧版の twiddle 逐次再帰は長さに応じて丸め誤差を蓄積する。
            close(&actual, &expected, 1e-9_f64.max(n as f64 * 1e-12));
        }
        let mut actual = input.clone();
        plan.transform(&mut actual, false);
        plan.transform(&mut actual, true);
        close(&actual, &input, 1e-10);
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
            let mut expected: Vec<_> = input.iter().map(|&v| c64::new(v, 0.0)).collect();
            reference(&mut expected, false);
            let mut spectrum = vec![c64::new(0.0, 0.0); n / 2 + 1];
            plan.forward(&input, &mut spectrum);
            close(
                &spectrum,
                &expected[..n / 2 + 1],
                1e-9_f64.max(n as f64 * 1e-12),
            );
            assert_eq!(spectrum[0].im, 0.0);
            if n > 1 {
                assert_eq!(spectrum[n / 2].im, 0.0);
            }
            let mut output = vec![0.0; n];
            plan.inverse(&spectrum, &mut output);
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
    Fft::new(4).transform(&mut [c64::new(0.0, 0.0); 2], false);
}

#[test]
#[should_panic]
fn rejects_wrong_half_spectrum_length() {
    RealFft::new(8).forward(&[0.0; 8], &mut [c64::new(0.0, 0.0); 8]);
}

#[test]
fn matches_independent_dft_and_is_repeatable() {
    for n in [1, 2, 4, 8, 16, 32, 64] {
        let input: Vec<_> = (0..n)
            .map(|i| c64::new((i as f64 * 0.37).sin(), (i as f64 * 0.71).cos()))
            .collect();
        let plan = Fft::new(n);
        for inverse in [false, true] {
            let expected: Vec<_> = (0..n)
                .map(|k| {
                    let mut sum = c64::new(0.0, 0.0);
                    for (j, &x) in input.iter().enumerate() {
                        let a = (if inverse { TAU } else { -TAU }) * (j * k) as f64 / n as f64;
                        sum += x * c64::new(a.cos(), a.sin());
                    }
                    if inverse { sum / n as f64 } else { sum }
                })
                .collect();
            let mut first = input.clone();
            let mut second = input.clone();
            plan.transform(&mut first, inverse);
            plan.transform(&mut second, inverse);
            close(&first, &expected, 1e-11);
            assert_eq!(first, second);
        }
    }
}

#[test]
fn real_inverse_ignores_endpoint_imaginary_parts() {
    let mut plan = RealFft::new(4);
    let mut actual = [0.0; 4];
    plan.inverse(
        &[c64::new(4.0, 12.0), c64::new(0.0, 0.0), c64::new(0.0, -7.0)],
        &mut actual,
    );
    assert_eq!(actual, [1.0; 4]);
}

#[test]
fn immutable_complex_plan_can_be_shared() {
    let plan = Fft::new(2048);
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                let mut values = vec![c64::new(0.0, 0.0); 2048];
                values[0] = c64::new(1.0, 0.0);
                plan.transform(&mut values, false);
                assert_eq!(values, vec![c64::new(1.0, 0.0); 2048]);
            });
        }
    });
}
