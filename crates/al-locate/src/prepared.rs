//! ジョブ内で共有する不変 DSP 入力。窓のフィルタには流用しない。
use std::collections::HashMap;
use std::sync::Arc;

use al_math::resample::{ResamplePlan, resample};

/// 現行 BandNcc の帯域。Nyquist で切り詰める。
const BANDS: &[(f64, f64)] = &[
    (20.0, 250.0),
    (250.0, 1000.0),
    (1000.0, 4000.0),
    (4000.0, 16000.0),
    (16000.0, 96000.0),
];

pub(crate) fn bands_for_rate(rate: u32) -> Vec<(f64, f64)> {
    let nyquist = f64::from(rate) / 2.0;
    BANDS
        .iter()
        .copied()
        .filter(|&(lo, _)| lo < nyquist)
        .map(|(lo, hi)| (lo, hi.min(nyquist)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn custom_band_cache_preserves_identity_and_separates_filter_settings() {
        let samples: Vec<_> = (0..512).map(|i| (i as f64 * 0.2).sin()).collect();
        let mut cache = InputCache::default();
        let bands = [(100.0, 500.0), (500.0, 1000.0)];
        let a = cache
            .prepare_with_bands("input", 0, 8000, "v1", &samples, &bands)
            .unwrap();
        let b = cache
            .prepare_with_bands("input", 0, 8000, "v1", &samples, &bands)
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let other = cache
            .prepare_with_bands("input", 0, 8000, "v1", &samples, &[(100.0, 1000.0)])
            .unwrap();
        assert!(!Arc::ptr_eq(&a, &other));
        let filtered = crate::dsp::bandpass_many(&samples, 8000, &bands);
        for ((_, actual), expected) in a.bands().iter().zip(filtered) {
            assert_eq!(actual.as_ref(), expected);
        }
    }

    #[test]
    fn hit_and_miss_counts_distinguish_reuse_from_recompute() {
        let samples = noise(3, 1025);
        let mut cache = InputCache::default();
        cache
            .prepare("capture", 0, 48000, "original", &samples)
            .unwrap();
        cache
            .prepare("capture", 0, 48000, "original", &samples)
            .unwrap();
        cache
            .prepare("source", 0, 48000, "original", &samples)
            .unwrap();
        let stats = cache.stats();
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.hits, 1);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn release_drops_only_the_matching_input_and_version() {
        let samples = noise(5, 1025);
        let mut cache = InputCache::default();
        cache
            .prepare("capture", 0, 48000, "original", &samples)
            .unwrap();
        cache
            .prepare("capture", 1, 48000, "original", &samples)
            .unwrap();
        cache
            .prepare("capture", 0, 48000, "residual:x", &samples)
            .unwrap();
        cache
            .prepare("other", 0, 48000, "original", &samples)
            .unwrap();
        assert_eq!(cache.len(), 4);
        cache.release("capture", "original");
        assert_eq!(cache.len(), 2);
        // release 後の再 prepare は miss(フィルタ再計算)として計上される。
        cache
            .prepare("capture", 0, 48000, "original", &samples)
            .unwrap();
        assert_eq!(cache.stats().misses, 5);
    }
}

/// F5: 同じ全区間、次の 2 冪へ末尾ゼロ埋め、半開帯域、元長へ切り戻す。
/// 端点の周期拡張や窓を追加しない。規則の変更時は別設定となる。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FilterKey {
    frames: usize,
    fft_len: usize,
    bands: Vec<(u64, u64)>,
    algorithm: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InputKey {
    id: String,
    channel: usize,
    rate: u32,
    version: String,
    filter: FilterKey,
}

/// 半開帯域 (Hz) と、その全レート信号。
pub type BandSignal = ((f64, f64), Arc<[f64]>);

/// 入力 1 チャンネルのフィルタ済み全区間とアンチエイリアス付き間引き信号。
/// 原信号は呼び出し元の正規化 PCM を参照し、複製しない。
pub struct PreparedChannel {
    bands: Vec<BandSignal>,
    coarse: Option<Vec<f64>>,
    pub(crate) coarse_rate: u32,
    pub(crate) radius: usize,
}

impl PreparedChannel {
    /// 全レート帯域信号。帯域と FFT grid は従来の全区間フィルタと同じ。
    pub fn bands(&self) -> &[BandSignal] {
        &self.bands
    }
    /// ほぼ 1/4 レートの候補提案用信号。
    pub fn coarse_samples(&self) -> Option<&[f64]> {
        self.coarse.as_deref()
    }
}

/// `InputCache` の計測値。`hits` は既存エントリの再利用、
/// `misses` は実際にフィルタ・間引きを計算した回数。
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

/// 入力準備層が所有し、全ペア・全メソッドの完了後に解放する。
#[derive(Default)]
pub struct InputCache {
    channels: HashMap<InputKey, Arc<PreparedChannel>>,
    hits: u64,
    misses: u64,
}

impl InputCache {
    /// この入力の処理完了時に、キャッシュの所有参照を解放する。
    pub(crate) fn release(&mut self, id: &str, version: &str) {
        self.channels
            .retain(|key, _| key.id != id || key.version != version);
    }

    /// `version` は不変信号を識別する。残差には親 alignment ごとの版を使う。
    /// 同じ識別子・版・区間で異なるサンプルを渡してはならない。
    pub fn prepare(
        &mut self,
        id: &str,
        channel: usize,
        rate: u32,
        version: &str,
        samples: &[f64],
    ) -> Result<Arc<PreparedChannel>, &'static str> {
        if rate == 0 {
            return Err("sample rates must be nonzero");
        }
        let bands = bands_for_rate(rate);
        self.prepare_with_bands(id, channel, rate, version, samples, &bands)
    }

    /// 指定帯域を同じ入力単位キャッシュで準備する。窓ごとの再フィルタを避ける。
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_with_bands(
        &mut self,
        id: &str,
        channel: usize,
        rate: u32,
        version: &str,
        samples: &[f64],
        bands: &[(f64, f64)],
    ) -> Result<Arc<PreparedChannel>, &'static str> {
        if rate == 0 {
            return Err("sample rates must be nonzero");
        }
        let key = InputKey {
            id: id.into(),
            channel,
            rate,
            version: version.into(),
            filter: FilterKey {
                frames: samples.len(),
                fft_len: samples.len().max(1).next_power_of_two(),
                bands: bands
                    .iter()
                    .map(|(lo, hi)| (lo.to_bits(), hi.to_bits()))
                    .collect(),
                algorithm: "spectral-mask-half-open-zero-pad-truncate-v1",
            },
        };
        if let Some(prepared) = self.channels.get(&key) {
            self.hits += 1;
            return Ok(Arc::clone(prepared));
        }
        self.misses += 1;
        let filtered = crate::dsp::bandpass_many(samples, rate, bands);
        let coarse_rate = rate / 4;
        let (coarse, radius) = if coarse_rate > 0 && samples.len() >= 4 {
            let plan = ResamplePlan::new(rate, coarse_rate)?;
            (
                Some(resample(samples, &plan)?),
                2 * plan.radius() + (rate as usize).div_ceil(coarse_rate as usize),
            )
        } else {
            (None, 0)
        };
        let prepared = Arc::new(PreparedChannel {
            bands: bands
                .iter()
                .copied()
                .zip(filtered.into_iter().map(Arc::from))
                .collect(),
            coarse,
            coarse_rate,
            radius,
        });
        self.channels.insert(key, Arc::clone(&prepared));
        Ok(prepared)
    }

    /// hit/miss の計測値。診断用で永続成果物には出力しない。
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
        }
    }

    /// 現在キャッシュに生存している入力×フィルタ形状の数。
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    /// キャッシュが空かどうか。
    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}
