//! アンチエイリアス付き、有理レート比の polyphase FIR。
//! 対称 FIR の中心を出力時刻に置くことで群遅延を補償する。

use std::collections::HashMap;
use std::f64::consts::PI;

use crate::{Simd, WithSimd, dispatch};

/// 係数を完全に再生成できる変換設定。端点はゼロ拡張、長さは ceil。
#[derive(Debug, Clone, PartialEq)]
pub struct ResamplePlan {
    source_rate: u32,
    target_rate: u32,
    up: u32,
    down: u32,
    radius: usize,
    cutoff: f64,
}

impl ResamplePlan {
    pub const ALGORITHM: &str = "polyphase-blackman-sinc";
    pub const VERSION: &str = "1";

    pub fn new(source_rate: u32, target_rate: u32) -> Result<Self, &'static str> {
        if source_rate == 0 || target_rate == 0 {
            return Err("sample rates must be nonzero");
        }
        let (mut a, mut b) = (source_rate, target_rate);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let ratio = (target_rate as f64 / source_rate as f64).min(1.0);
        Ok(Self {
            source_rate,
            target_rate,
            up: target_rate / a,
            down: source_rate / a,
            radius: (32.0 / ratio).ceil() as usize,
            cutoff: 0.94 * ratio,
        })
    }

    pub fn source_rate(&self) -> u32 {
        self.source_rate
    }
    pub fn target_rate(&self) -> u32 {
        self.target_rate
    }
    pub fn up(&self) -> u32 {
        self.up
    }
    pub fn down(&self) -> u32 {
        self.down
    }
    pub fn radius(&self) -> usize {
        self.radius
    }
    pub fn cutoff(&self) -> f64 {
        self.cutoff
    }

    pub fn output_len(&self, frames: usize) -> Result<usize, &'static str> {
        usize::try_from((frames as u128 * self.up as u128).div_ceil(self.down as u128))
            .map_err(|_| "resampled length overflow")
    }

    fn coefficients(&self, phase: u32) -> Vec<f64> {
        let fraction = phase as f64 / self.up as f64;
        let radius = self.radius as isize;
        let mut coefficients: Vec<f64> = (-radius..=radius)
            .map(|k| {
                let x = k as f64 - fraction;
                let w = x / self.radius as f64;
                if w.abs() > 1.0 {
                    return 0.0;
                }
                let window = 0.42 + 0.5 * (PI * w).cos() + 0.08 * (2.0 * PI * w).cos();
                let z = PI * self.cutoff * x;
                let sinc = if z == 0.0 { 1.0 } else { z.sin() / z };
                self.cutoff * sinc * window
            })
            .collect();
        let sum: f64 = coefficients.iter().sum();
        for c in &mut coefficients {
            *c /= sum;
        }
        coefficients
    }
}

struct Resample<'a> {
    input: &'a [f64],
    plan: &'a ResamplePlan,
    length: usize,
}

impl WithSimd for Resample<'_> {
    type Output = Vec<f64>;

    #[inline(always)]
    fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
        let mut phases = HashMap::new();
        let taps = 2 * self.plan.radius + 1;
        let mut values = vec![0.0; taps];
        let mut products = vec![0.0; taps];
        let mut output = Vec::with_capacity(self.length);
        for n in 0..self.length {
            let position = n as u128 * self.plan.down as u128;
            let center = (position / self.plan.up as u128) as i128;
            let phase = (position % self.plan.up as u128) as u32;
            let coefficients = phases
                .entry(phase)
                .or_insert_with(|| self.plan.coefficients(phase));
            for (i, value) in values.iter_mut().enumerate() {
                let index = center + i as i128 - self.plan.radius as i128;
                *value = usize::try_from(index)
                    .ok()
                    .and_then(|i| self.input.get(i))
                    .copied()
                    .unwrap_or(0.0);
            }
            let (a, at) = S::as_simd_f64s(&values);
            let (b, bt) = S::as_simd_f64s(coefficients);
            let (p, pt) = S::as_mut_simd_f64s(&mut products);
            for ((a, b), p) in a.iter().zip(b).zip(p) {
                *p = simd.mul_f64s(*a, *b);
            }
            for ((a, b), p) in at.iter().zip(bt).zip(pt) {
                *p = a * b;
            }
            // ISA に依存しない順序で加算する。FMA を使わない。
            output.push(products.iter().sum());
        }
        output
    }
}

/// レート一致はサンプルをそのまま複製し、波形を一切変えない。
pub fn resample(input: &[f64], plan: &ResamplePlan) -> Result<Vec<f64>, &'static str> {
    if plan.source_rate == plan.target_rate {
        return Ok(input.to_vec());
    }
    Ok(dispatch(Resample {
        input,
        plan,
        length: plan.output_len(input.len())?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_empty_and_scalar_dispatch() {
        for n in [0, 1, 7, 33, 257] {
            let input: Vec<f64> = (0..n).map(|i| (i as f64 * 0.37).sin()).collect();
            assert_eq!(
                resample(&input, &ResamplePlan::new(48000, 48000).unwrap()).unwrap(),
                input
            );
            for (rs, rn) in [(44100, 48000), (48000, 44100), (48000, 12000)] {
                let plan = ResamplePlan::new(rs, rn).unwrap();
                let scalar = pulp::Scalar.vectorize(Resample {
                    input: &input,
                    plan: &plan,
                    length: plan.output_len(n).unwrap(),
                });
                assert_eq!(resample(&input, &plan).unwrap(), scalar);
            }
        }
        assert!(ResamplePlan::new(0, 48000).is_err());
    }

    #[test]
    fn suppresses_aliases_preserves_passband_and_compensates_delay() {
        let plan = ResamplePlan::new(48000, 12000).unwrap();
        for (hz, expected) in [(1000.0, true), (10000.0, false)] {
            let input: Vec<f64> = (0..4800)
                .map(|i| (2.0 * PI * hz * i as f64 / 48000.0).sin())
                .collect();
            let out = resample(&input, &plan).unwrap();
            let rms = (out[100..1100].iter().map(|v| v * v).sum::<f64>() / 1000.0).sqrt();
            if expected {
                assert!((rms - std::f64::consts::FRAC_1_SQRT_2).abs() < 0.003);
            } else {
                assert!(rms < 1e-4, "alias rms {rms}");
            }
        }
        let mut impulse = vec![0.0; 441];
        impulse[147] = 1.0;
        let out = resample(&impulse, &ResamplePlan::new(44100, 48000).unwrap()).unwrap();
        assert_eq!(
            out.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .unwrap()
                .0,
            160
        );
    }

    #[test]
    fn suppresses_upsampling_images_and_keeps_overshoot() {
        let plan = ResamplePlan::new(12000, 48000).unwrap();
        let input: Vec<f64> = (0..1200)
            .map(|i| (2.0 * PI * 1000.0 * i as f64 / 12000.0).sin())
            .collect();
        let output = resample(&input, &plan).unwrap();
        let amplitude = |hz: f64| {
            let (re, im) =
                output[400..4400]
                    .iter()
                    .enumerate()
                    .fold((0.0, 0.0), |(re, im), (i, v)| {
                        let phase = 2.0 * PI * hz * (400 + i) as f64 / 48000.0;
                        (re + v * phase.cos(), im + v * phase.sin())
                    });
            re.hypot(im) * 2.0 / 4000.0
        };
        assert!((amplitude(1000.0) - 1.0).abs() < 0.005);
        assert!(amplitude(11000.0) < 0.002);
        let mut step = vec![0.0; 200];
        step[100..].fill(1.0);
        let output = resample(&step, &plan).unwrap();
        assert!(output.iter().any(|v| *v > 1.05));
        assert!(output.iter().any(|v| *v < -0.05));
    }
}
