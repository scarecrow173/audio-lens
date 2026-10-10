//! AudioLens の算術型と実行時 SIMD dispatch 基盤。
//!
//! カーネルは [`Simd`] に対して単一ソースで実装し、演算の公開入口で
//! [`dispatch`] を一度呼ぶ。通常の利用側には f64 slice を公開し、
//! ISA 検出やレーン幅の扱いはこのクレート内に留める。

use std::sync::LazyLock;

pub mod fft;
pub mod linalg;

pub use pulp::{Simd, WithSimd, c64};

/// カーネルが利用する、実行時に選択された f64 ベクトル型。
pub type F64s<S> = <S as Simd>::f64s;
/// カーネルが利用する、実行時に選択された複素 f64 ベクトル型。
pub type C64s<S> = <S as Simd>::c64s;

// CPU 検出はプロセス内で一度だけ行い、並列の演算入口でも共有する。
static ARCH: LazyLock<pulp::Arch> = LazyLock::new(pulp::Arch::new);

/// 共有の CPU 検出結果を使ってカーネルを実行する。
///
/// 利用可能な ISA がない環境では同じ [`WithSimd`] 実装が scalar で動く。
/// 内側のループから繰り返し呼ばず、演算入口で呼び出す。
pub fn dispatch<K: WithSimd>(kernel: K) -> K::Output {
    ARCH.dispatch(kernel)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scale<'a> {
        values: &'a mut [f64],
        factor: f64,
    }

    impl WithSimd for Scale<'_> {
        type Output = usize;

        #[inline(always)]
        fn with_simd<S: Simd>(self, simd: S) -> Self::Output {
            let (vectors, tail) = S::as_mut_simd_f64s(self.values);
            let factor: F64s<S> = simd.splat_f64s(self.factor);
            for value in vectors {
                *value = simd.mul_f64s(*value, factor);
            }
            for value in tail {
                *value *= self.factor;
            }
            std::mem::size_of::<F64s<S>>() / std::mem::size_of::<f64>()
        }
    }

    #[test]
    fn dispatch_matches_scalar_for_empty_short_full_and_tail_inputs() {
        // 0..=33 は空・短入力、複数のベクトルと端数を含む。
        for length in 0..=33 {
            let input: Vec<f64> = (0..length).map(|i| (i as f64 - 16.0) / 8.0).collect();
            for factor in [0.0, -2.0, 0.125, 3.25] {
                let expected: Vec<f64> = input.iter().map(|value| value * factor).collect();
                let mut native = input.clone();
                let mut scalar = input.clone();
                let native_lanes = dispatch(Scale {
                    values: &mut native,
                    factor,
                });
                let scalar_lanes = pulp::Scalar.vectorize(Scale {
                    values: &mut scalar,
                    factor,
                });
                assert!(native_lanes >= 1);
                if length == 33 && factor == 3.25 {
                    eprintln!(
                        "dispatch f64 lanes: {native_lanes}, scalar f64 lanes: {scalar_lanes}"
                    );
                }
                assert_eq!(scalar_lanes, 1);
                assert_eq!(native, expected);
                assert_eq!(scalar, expected);
            }
        }
    }

    #[test]
    fn dispatch_can_be_used_concurrently() {
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    let mut values = [0.5; 17];
                    dispatch(Scale {
                        values: &mut values,
                        factor: -2.0,
                    });
                    assert_eq!(values, [-1.0; 17]);
                });
            }
        });
    }
}
