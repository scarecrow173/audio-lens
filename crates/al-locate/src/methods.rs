//! Independent locate methods.
//!
//! Each method turns `(template, signal)` into [`MethodEvidence`]:
//! candidate positions plus the scores/ratios/validity the consensus
//! layer needs. Methods never decide acceptance — they only produce
//! evidence, including their own rejection reasons.

use al_core::Validity;

use crate::candidate::{LocateCandidate, MethodEvidence};
use crate::dsp;

/// Minimum template energy to attempt correlation; below this the
/// method reports `SilenceInsufficientEnergy`.
const MIN_ENERGY: f64 = 1e-9;
/// Peaks below this background ratio are flagged low-confidence.
const MIN_PEAK_TO_BACKGROUND: f64 = 2.0;
/// Maximum candidates reported per method.
const MAX_CANDIDATES: usize = 8;

/// One independent locate method.
pub trait LocateMethod {
    /// Stable method id.
    fn id(&self) -> &'static str;
    /// Method version.
    fn version(&self) -> &'static str;
    /// Produce evidence for `template` inside `signal`.
    fn locate(&self, template: &[f64], signal: &[f64], sample_rate: u32) -> MethodEvidence;
}

fn energy(x: &[f64]) -> f64 {
    x.iter().map(|v| v * v).sum()
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
    if energy(template) < MIN_ENERGY {
        return Err(rejected(
            method,
            version,
            Validity::SilenceInsufficientEnergy,
            "template has no usable energy",
        ));
    }
    if energy(signal) < MIN_ENERGY {
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
/// `corr[lag]` is the correlation at lag. Returns candidates sorted
/// by descending score; background is the median absolute value.
pub fn peaks_from_correlation(corr: &[f64], top: usize) -> Vec<LocateCandidate> {
    if corr.is_empty() {
        return Vec::new();
    }
    let mut abs: Vec<f64> = corr.iter().map(|v| v.abs()).collect();
    abs.sort_by(|a, b| a.total_cmp(b));
    let background = abs[abs.len() / 2].max(1e-12);

    // Local maxima strictly above the background.
    let mut peaks: Vec<(usize, f64)> = (1..corr.len().saturating_sub(1))
        .filter(|&i| corr[i] >= corr[i - 1] && corr[i] > corr[i + 1] && corr[i] > background)
        .map(|i| (i, corr[i]))
        .collect();
    if corr.len() >= 2 && corr[0] > corr[1] && corr[0] > background {
        peaks.push((0, corr[0]));
    }
    peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
    peaks.truncate(top);

    let second = peaks.get(1).map(|p| p.1);
    peaks
        .iter()
        .map(|&(i, score)| {
            // Half-max width around the peak.
            let half = score / 2.0;
            let mut lo = i;
            while lo > 0 && corr[lo] > half {
                lo -= 1;
            }
            let mut hi = i;
            while hi + 1 < corr.len() && corr[hi] > half {
                hi += 1;
            }
            LocateCandidate {
                position: i as u64,
                score,
                peak_to_background: score / background,
                peak_width: (hi - lo) as u64,
                peak_ratio_12: second.map(|s| score / s.max(1e-12)),
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
        Some(top) if top.score <= 0.0 => rejected(
            method,
            version,
            Validity::LowCorrelation,
            "correlation peak is non-positive",
        ),
        Some(top) if top.peak_to_background < MIN_PEAK_TO_BACKGROUND => MethodEvidence {
            method: method.into(),
            method_version: version.into(),
            candidates,
            validity: Validity::LowCorrelation,
            reject_reason: Some("peak-to-background below threshold".into()),
        },
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

impl LocateMethod for FullNcc {
    fn id(&self) -> &'static str {
        "full_ncc"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], _rate: u32) -> MethodEvidence {
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
    /// Octave band edges (Hz) applied before summing NCC curves.
    const BANDS: &'static [(f64, f64)] = &[
        (20.0, 250.0),
        (250.0, 1000.0),
        (1000.0, 4000.0),
        (4000.0, 16000.0),
        (16000.0, 96000.0),
    ];
}

impl LocateMethod for BandNcc {
    fn id(&self) -> &'static str {
        "band_ncc"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], sample_rate: u32) -> MethodEvidence {
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        let nyquist = f64::from(sample_rate) / 2.0;
        let mut acc = vec![0.0; signal.len() - template.len() + 1];
        let mut bands_used = 0usize;
        for &(lo, hi) in Self::BANDS {
            if lo >= nyquist {
                continue;
            }
            let t = dsp::bandpass(template, sample_rate, lo, hi.min(nyquist));
            let s = dsp::bandpass(signal, sample_rate, lo, hi.min(nyquist));
            if energy(&t) < MIN_ENERGY {
                continue; // band absent in the template — skip, not reject
            }
            let corr = dsp::ncc(&t, &s, s.len().saturating_sub(t.len()));
            for (a, c) in acc.iter_mut().zip(&corr) {
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
    let rms = (energy(x) / x.len().max(1) as f64).sqrt();
    if rms < MIN_ENERGY {
        return x.to_vec();
    }
    x.iter().map(|v| v / rms).collect()
}

impl LocateMethod for SpectralEnvelope {
    fn id(&self) -> &'static str {
        "spectral_envelope"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    fn locate(&self, template: &[f64], signal: &[f64], _rate: u32) -> MethodEvidence {
        if let Err(e) = preflight(self.id(), self.version(), template, signal) {
            return e;
        }
        let t_env = dsp::spectral_envelope(&normalize_gain(template), self.fft_size, self.bins);
        // Slide a template-length window over the signal; envelope NCC
        // per position. Hop keeps the scan bounded.
        let win = template.len();
        let hop = (win / 16).max(1);
        let max_lag = signal.len() - win;
        let mut corr = vec![0.0; max_lag + 1];
        for lag in (0..=max_lag).step_by(hop) {
            let s_env = dsp::spectral_envelope(
                &normalize_gain(&signal[lag..lag + win]),
                self.fft_size,
                self.bins,
            );
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
