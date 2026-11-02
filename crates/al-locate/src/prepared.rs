//! ジョブ内で共有する不変 DSP 入力。窓のフィルタには流用しない。
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use ruviform_numeric::resample::ResamplePlan;

use crate::pool::run_pool;

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
        let filtered = ruviform_numeric::bands::bandpass_many(&samples, 8000, &bands)
            .expect("band settings and input must be valid");
        for ((_, actual), expected) in a.bands().iter().zip(filtered) {
            assert_eq!(actual.as_ref(), expected);
        }
    }

    #[test]
    fn prepare_many_matches_sequential_prepare_and_dedups_in_batch() {
        let a = noise(7, 4097);
        let b = noise(11, 4097);
        let specs = [
            PrepareSpec {
                id: "capture",
                channel: 0,
                rate: 48000,
                version: "original",
                samples: &a,
            },
            PrepareSpec {
                id: "capture",
                channel: 1,
                rate: 48000,
                version: "original",
                samples: &b,
            },
            PrepareSpec {
                id: "source",
                channel: 0,
                rate: 48000,
                version: "original",
                samples: &a,
            },
            // バッチ内重複: 先の miss 挿入を再利用する。
            PrepareSpec {
                id: "capture",
                channel: 0,
                rate: 48000,
                version: "original",
                samples: &a,
            },
        ];
        let mut cache = InputCache::default();
        let batch = cache.prepare_many(&specs).unwrap();
        assert!(Arc::ptr_eq(&batch[0], &batch[3]));

        let mut seq = InputCache::default();
        let expected: Vec<_> = specs
            .iter()
            .map(|s| {
                seq.prepare(s.id, s.channel, s.rate, s.version, s.samples)
                    .unwrap()
            })
            .collect();
        assert_eq!(cache.stats(), seq.stats());
        for (got, want) in batch.iter().zip(&expected) {
            assert_eq!(got.bands().len(), want.bands().len());
            for ((band_a, actual), (band_b, expected)) in got.bands().iter().zip(want.bands()) {
                assert_eq!(band_a, band_b);
                assert_eq!(actual.as_ref(), expected.as_ref());
            }
            assert_eq!(got.coarse_samples(), want.coarse_samples());
        }

        let again = cache.prepare_many(&specs[..3]).unwrap();
        assert!(Arc::ptr_eq(&again[0], &batch[0]));
        assert_eq!(cache.stats().misses, 3);
        assert_eq!(cache.stats().hits, 4);
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

    /// `cargo test --release -p al-locate bench_prepare_many -- --ignored --nocapture`
    /// で逐次 `prepare` と `prepare_many` の壁時計を比較する手動計測用。
    #[test]
    #[ignore]
    fn bench_prepare_many_parallel_vs_sequential() {
        use std::time::Instant;
        let n = 2_000_000usize;
        let data: Vec<Vec<f64>> = (0..6).map(|i| noise(i + 1, n)).collect();
        let specs: Vec<PrepareSpec<'_>> = data
            .iter()
            .enumerate()
            .map(|(channel, samples)| PrepareSpec {
                id: "capture",
                channel,
                rate: 48000,
                version: "original",
                samples,
            })
            .collect();
        let t = Instant::now();
        let mut seq = InputCache::default();
        for s in &specs {
            seq.prepare(s.id, s.channel, s.rate, s.version, s.samples)
                .unwrap();
        }
        let seq_ms = t.elapsed().as_secs_f64() * 1e3;

        let t = Instant::now();
        let mut par = InputCache::default();
        let out = par.prepare_many(&specs).unwrap();
        let par_ms = t.elapsed().as_secs_f64() * 1e3;

        assert_eq!(out.len(), specs.len());
        assert_eq!(seq.stats(), par.stats());
        eprintln!(
            "specs={} sequential={seq_ms:.0}ms prepare_many={par_ms:.0}ms speedup={:.2}x",
            specs.len(),
            seq_ms / par_ms
        );
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

/// `prepare_many` の 1 件分の指定。帯域は `bands_for_rate(rate)` で
/// 導き、`prepare` と同じキー・計算規則を共有する。
pub(crate) struct PrepareSpec<'a> {
    pub id: &'a str,
    pub channel: usize,
    pub rate: u32,
    pub version: &'a str,
    pub samples: &'a [f64],
}

/// 半開帯域 (Hz) と、その全レート信号。
pub type BandSignal = ((f64, f64), Arc<[f64]>);

/// `prepare`/`prepare_with_bands` 共通のキャッシュキー。
fn input_key(
    id: &str,
    channel: usize,
    rate: u32,
    version: &str,
    samples: &[f64],
    bands: &[(f64, f64)],
) -> InputKey {
    InputKey {
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
    }
}

/// 1 チャンネルのフィルタ・間引き計算。純粋関数であり、呼出順序や
/// ワーカースレッドに依存しない。
fn filter_channel(
    samples: &[f64],
    rate: u32,
    bands: &[(f64, f64)],
) -> Result<PreparedChannel, &'static str> {
    let filtered = ruviform_numeric::bands::bandpass_many(samples, rate, bands)
        .expect("band settings and input must be valid");
    let coarse_rate = rate / 4;
    let (coarse, radius) = if coarse_rate > 0 && samples.len() >= 4 {
        let plan =
            ResamplePlan::new(rate, coarse_rate).map_err(|_| "sample rates must be nonzero")?;
        (
            Some(
                plan.resample(samples)
                    .map_err(|_| "resample input length mismatch")?,
            ),
            2 * plan.radius() + (rate as usize).div_ceil(coarse_rate as usize),
        )
    } else {
        (None, 0)
    };
    Ok(PreparedChannel {
        bands: bands
            .iter()
            .copied()
            .zip(filtered.into_iter().map(Arc::from))
            .collect(),
        coarse,
        coarse_rate,
        radius,
    })
}

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
        let key = input_key(id, channel, rate, version, samples, bands);
        if let Some(prepared) = self.channels.get(&key) {
            self.hits += 1;
            return Ok(Arc::clone(prepared));
        }
        self.misses += 1;
        let prepared = Arc::new(filter_channel(samples, rate, bands)?);
        self.channels.insert(key, Arc::clone(&prepared));
        Ok(prepared)
    }

    /// `prepare` と同じ帯域・計算規則で複数 (id, channel) を一括準備する。
    /// フィルタ・間引きは純粋な計算のためワーカープールで並列化し、
    /// キャッシュへの登録と hit/miss の計上は spec 順に逐次 `prepare`
    /// した場合と同じ規則で行う。同一キーの重複 spec は、先の計算
    /// 結果を挿入したエントリを再利用する。
    pub(crate) fn prepare_many(
        &mut self,
        specs: &[PrepareSpec<'_>],
    ) -> Result<Vec<Arc<PreparedChannel>>, &'static str> {
        let _span = tracing::info_span!("locate::prepare_channels", specs = specs.len()).entered();
        // キーを確定し、計算が必要な miss を集める。重複キーは pending
        // に残り、先の miss の挿入結果を共有する。
        let mut keys = Vec::with_capacity(specs.len());
        let mut spec_bands = Vec::with_capacity(specs.len());
        let mut miss_specs: Vec<usize> = Vec::new();
        let mut pending: HashSet<InputKey> = HashSet::new();
        for spec in specs {
            if spec.rate == 0 {
                return Err("sample rates must be nonzero");
            }
            let bands = bands_for_rate(spec.rate);
            let key = input_key(
                spec.id,
                spec.channel,
                spec.rate,
                spec.version,
                spec.samples,
                &bands,
            );
            if !self.channels.contains_key(&key) && pending.insert(key.clone()) {
                miss_specs.push(keys.len());
            }
            keys.push(key);
            spec_bands.push(bands);
        }
        let computed = run_pool(miss_specs.len(), &|| (), &|j, _| {
            let i = miss_specs[j];
            filter_channel(specs[i].samples, specs[i].rate, &spec_bands[i])
        });
        // spec 順に逐次呼出と同じ規則で処理する。既存エントリは hit、
        // miss は spec 順に計算結果を消費して挿入する。
        let mut computed = computed.into_iter();
        let mut out = Vec::with_capacity(specs.len());
        for key in &keys {
            if let Some(prepared) = self.channels.get(key) {
                self.hits += 1;
                out.push(Arc::clone(prepared));
                continue;
            }
            self.misses += 1;
            let prepared = Arc::new(computed.next().expect("one compute per miss")?);
            self.channels.insert(key.clone(), Arc::clone(&prepared));
            out.push(prepared);
        }
        Ok(out)
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
