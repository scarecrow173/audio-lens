//! Independent locate methods.
//!
//! Each method turns `(template, signal)` into [`MethodEvidence`]:
//! candidate positions plus the scores/ratios/validity the consensus
//! layer needs. Methods never decide acceptance — they only produce
//! evidence, including their own rejection reasons.

use al_core::Validity;

use crate::candidate::{LocateCandidate, MethodEvidence};
use crate::dsp;
use crate::prepared::{PreparedChannel, bands_for_rate};

/// Minimum template energy to attempt correlation; below this the
/// method reports `SilenceInsufficientEnergy`.
const MIN_ENERGY: f64 = 1e-9;
/// Peaks below this background ratio are flagged low-confidence.
const MIN_PEAK_TO_BACKGROUND: f64 = 2.0;
/// Maximum candidates reported per method.
const MAX_CANDIDATES: usize = 8;

/// 粗候補と原レート近傍精密化の実行記録。合議用の証拠ではない。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoarseSearch {
    /// 元レートと粗探索レート。比は必ずしも整数にならない。
    pub original_rate: u32,
    pub coarse_rate: u32,
    /// 元レートでの閉区間 lag 範囲。
    pub search_range: [u64; 2],
    /// 原レートでの左右の近傍幅。
    pub neighborhood_radius: usize,
    /// 原レート座標へ丸めた粗候補。周期信号の複数候補を保持する。
    pub coarse_positions: Vec<u64>,
    /// 原レートの近傍で精密化した候補。順位・統計を最終証拠に流用しない。
    pub refined_positions: Vec<u64>,
    /// F3 の全探索域証拠契約により常に true。
    pub full_search_required: bool,
    pub fallback_reason: &'static str,
}

/// 長尺素材 (2 秒以上) の候補提案。NCC の粗候補を原レートで再探索する。
/// 近傍だけでは全域の背景・競合・幅を保証できず、全手法は別途全探索する。
pub fn propose_coarse_to_fine(
    template: &[f64],
    signal: &[f64],
    rate: u32,
    prepared_template: &PreparedChannel,
    prepared_signal: &PreparedChannel,
) -> Option<CoarseSearch> {
    let mut search = propose_coarse(template, signal, rate, prepared_template, prepared_signal)?;
    let corr = dsp::ncc(template, signal, signal.len() - template.len());
    refine_coarse(&mut search, &corr);
    Some(search)
}

fn propose_coarse(
    template: &[f64],
    signal: &[f64],
    rate: u32,
    prepared_template: &PreparedChannel,
    prepared_signal: &PreparedChannel,
) -> Option<CoarseSearch> {
    if template.is_empty() || template.len() < rate as usize * 2 || signal.len() < template.len() {
        return None;
    }
    let t = prepared_template.coarse_samples()?;
    let s = prepared_signal.coarse_samples()?;
    let coarse_rate = prepared_template.coarse_rate;
    if s.len() < t.len() {
        return None;
    }
    let max_lag = signal.len() - template.len();
    let radius = prepared_signal.radius;
    let coarse = peaks_from_correlation(&dsp::ncc(t, s, s.len() - t.len()), MAX_CANDIDATES);
    let positions: Vec<_> = coarse
        .iter()
        .map(|c| {
            ((c.position as u128 * rate as u128 + coarse_rate as u128 / 2) / coarse_rate as u128)
                .min(max_lag as u128) as u64
        })
        .collect();
    let reason = if positions.is_empty() {
        "no-coarse-candidates"
    } else if positions
        .iter()
        .any(|&p| p <= radius as u64 || max_lag as u64 - p <= radius as u64)
    {
        "boundary-candidate"
    } else {
        "global-evidence-required"
    };
    Some(CoarseSearch {
        original_rate: rate,
        coarse_rate,
        search_range: [0, max_lag as u64],
        neighborhood_radius: radius,
        coarse_positions: positions,
        refined_positions: Vec::new(),
        full_search_required: true,
        fallback_reason: reason,
    })
}

fn refine_coarse(search: &mut CoarseSearch, corr: &[f64]) {
    // 全域証拠に必要な原レート NCC を近傍精密化にも共用し、長尺 FFT の重複を避ける。
    for &position in &search.coarse_positions {
        let lo = (position as usize).saturating_sub(search.neighborhood_radius);
        let hi = (position as usize + search.neighborhood_radius).min(corr.len() - 1);
        if let Some(candidate) = peaks_from_correlation(&corr[lo..=hi], 1).first() {
            search
                .refined_positions
                .push(lo as u64 + candidate.position);
        }
    }
    if !search.coarse_positions.is_empty() && search.refined_positions.is_empty() {
        search.fallback_reason = "no-refined-candidates";
    }
}

/// One independent locate method.
pub trait LocateMethod {
    /// Stable method id.
    fn id(&self) -> &'static str;
    /// Method version.
    fn version(&self) -> &'static str;
    /// Produce evidence for `template` inside `signal`.
    fn locate(&self, template: &[f64], signal: &[f64], sample_rate: u32) -> MethodEvidence;

    /// 入力準備層が共有するフィルタを利用する。証拠の探索域は変更しない。
    fn locate_prepared(
        &self,
        template: &[f64],
        signal: &[f64],
        sample_rate: u32,
        _prepared_template: &PreparedChannel,
        _prepared_signal: &PreparedChannel,
    ) -> MethodEvidence {
        self.locate(template, signal, sample_rate)
    }
}

fn rejected(
    method: &'static str,
    version: &'static str,
    validity: Validity,
    reason: &str,
) -> MethodEvidence {
    MethodEvidence {
        method: method.into(),
        method_version: version.into(),
        candidates: vec![],
        validity,
        reject_reason: Some(reason.into()),
    }
}

fn preflight(
    method: &'static str,
    version: &'static str,
    template: &[f64],
    signal: &[f64],
) -> Result<(), MethodEvidence> {
    if template.is_empty() || signal.len() < template.len() {
        return Err(rejected(
            method,
            version,
            Validity::InsufficientSupport,
            "signal shorter than template",
        ));
    }
    if dsp::sum_sq(template) < MIN_ENERGY {
        return Err(rejected(
            method,
            version,
            Validity::SilenceInsufficientEnergy,
            "template has no usable energy",
        ));
    }
    if dsp::sum_sq(signal) < MIN_ENERGY {
        return Err(rejected(
            method,
            version,
            Validity::SilenceInsufficientEnergy,
            "signal has no usable energy",
        ));
    }
    Ok(())
}

/// Extract ranked peaks from a correlation curve.
///
/// `corr[lag]` is the correlation at lag. Peaks are local maxima of
/// `|corr|` — including both endpoints and the single-element case —
/// so a source embedded flush at the capture tail is still found, and
/// a polarity-inverted source produces a negative-score candidate
/// rather than nothing. Candidates are sorted by descending `|score|`;
/// background is the median absolute value.
pub fn peaks_from_correlation(corr: &[f64], top: usize) -> Vec<LocateCandidate> {
    if corr.is_empty() {
        return Vec::new();
    }
    let mut abs: Vec<f64> = corr.iter().map(|v| v.abs()).collect();
    abs.sort_by(|a, b| a.total_cmp(b));
    let background = abs[abs.len() / 2].max(1e-12);

    let n = corr.len();
    let is_peak = |i: usize| {
        let v = corr[i].abs();
        v > background
            && (i == 0 || corr[i - 1].abs() <= v)
            && (i + 1 == n || corr[i + 1].abs() < v)
    };
    let mut peaks: Vec<(usize, f64)> = if n == 1 {
        vec![(0, corr[0])]
    } else {
        (0..n)
            .filter(|&i| is_peak(i))
            .map(|i| (i, corr[i]))
            .collect()
    };
    peaks.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()));
    peaks.truncate(top);

    let second = peaks.get(1).map(|p| p.1.abs());
    peaks
        .iter()
        .enumerate()
        .map(|(rank, &(i, score))| {
            // Half-max width around the peak in |corr|.
            let half = score.abs() / 2.0;
            let mut lo = i;
            while lo > 0 && corr[lo].abs() > half {
                lo -= 1;
            }
            let mut hi = i;
            while hi + 1 < n && corr[hi].abs() > half {
                hi += 1;
            }
            LocateCandidate {
                position: i as u64,
                score,
                peak_to_background: score.abs() / background,
                peak_width: (hi - lo) as u64,
                peak_ratio_12: if rank == 0 {
                    second.map(|s| score.abs() / s.max(1e-12))
                } else {
                    None
                },
                channel_pair: None,
            }
        })
        .collect()
}

fn evidence_from_corr(method: &'static str, version: &'static str, corr: &[f64]) -> MethodEvidence {
    let candidates = peaks_from_correlation(corr, MAX_CANDIDATES);
    match candidates.first() {
        None => rejected(
            method,
            version,
            Validity::InsufficientSupport,
            "no correlation peak above background",
        ),
        Some(top) if !top.score.is_finite() || top.score.abs() <= 1e-12 => rejected(
            method,
            version,
            Validity::LowCorrelation,
            "correlation peak is non-positive",
        ),
        // A single-lag correlation has no background to compare
        // against; |score| alone carries the evidence.
        Some(top) if corr.len() > 1 && top.peak_to_background < MIN_PEAK_TO_BACKGROUND => {
            MethodEvidence {
                method: method.into(),
                method_version: version.into(),
                candidates,
                validity: Validity::LowCorrelation,
                reject_reason: Some("peak-to-background below threshold".into()),
            }
        }
        Some(_) => MethodEvidence {
            method: method.into(),
            method_version: version.into(),
            candidates,
            validity: Validity::Valid,
            reject_reason: None,
        },
    }
}

/// Full-band normalized cross-correlation.
pub struct FullNcc;

impl FullNcc {
    /// 粗候補を提案し、必須の全探索曲線を精密化と最終証拠で一度だけ共用する。
    pub fn locate_coarse_to_fine(
        &self,
        template: &[f64],
        signal: &[f64],
        rate: u32,
        prepared_template: &PreparedChannel,
        prepared_signal: &PreparedChannel,
    ) -> (MethodEvidence, Option<CoarseSearch>) {
        let span = tracing::info_span!(
            "locate::full_ncc",
            method = self.id(),
            len_a = template.len(),
            len_b = signal.len()
        );
        let _enter = span.enter();
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return (e, None);
        }
        let mut search = propose_coarse(template, signal, rate, prepared_template, prepared_signal);
        let corr = dsp::ncc(template, signal, signal.len() - template.len());
        if let Some(search) = &mut search {
            refine_coarse(search, &corr);
        }
        (evidence_from_corr(self.id(), self.version(), &corr), search)
    }
}

impl LocateMethod for FullNcc {
    fn id(&self) -> &'static str {
        "full_ncc"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], _rate: u32) -> MethodEvidence {
        let span = tracing::info_span!(
            "locate::full_ncc",
            method = self.id(),
            len_a = template.len(),
            len_b = signal.len()
        );
        let _enter = span.enter();
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        let corr = dsp::ncc(template, signal, signal.len() - template.len());
        evidence_from_corr(self.id(), self.version(), &corr)
    }
}

/// Band-split NCC: correlation evidence accumulated over octave
/// bands, which resists full-band fades.
pub struct BandNcc;

impl BandNcc {
    fn evidence_from_bands<'a>(
        &self,
        curve_len: usize,
        bands: impl Iterator<Item = (&'a [f64], &'a [f64])>,
    ) -> MethodEvidence {
        let mut acc = vec![0.0; curve_len];
        let mut bands_used = 0;
        for (t, s) in bands {
            if dsp::sum_sq(t) < MIN_ENERGY {
                continue;
            }
            for (a, c) in acc.iter_mut().zip(dsp::ncc(t, s, s.len() - t.len())) {
                *a += c;
            }
            bands_used += 1;
        }
        if bands_used == 0 {
            return rejected(
                self.id(),
                self.version(),
                Validity::OutOfSupportedFrequencyRange,
                "template has no energy in any usable band",
            );
        }
        evidence_from_corr(self.id(), self.version(), &acc)
    }
}

impl LocateMethod for BandNcc {
    fn id(&self) -> &'static str {
        "band_ncc"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], sample_rate: u32) -> MethodEvidence {
        let span = tracing::info_span!(
            "locate::band_ncc",
            method = self.id(),
            len_a = template.len(),
            len_b = signal.len()
        );
        let _enter = span.enter();
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        let bands = bands_for_rate(sample_rate);
        let templates = dsp::bandpass_many(template, sample_rate, &bands);
        // 素材に存在する帯域だけキャプチャを処理する。区間と FFT grid は従来と同じ。
        let active: Vec<_> = bands
            .iter()
            .copied()
            .zip(templates)
            .filter(|(_, t)| dsp::sum_sq(t) >= MIN_ENERGY)
            .collect();
        let active_bands: Vec<_> = active.iter().map(|(band, _)| *band).collect();
        let signals = dsp::bandpass_many(signal, sample_rate, &active_bands);
        self.evidence_from_bands(
            signal.len() - template.len() + 1,
            active
                .iter()
                .zip(&signals)
                .map(|((_, t), s)| (t.as_slice(), s.as_slice())),
        )
    }

    fn locate_prepared(
        &self,
        template: &[f64],
        signal: &[f64],
        _sample_rate: u32,
        prepared_template: &PreparedChannel,
        prepared_signal: &PreparedChannel,
    ) -> MethodEvidence {
        let span = tracing::info_span!(
            "locate::band_ncc",
            method = self.id(),
            len_a = template.len(),
            len_b = signal.len()
        );
        let _enter = span.enter();
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        self.evidence_from_bands(
            signal.len() - template.len() + 1,
            prepared_template
                .bands()
                .iter()
                .zip(prepared_signal.bands())
                .map(|((band, t), (signal_band, s))| {
                    assert_eq!(band, signal_band);
                    (t.as_ref(), s.as_ref())
                }),
        )
    }
}

/// GCC-PHAT generalized cross-correlation (phase-whitened).
pub struct GccPhat;

impl LocateMethod for GccPhat {
    fn id(&self) -> &'static str {
        "gcc_phat"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], _rate: u32) -> MethodEvidence {
        let span = tracing::info_span!(
            "locate::gcc_phat",
            method = self.id(),
            len_a = template.len(),
            len_b = signal.len()
        );
        let _enter = span.enter();
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        let corr = dsp::gcc_phat(template, signal, signal.len() - template.len());
        evidence_from_corr(self.id(), self.version(), &corr)
    }
}

/// Spectral-envelope matching: NCC over log-magnitude envelopes of
/// sliding windows. Absolute-level sensitive — inputs are normalized
/// to unit RMS by the caller-side [`normalize_gain`] helper.
pub struct SpectralEnvelope {
    /// FFT size for envelope extraction (power of two).
    pub fft_size: usize,
    /// Number of envelope bins.
    pub bins: usize,
}

impl Default for SpectralEnvelope {
    fn default() -> Self {
        Self {
            fft_size: 1024,
            bins: 32,
        }
    }
}

/// Normalize to unit RMS so absolute-scale comparisons are
/// meaningful across different `sample_format` inputs.
pub fn normalize_gain(x: &[f64]) -> Vec<f64> {
    let r = dsp::rms(x);
    if r < MIN_ENERGY {
        return x.to_vec();
    }
    let mut out = vec![0.0; x.len()];
    dsp::scale(x, r.recip(), &mut out);
    out
}

impl LocateMethod for SpectralEnvelope {
    fn id(&self) -> &'static str {
        "spectral_envelope"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], _rate: u32) -> MethodEvidence {
        let span = tracing::info_span!(
            "locate::spectral_envelope",
            method = self.id(),
            len_a = template.len(),
            len_b = signal.len()
        );
        let _enter = span.enter();
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        let t_env = dsp::spectral_envelope(&normalize_gain(template), self.fft_size, self.bins);
        // Slide a template-length window over the signal; envelope NCC
        // per position. Hop keeps the scan bounded even for very short
        // templates, where win/16 alone would explode the lag count.
        const MAX_WINDOWS: usize = 2048;
        let win = template.len();
        let max_lag = signal.len() - win;
        let hop = (win / 16).max(1).max(max_lag / MAX_WINDOWS);
        let mut corr = vec![0.0; max_lag + 1];
        for lag in (0..=max_lag).step_by(hop) {
            let s_env = dsp::spectral_envelope(
                &normalize_gain(&signal[lag..lag + win]),
                self.fft_size,
                self.bins,
            );
            // 残存許容(TASK-034EpPesSpbjbAQBsmjvJx): `dsp::corr0` 化すると
            // 周期入力で各 lag が厳密に同点(≈1.0)になり、同点 argmax が
            // 先頭 lag に移って合議の曖昧性棄却を固定する golden が破壊
            // される(偽受理の再現)。順序が結合順に本質依存するため、
            // 契約保持を優先して逐次式を維持する。
            let dot: f64 = t_env.iter().zip(&s_env).map(|(a, b)| a * b).sum();
            let tn: f64 = t_env.iter().map(|v| v * v).sum::<f64>().sqrt();
            let sn: f64 = s_env.iter().map(|v| v * v).sum::<f64>().sqrt();
            corr[lag] = if tn * sn > 0.0 { dot / (tn * sn) } else { 0.0 };
        }
        evidence_from_corr(self.id(), self.version(), &corr)
    }
}

/// All default methods in declaration order.
pub fn default_methods() -> Vec<Box<dyn LocateMethod>> {
    vec![
        Box::new(FullNcc),
        Box::new(BandNcc),
        Box::new(GccPhat),
        Box::new(SpectralEnvelope::default()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noise_signal(seed: u64, n: usize) -> Vec<f64> {
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
    fn full_ncc_finds_embedded_source() {
        let signal = noise_signal(7, 4096);
        let template = signal[1500..2000].to_vec();
        let ev = FullNcc.locate(&template, &signal, 48000);
        assert_eq!(ev.validity, Validity::Valid);
        assert_eq!(ev.candidates[0].position, 1500);
    }

    #[test]
    fn correlation_methods_rank_inverted_source_by_magnitude_and_keep_signed_score() {
        let signal = noise_signal(7, 4097);
        let template: Vec<_> = signal[1500..2001].iter().map(|v| -v).collect();
        let methods: Vec<Box<dyn LocateMethod>> =
            vec![Box::new(FullNcc), Box::new(BandNcc), Box::new(GccPhat)];
        for method in methods {
            let evidence = method.locate(&template, &signal, 48000);
            assert_eq!(evidence.validity, Validity::Valid, "{}", method.id());
            assert_eq!(evidence.candidates[0].position, 1500, "{}", method.id());
            assert!(evidence.candidates[0].score < 0.0, "{}", method.id());
        }
    }

    #[test]
    fn methods_reject_silence_and_short_inputs() {
        let signal = noise_signal(3, 2048);
        let silence = vec![0.0; 512];
        for m in default_methods() {
            let ev = m.locate(&silence, &signal, 48000);
            assert_eq!(
                ev.validity,
                Validity::SilenceInsufficientEnergy,
                "{}",
                ev.method
            );
            assert!(ev.reject_reason.is_some());
        }
        let short = noise_signal(5, 64);
        let ev = FullNcc.locate(&noise_signal(9, 512), &short, 48000);
        assert_eq!(ev.validity, Validity::InsufficientSupport);
    }

    #[test]
    fn single_lag_correlation_yields_a_candidate() {
        // Signal identical in length to the template → correlation of
        // length 1; the sole lag is still a valid peak.
        let template = noise_signal(13, 512);
        let ev = FullNcc.locate(&template, &template, 48000);
        assert_eq!(ev.validity, Validity::Valid);
        assert_eq!(ev.candidates.len(), 1);
        assert_eq!(ev.candidates[0].position, 0);
        assert!((ev.candidates[0].score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn peaks_from_correlation_checks_endpoints_and_magnitude() {
        // Peak at the last index, and a polarity-inverted peak.
        let mut corr = vec![0.1; 8];
        corr[7] = 0.95;
        assert_eq!(peaks_from_correlation(&corr, 4)[0].position, 7);
        corr[7] = -0.95;
        assert_eq!(peaks_from_correlation(&corr, 4)[0].score, -0.95);
        assert_eq!(peaks_from_correlation(&[0.9], 4).len(), 1);
        assert!(peaks_from_correlation(&[], 4).is_empty());
    }

    #[test]
    fn spectral_envelope_locates_offset() {
        // Envelope method is approximate; just check it produces
        // usable evidence on a loud embedded template.
        let mut signal = vec![0.0; 8192];
        let template = noise_signal(11, 2048);
        for (i, &v) in template.iter().enumerate() {
            signal[4000 + i] = v * 3.0;
        }
        let ev = SpectralEnvelope::default().locate(&template, &signal, 48000);
        assert!(ev.validity == Validity::Valid || !ev.candidates.is_empty());
    }
}
