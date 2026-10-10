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

/// 入力準備層が所有し、全ペア・全メソッドの完了後に解放する。
#[derive(Default)]
pub struct InputCache {
    channels: HashMap<InputKey, Arc<PreparedChannel>>,
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
            return Ok(Arc::clone(prepared));
        }
        let filtered = crate::dsp::bandpass_many(samples, rate, &bands);
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
                .into_iter()
                .zip(filtered.into_iter().map(Arc::from))
                .collect(),
            coarse,
            coarse_rate,
            radius,
        });
        self.channels.insert(key, Arc::clone(&prepared));
        Ok(prepared)
    }
}
