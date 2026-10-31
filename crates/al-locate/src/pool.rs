//! ワーカー単位で所有する形状キー相関プール。
//!
//! `corr::ncc`/`corr::gcc_phat` フリー関数は呼出ごとに
//! `CorrelationPlan`(FFT twiddle 構築)と `CorrelationWorkspace`
//! (O(n) バッファ群)を再構築し、返却のために曲線全体を `.to_vec()`
//! で複製する。locate のメソッド経路は同一 (template_len, signal_len)
//! の呼出を繰り返すため、形状をキーに plan + workspace を再利用する。
//!
//! 借用セマンティクスは `CorrelationPlan::ncc` の契約をそのまま継承
//! する: 返却曲線はプール内の workspace を借用するため、曲線を保持
//! したまま次の呼出はできない。プールの同時メモリは
//! `MAX_SHAPES` で上限を設け、超過時は全エントリを破棄して再構築
//! する(最悪でも呼出ごと再構築の現状と同等)。

use std::collections::HashMap;

use ruviform_numeric::corr::{CorrelationPlan, CorrelationWorkspace};

/// pool が同時に保持する形状数の上限。帯域フィルタは長さを維持する
/// ため通常の pair は full-rate と coarse の 2 形状に収束する。
const MAX_SHAPES: usize = 8;

struct Entry {
    plan: CorrelationPlan,
    workspace: CorrelationWorkspace,
}

/// plan の演算範囲。NCC は有効ラグ域の短縮 FFT(`new_ncc`)、
/// PHAT は FFT 長依存の位相正規化のため従来の全域 FFT(`new`)を使う。
/// 同一 (template_len, signal_len) でも mode ごとに別 plan を持つ。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum PoolMode {
    Ncc,
    Phat,
}

/// pool の計測値。`builds` は plan/workspace を新規構築した回数。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PoolStats {
    pub builds: u64,
    pub hits: u64,
}

/// (template_len, signal_len, mode) をキーに plan/workspace を再利用する。
/// 呼出側(ワーカー)が排他所有し、スレッド間で共有しない。
#[derive(Default)]
pub struct CorrelationPool {
    entries: HashMap<(usize, usize, PoolMode), Entry>,
    builds: u64,
    hits: u64,
}

impl CorrelationPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// `signal[lag..lag + template.len()]` との NCC。返却曲線は
    /// pool を借用し、保持したまま pool を再使用できない。
    pub fn ncc<'w>(&'w mut self, template: &[f64], signal: &[f64], max_lag: usize) -> &'w [f64] {
        let entry = self.entry(template.len(), signal.len(), PoolMode::Ncc);
        entry
            .plan
            .ncc(&mut entry.workspace, template, signal, max_lag)
            .expect("correlation template/signal shapes must be valid")
            .values
    }

    /// 同じ形状キー(Phat mode)の plan で PHAT を実行する。
    pub fn gcc_phat<'w>(
        &'w mut self,
        template: &[f64],
        signal: &[f64],
        max_lag: usize,
    ) -> &'w [f64] {
        let entry = self.entry(template.len(), signal.len(), PoolMode::Phat);
        entry
            .plan
            .gcc_phat(&mut entry.workspace, template, signal, max_lag)
            .expect("correlation template/signal shapes must be valid")
    }

    /// 現在有効な形状エントリ数。
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// 構築回数と命中回数の計測値。
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            builds: self.builds,
            hits: self.hits,
        }
    }

    fn entry(&mut self, template_len: usize, signal_len: usize, mode: PoolMode) -> &mut Entry {
        let key = (template_len, signal_len, mode);
        if !self.entries.contains_key(&key) {
            if self.entries.len() >= MAX_SHAPES {
                self.entries.clear();
            }
            let plan = match mode {
                PoolMode::Ncc => CorrelationPlan::new_ncc(template_len, signal_len),
                PoolMode::Phat => CorrelationPlan::new(template_len, signal_len),
            }
            .expect("correlation plan shape must be valid");
            let workspace = CorrelationWorkspace::new(&plan);
            self.entries.insert(key, Entry { plan, workspace });
            self.builds += 1;
        } else {
            self.hits += 1;
        }
        self.entries.get_mut(&key).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruviform_numeric::corr;

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

    #[test]
    fn same_shape_is_built_once_and_matches_the_free_function() {
        let template = noise(3, 512);
        let signal = noise(7, 4096);
        let max_lag = signal.len() - template.len();

        let mut pool = CorrelationPool::new();
        let first: Vec<f64> = pool.ncc(&template, &signal, max_lag).to_vec();
        let second: Vec<f64> = pool.ncc(&template, &signal, max_lag).to_vec();
        assert_eq!(
            first,
            corr::ncc(&template, &signal, max_lag)
                .expect("correlation template/signal shapes must be valid")
        );
        assert_eq!(first, second);
        let stats = pool.stats();
        assert_eq!(stats.builds, 1);
        assert_eq!(stats.hits, 1);

        // 同一 shape の PHAT は別 mode の plan を構築する(NCC は
        // 有効ラグ域の短縮 FFT、PHAT は位相正規化が FFT 長依存のため
        // plan を共有できない)。
        let phat: Vec<f64> = pool.gcc_phat(&template, &signal, max_lag).to_vec();
        assert_eq!(
            phat,
            corr::gcc_phat(&template, &signal, max_lag)
                .expect("correlation template/signal shapes must be valid")
        );
        assert_eq!(pool.stats().builds, 2);
        assert_eq!(pool.entry_count(), 2);
    }

    #[test]
    fn pool_is_bounded_by_max_shapes() {
        let mut pool = CorrelationPool::new();
        // 全て異なる形状で MAX_SHAPES 超過を起こす。
        for i in 0..=MAX_SHAPES {
            let template = noise(i as u64, 64);
            let signal = noise(i as u64 + 100, 256 + i * 64);
            let _ = pool
                .ncc(&template, &signal, signal.len() - template.len())
                .to_vec();
        }
        assert!(pool.entry_count() <= MAX_SHAPES);
        assert!(pool.stats().builds > MAX_SHAPES as u64);
    }
}
