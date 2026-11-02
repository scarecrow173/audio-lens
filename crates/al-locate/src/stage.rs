//! `LocateStage`: pairs `source_asset` inputs with `capture_*`
//! inputs, runs the four locate methods per channel, evaluates
//! consensus, fits the mix, identifies occurrences, builds the
//! piecewise time map, runs the two-stage residual search, and
//! persists every result — accepted and rejected alike.
//!
//! 素材はキャプチャのレートに正規化し、比較した PCM を永続化する。
//! A pair whose decoded PCM is
//! missing or empty is marked per-pair — one bad input never aborts
//! the stage's other pairs.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use al_core::{AlignmentId, Confidence, InputRole, StageKind, Validity};
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use crate::candidate::{LocateCandidate, MethodEvidence};
use crate::consensus::{ConsensusConfig, evaluate};
use crate::decode::{DecodedInput, decode_input};
use ruviform_numeric::resample::ResamplePlan;
use ruviform_numeric::{spec, vec};

use crate::methods::{
    CoarseSearch, FullNcc, TIE_ABS_FLOOR, TIE_REL_EPS, argmax_tie_count, default_methods,
    peaks_from_correlation,
};
use crate::mixer::fit_mix;
use crate::normalize::prepare_input;
use crate::pool::{CorrelationPool, run_pool};
use crate::prepared::{InputCache, PrepareSpec, PreparedChannel};
use crate::residual::{assemble_residual_search, persist_residual, prepare_residual};
use crate::result::{LocateResult, SampleRange};
use crate::tracking::{Anchor, build_time_map, identify_loop_occurrences};

/// Locate stage configuration (overlaid on defaults).
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LocateConfig {
    /// Consensus thresholds.
    #[serde(default)]
    pub consensus: ConsensusConfig,
    /// Ridge parameter for mixer fitting.
    #[serde(default = "default_ridge")]
    pub ridge: f64,
    /// Allowed deviation of a segment's dcapture/dsource rate from 1.0
    /// before it stops being `Continuous`.
    #[serde(default = "default_rate_tolerance")]
    pub rate_tolerance: f64,
    /// Anchors an off-rate run must span to count as `Drift`; shorter
    /// off-rate runs are `Jitter`.
    #[serde(default = "default_min_run")]
    pub min_run: usize,
    /// Source window length for per-window anchor extraction
    /// (samples). A source that fits at least two windows gets one
    /// anchor per window, so drift, jitter, skips, gaps and resumes
    /// inside a single occurrence stay observable; shorter sources
    /// keep one anchor per occurrence.
    #[serde(default = "default_anchor_window")]
    pub anchor_window: u64,
    /// Hop between successive anchor windows (samples).
    #[serde(default = "default_anchor_hop")]
    pub anchor_hop: u64,
    /// Slack between the source and capture advances of consecutive
    /// anchors before their boundary becomes `Gap`/`Skip` (samples).
    #[serde(default = "default_anchor_gap")]
    pub anchor_gap: u64,
}

fn default_ridge() -> f64 {
    1e-3
}

fn default_rate_tolerance() -> f64 {
    0.05
}

fn default_min_run() -> usize {
    3
}

fn default_anchor_window() -> u64 {
    512
}

fn default_anchor_hop() -> u64 {
    512
}

fn default_anchor_gap() -> u64 {
    256
}

impl Default for LocateConfig {
    fn default() -> Self {
        Self {
            consensus: ConsensusConfig::default(),
            ridge: default_ridge(),
            rate_tolerance: default_rate_tolerance(),
            min_run: default_min_run(),
            anchor_window: default_anchor_window(),
            anchor_hop: default_anchor_hop(),
            anchor_gap: default_anchor_gap(),
        }
    }
}

/// Locate stage.
#[derive(Debug, Default)]
pub struct LocateStage {
    config: LocateConfig,
}

impl LocateStage {
    /// Stage with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stage with an explicit configuration.
    pub fn with_config(config: LocateConfig) -> Self {
        Self { config }
    }
}

fn err(e: impl std::fmt::Display) -> StageError {
    StageError::new(StageKind::Locate, e.to_string())
}

fn prepare_channels(
    cache: &mut InputCache,
    input: &DecodedInput,
    version: &str,
) -> Result<Vec<std::sync::Arc<PreparedChannel>>, &'static str> {
    let specs: Vec<_> = input
        .channel_samples
        .iter()
        .enumerate()
        .map(|(channel, samples)| PrepareSpec {
            id: &input.input_id,
            channel,
            rate: input.sample_rate,
            version,
            samples,
        })
        .collect();
    cache.prepare_many(&specs)
}

/// フォールバック探索の間引き比。粗探索は ~1/8 レートの
/// アンチエイリアス間引き信号上の全域 NCC で候補を絞る。
const HIER_DECIMATION: u32 = 8;
/// 階層探索を適用する最小 signal 長。これ未満では間引き・
/// 精密化・背景推定の固定コストが直接 NCC を上回るため縮退する。
const HIER_MIN_SIGNAL: usize = 128 * 1024;
/// 粗テンプレ長の下限。間引き後にこれ未満しか残らない窓は
/// 粗相関の根拠として弱いため直接探索を使う。
const HIER_MIN_COARSE_TEMPLATE: usize = 8;
/// 粗曲線から原レートで精密化する上位ピーク数。各ピークの
/// 精密化は ±`refine_radius` の小 NCC で済むため、周期性で
/// 混み合う曲線の取りこぼしを抑える目的で深めに取る
/// (実測: K=8 で真の argmax が粗 top-K 圏外になる窓が稀に
/// 存在した)。
const HIER_REFINE_PEAKS: usize = 256;
/// 背景推定に使うフルレート `corr0` の標本数(ラグ軸
/// ストライド抽出)。曲線全体を計算せずに median を推定する。
const HIER_BG_SAMPLES: usize = 16 * 1024;
/// 標本背景の推定誤差が受理閾値(p2b)の採否を左右しないよう、
/// 閾値 ±この割合の帯に落ちる候補は直接探索へ縮退する。
/// (実測: full-curve median と標本 median の差は ~0.2% だが、
/// 閾値直上の候補では採否が反転しうる)
const HIER_P2B_GATE_MARGIN: f64 = 0.02;
/// スコア閾値側の絶対帯。スコアは同一ラグで本質的に同一値だが
/// FFT 丸めで ~1e-14 揺れるため、理論上の境界近傍だけ縮退する。
const HIER_SCORE_GATE_EPS: f64 = 1e-9;

/// 受理済み (source, capture) ペアで共有する間引き済み全チャンネル。
/// `window_anchors_in` のフォールバック層が借用し、窓ごとの再
/// 間引きを避ける。ワーカープールに共有される(信号面は不変、
/// 計測カウンタのみ atomic 更新)。
struct AnchorPyramid {
    /// `coarse_rate` へ間引きした全長ソース(チャンネルごと)。
    source: Vec<Vec<f64>>,
    /// `coarse_rate` へ間引きした全長キャプチャ(チャンネルごと)。
    capture: Vec<Vec<f64>>,
    /// 約分済み有理比 up/down = coarse_rate/rate。orig→coarse は
    /// `pos*up/down`、coarse→orig は `idx*down/up`。
    up: u32,
    down: u32,
    /// 精密化の片側近傍幅(原レート samples)。FIR 半径・位相
    /// 量子化・座標マッピング誤差を内包する。
    refine_radius: usize,
    /// 階層ペア評価の呼出回数(計測用)。
    hier_calls: AtomicUsize,
    /// 階層評価が曖昧さにより直接探索へ縮退した回数
    /// (計測用。理由別: 0=形状 1=粗同点 2=粗候補なし
    /// 3=精密化候補なし 4=精密化後の近同点 5=背景未満
    /// 6=受理閾値近傍)。
    exact_fallbacks: [AtomicUsize; 7],
}

impl AnchorPyramid {
    fn orig_to_coarse_floor(&self, pos: usize) -> usize {
        (pos as u128 * u128::from(self.up) / u128::from(self.down)) as usize
    }

    fn orig_to_coarse_ceil(&self, pos: usize) -> usize {
        ((pos as u128 * u128::from(self.up)).div_ceil(u128::from(self.down))) as usize
    }

    fn coarse_to_orig(&self, idx: usize) -> usize {
        ((idx as u128 * u128::from(self.down) + u128::from(self.up) / 2) / u128::from(self.up))
            as usize
    }
}

/// ~`1/HIER_DECIMATION` レートの間引きピラミッドを共有リサンプラで
/// 構築する。間引き不能なレートやチャンネル欠損では `None`
/// (呼出側は全域で直接探索を使う)。
fn anchor_pyramid(source: &DecodedInput, capture: &DecodedInput) -> Option<AnchorPyramid> {
    let rate = capture.sample_rate;
    let coarse_rate = rate / HIER_DECIMATION;
    if coarse_rate == 0 || source.sample_rate != rate {
        return None;
    }
    let plan = ResamplePlan::new(rate, coarse_rate).ok()?;
    let source = source
        .channel_samples
        .iter()
        .map(|ch| plan.resample(ch).ok())
        .collect::<Option<Vec<_>>>()?;
    let capture = capture
        .channel_samples
        .iter()
        .map(|ch| plan.resample(ch).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(AnchorPyramid {
        source,
        capture,
        up: plan.up(),
        down: plan.down(),
        refine_radius: 2 * plan.radius() + (rate as usize).div_ceil(coarse_rate as usize),
        hier_calls: AtomicUsize::new(0),
        exact_fallbacks: std::array::from_fn(|_| AtomicUsize::new(0)),
    })
}

/// 全曲線を計算せずに NCC 背景を推定する。ラグ軸をストライドで
/// 標本化した `corr0` の median — `peaks_from_correlation` が完全
/// 曲線から求める量と同じものの標本推定。
fn sampled_ncc_background(template: &[f64], signal: &[f64], target_samples: usize) -> f64 {
    let max_lag = signal.len() - template.len();
    let stride = (max_lag / target_samples).max(1);
    let mut vals = Vec::with_capacity(max_lag / stride + 1);
    let mut lag = 0;
    while lag <= max_lag {
        let v = vec::corr0(template, &signal[lag..lag + template.len()]).unwrap_or(0.0);
        vals.push(v.abs());
        lag += stride;
    }
    spec::median(&vals).unwrap_or(0.0).max(1e-12)
}

impl LocateStage {
    /// Explicit non-accepted result for a pair that cannot be
    /// evaluated (missing/empty decode).
    fn marked_result(
        &self,
        source_input_id: &str,
        capture_input_id: &str,
        validity: Validity,
        reason: impl Into<String>,
    ) -> LocateResult {
        LocateResult {
            alignment_id: AlignmentId::new(),
            source_input_id: source_input_id.into(),
            capture_input_id: capture_input_id.into(),
            source_range: SampleRange { start: 0, end: 0 },
            capture_range: SampleRange { start: 0, end: 0 },
            normalized_source_range: SampleRange { start: 0, end: 0 },
            source_pcm: None,
            capture_pcm: None,
            evidence: vec![],
            agreeing_methods: 0,
            position_spread: 0,
            confidence: Confidence::Low,
            validity,
            reject_reason: Some(reason.into()),
            accepted: false,
            algorithm_id: "locate-consensus".into(),
            algorithm_version: "2".into(),
            params: serde_json::to_value(&self.config).unwrap_or_default(),
        }
    }

    /// Run every method on every channel pair of
    /// `source_channels` × `capture_channels`. Returned evidence is
    /// grouped by method (declaration order) so [`merged_votes`] can
    /// pick each method's strongest pair, and the full per-pair list
    /// is kept for the `locate_evidence` artifact so channel detail
    /// survives.
    fn locate_channels(
        &self,
        source_channels: &[Vec<f64>],
        capture_channels: &[Vec<f64>],
        sample_rate: u32,
        prepared_source: &[std::sync::Arc<PreparedChannel>],
        prepared_capture: &[std::sync::Arc<PreparedChannel>],
    ) -> Vec<PairEvidence> {
        // Tasks are (method, source_channel, capture_channel) tuples,
        // method-major so the result order matches the sequential
        // declaration order that merged_votes expects.
        let n_src = source_channels.len();
        let n_cap = capture_channels.len();
        let n_methods = default_methods().len();
        let mut tasks = Vec::with_capacity(n_methods * n_src * n_cap);
        for m in 0..n_methods {
            for (src_ch, prepared_template) in source_channels.iter().zip(prepared_source) {
                for (cap_ch, prepared_signal) in capture_channels.iter().zip(prepared_capture) {
                    tasks.push(MethodTask {
                        method: m,
                        template: src_ch,
                        signal: cap_ch,
                        rate: sample_rate,
                        prepared_template,
                        prepared_signal,
                    });
                }
            }
        }
        run_method_tasks(&tasks)
            .into_iter()
            .enumerate()
            .map(|(idx, output)| PairEvidence {
                source_channel: (idx / n_cap) % n_src,
                capture_channel: idx % n_cap,
                evidence: output.evidence,
                coarse_search: output.coarse_search,
            })
            .collect()
    }

    /// Best full-band NCC peak of `source[..][w_off..w_off+window]`
    /// inside `capture[..][lo..hi]`, taken across every channel pair.
    /// Returns the absolute capture position and score when the peak
    /// clears the consensus thresholds, `None` otherwise.
    ///
    /// `hier` があり探索域が `HIER_MIN_SIGNAL` 以上なら、間引き信号
    /// の粗探索 + 原レート近傍精密化の階層探索で全範囲 NCC を
    /// 置き換える。曖昧なペアは直接探索へ縮退し、直接探索と同一の
    /// 判定を維持する。
    #[allow(clippy::too_many_arguments)]
    fn best_window_match(
        &self,
        source: &DecodedInput,
        capture: &DecodedInput,
        w_off: usize,
        window: usize,
        capture_range: std::ops::Range<usize>,
        pool: &mut CorrelationPool,
        hier: Option<&AnchorPyramid>,
    ) -> Option<(u64, f64)> {
        let (lo, hi) = (capture_range.start, capture_range.end);
        if hi.saturating_sub(lo) < window || w_off + window > source.channel_samples[0].len() {
            return None;
        }
        let hier = hier.filter(|_| hi - lo >= HIER_MIN_SIGNAL);
        let mut best: Option<LocateCandidate> = None;
        for (src_i, src_ch) in source.channel_samples.iter().enumerate() {
            let template = &src_ch[w_off..w_off + window];
            let template_coarse = hier.and_then(|h| {
                let coarse = h.source.get(src_i)?;
                let t_lo = h.orig_to_coarse_floor(w_off).min(coarse.len());
                let t_hi = h.orig_to_coarse_ceil(w_off + window).min(coarse.len());
                (t_hi - t_lo >= HIER_MIN_COARSE_TEMPLATE).then(|| &coarse[t_lo..t_hi])
            });
            for (cap_i, cap_ch) in capture.channel_samples.iter().enumerate() {
                let signal = &cap_ch[lo..hi];
                let candidate = match (
                    hier,
                    template_coarse,
                    hier.and_then(|h| h.capture.get(cap_i)),
                ) {
                    (Some(h), Some(t_c), Some(cap_coarse)) => {
                        self.pair_match_hier(template, signal, t_c, cap_coarse, lo, h, pool)
                    }
                    _ => Self::pair_match(template, signal, pool),
                };
                if let Some(c) = candidate
                    && best.as_ref().is_none_or(|b| c.score > b.score)
                {
                    best = Some(c);
                }
            }
        }
        let c = best?;
        (c.score >= self.config.consensus.min_score
            && c.peak_to_background >= self.config.consensus.min_peak_to_background)
            .then(|| (lo as u64 + c.position, c.score))
    }

    /// 直接ペア評価: 全域 NCC 曲線 → argmax 同点ガード → トップ
    /// ピーク。同点・背景未満のみの場合、ペアは候補を寄与しない。
    fn pair_match(
        template: &[f64],
        signal: &[f64],
        pool: &mut CorrelationPool,
    ) -> Option<LocateCandidate> {
        if signal.len() < template.len() {
            return None;
        }
        let corr = pool.ncc(template, signal, signal.len() - template.len());
        if argmax_tie_count(corr) >= 2 {
            return None;
        }
        peaks_from_correlation(corr, 1).into_iter().next()
    }

    /// 大探索域の階層ペア評価。間引き信号上の粗 NCC が候補ラグを
    /// 提案し、各候補を原レート近傍で精密化して勝者を決め、
    /// ストライド `corr0` 標本で直接経路の全曲線と同じ背景を推定
    /// する。一意な勝者や受理判定を粗・標本証拠で決められない
    /// 場合(粗同点・候補なし・精密化後の厳密同点・背景未満・
    /// 受理閾値近傍)は [`Self::pair_match`] へ縮退し、曖昧な
    /// ケースで直接探索の厳密な結果を保持する。
    #[allow(clippy::too_many_arguments)]
    fn pair_match_hier(
        &self,
        template: &[f64],
        signal: &[f64],
        template_coarse: &[f64],
        capture_coarse: &[f64],
        lo: usize,
        hier: &AnchorPyramid,
        pool: &mut CorrelationPool,
    ) -> Option<LocateCandidate> {
        if signal.len() < template.len() {
            return None;
        }
        hier.hier_calls.fetch_add(1, Ordering::Relaxed);
        let exact = |pool: &mut CorrelationPool, reason: usize| {
            hier.exact_fallbacks[reason].fetch_add(1, Ordering::Relaxed);
            Self::pair_match(template, signal, pool)
        };
        let c_lo = hier.orig_to_coarse_floor(lo).min(capture_coarse.len());
        let c_hi = hier
            .orig_to_coarse_ceil(lo + signal.len())
            .min(capture_coarse.len());
        let signal_coarse = &capture_coarse[c_lo..c_hi.max(c_lo)];
        if template_coarse.is_empty() || signal_coarse.len() <= template_coarse.len() {
            return exact(pool, 0);
        }
        // 粗曲線は短いテンプレ由来でピーク群が混み合うため、
        // 「競合数」を曖昧さの根拠にはしない(実測で大半が競合判定
        // となり階層化が無効化される)。曖昧さは粗同点・候補なし・
        // 精密化後の近同点・背景未満で判定し、いずれも直接探索へ
        // 縮退する。
        let (tied, peaks) = {
            let corr = pool.ncc(
                template_coarse,
                signal_coarse,
                signal_coarse.len() - template_coarse.len(),
            );
            (
                argmax_tie_count(corr),
                peaks_from_correlation(corr, HIER_REFINE_PEAKS),
            )
        };
        if tied >= 2 {
            return exact(pool, 1);
        }
        if peaks.is_empty() {
            return exact(pool, 2);
        }
        let max_lag = signal.len() - template.len();
        let radius = hier.refine_radius;
        let mut refined: Vec<(usize, f64, u64)> = Vec::with_capacity(peaks.len());
        for peak in &peaks {
            let est = hier.coarse_to_orig(c_lo + peak.position as usize);
            let center = est.saturating_sub(lo).min(max_lag);
            let (a, b) = (
                center.saturating_sub(radius),
                (center + radius).min(max_lag),
            );
            if a >= b {
                continue;
            }
            let (arg, score, width) = {
                let corr = pool.ncc(template, &signal[a..b + template.len()], b - a);
                let (i, &v) = corr
                    .iter()
                    .enumerate()
                    .max_by(|(_, x), (_, y)| x.abs().total_cmp(&y.abs()))
                    .unwrap();
                let half = v.abs() / 2.0;
                let mut w_lo = i;
                while w_lo > 0 && corr[w_lo].abs() > half {
                    w_lo -= 1;
                }
                let mut w_hi = i;
                while w_hi + 1 < corr.len() && corr[w_hi].abs() > half {
                    w_hi += 1;
                }
                (i, v, (w_hi - w_lo) as u64)
            };
            refined.push((a + arg, score, width));
        }
        if refined.is_empty() {
            return exact(pool, 3);
        }
        // 同一ピーク近傍(±2·refine_radius)はクラスタとして最高
        // |score| を残す。ラグが十分離れた候補がトップに近接する
        // 場合は、直接探索の argmax と階層側の勝者が入れ替わりうる
        // 曖昧な状態とみなし直接探索へ縮退する。
        refined.sort_by(|x, y| y.1.abs().total_cmp(&x.1.abs()));
        let mut clusters: Vec<(usize, f64, u64)> = Vec::with_capacity(refined.len());
        'candidates: for (pos, s, w) in refined {
            for c in clusters.iter_mut() {
                if pos.abs_diff(c.0) <= 2 * radius {
                    if s.abs() > c.1.abs() {
                        *c = (pos, s, w);
                    }
                    continue 'candidates;
                }
            }
            clusters.push((pos, s, w));
        }
        // 離れた位置の厳密同点のみ縮退対象とする。既存の同点判定
        // (argmax_tie_count と同じ tie_window)に揃える — それより
        // 粗い近同点判定は音楽信号の自己相似で事実上常に発火し、
        // 階層化が機能しなくなる(実測 97% が縮退)。粗 top-K に
        // 入らなかった argmax は近同点ではなくカバレッジの問題で、
        // K の深さで対処する。
        let m = clusters[0].1.abs();
        let tie_window = (TIE_REL_EPS * m).max(TIE_ABS_FLOOR);
        if clusters[1..].iter().any(|c| c.1.abs() >= m - tie_window) {
            return exact(pool, 4);
        }
        let (pos, score, width) = clusters[0];
        // rank-1 の 1st/2nd 比は精密化候補の 2 位を用いる(全曲線の
        // 2 位ではないが、このフィールドは anchor 受理判定に未使用)。
        let ratio_12 = clusters.get(1).map(|c| score.abs() / c.1.abs().max(1e-12));
        let background = sampled_ncc_background(template, signal, HIER_BG_SAMPLES);
        if score.abs() <= background {
            return exact(pool, 5);
        }
        // 標本背景は full-curve median の近似のため、受理閾値の
        // 近傍では採否が直接経路とずれうる。閾値帯の候補は厳密な
        // 背景を持つ直接探索で判定する(稀な境界ケースのみ)。
        let p2b = score.abs() / background;
        let near_p2b = (p2b - self.config.consensus.min_peak_to_background).abs()
            <= HIER_P2B_GATE_MARGIN * self.config.consensus.min_peak_to_background;
        let near_score = (score - self.config.consensus.min_score).abs() <= HIER_SCORE_GATE_EPS;
        if near_p2b || near_score {
            return exact(pool, 6);
        }
        Some(LocateCandidate {
            position: pos as u64,
            score,
            peak_to_background: score.abs() / background,
            peak_width: width,
            peak_ratio_12: ratio_12,
            channel_pair: None,
        })
    }

    /// One anchor per source window, located inside `capture[lo..hi]`
    /// around its expected position. `origin` is the capture position
    /// where the occurrence's body start lands; each window's search
    /// range is `origin + w_off` widened by the accumulated rate
    /// drift, position tolerance and gap slack — searching the whole
    /// capture per window is both costlier and less correct, since a
    /// few-hundred-sample window can false-match anywhere. Anchor
    /// source coordinates are `base + w_off` so callers can place
    /// them on the unrolled pass grid; windows whose peak stays below
    /// the consensus thresholds contribute no anchor. Returns an
    /// empty list when the source fits fewer than two windows.
    ///
    /// `hier` はペア共有の間引きピラミッド。フォールバック層
    /// (widen/full)の大探索域で粗探索+精密化を有効化する。
    #[allow(clippy::too_many_arguments)]
    fn window_anchors_in(
        &self,
        source: &DecodedInput,
        capture: &DecodedInput,
        lo: usize,
        hi: usize,
        base: u64,
        origin: u64,
        hier: Option<&AnchorPyramid>,
    ) -> Vec<Anchor> {
        let window = self.config.anchor_window as usize;
        let hop = self.config.anchor_hop.max(1) as usize;
        let n = source.channel_samples[0].len();
        if window == 0 || n < window + hop {
            return Vec::new();
        }
        // Bound the window count for long sources: a dense grid over
        // tens of seconds of audio buys no extra map resolution.
        const MAX_WINDOWS: usize = 512;
        let hop = hop.max((n - window) / MAX_WINDOWS);
        let mut offsets: Vec<usize> = (0..=n - window).step_by(hop).collect();
        if offsets.last().copied() != Some(n - window) {
            offsets.push(n - window);
        }
        let tol = self.config.consensus.position_tolerance;
        let gap = self.config.anchor_gap;
        let rate_tol = self.config.rate_tolerance;
        // 層到達回数・pool 計測(計測イベント用。並列窓処理から
        // 加算されるため atomic)。
        let (hit_narrow, hit_widen, hit_full) = (
            AtomicUsize::new(0),
            AtomicUsize::new(0),
            AtomicUsize::new(0),
        );
        let (pool_builds, pool_hits) = (AtomicUsize::new(0), AtomicUsize::new(0));
        let started = std::time::Instant::now();
        let anchors: Vec<Anchor> = run_pool(offsets.len(), &CorrelationPool::new, &|i, pool| {
            let w_off = offsets[i];
            let s0 = pool.stats();
            let slack = (w_off as f64 * rate_tol) as u64 + tol + gap;
            let expected = origin + w_off as u64;
            let w_lo = expected.saturating_sub(slack).max(lo as u64) as usize;
            let w_hi = (expected + window as u64 + slack).min(hi as u64) as usize;
            // Narrow first; widen ×8 so jumps still land before paying
            // for the caller's whole range, which misses and large
            // deviations still reach.
            hit_narrow.fetch_add(1, Ordering::Relaxed);
            let result = self
                .best_window_match(source, capture, w_off, window, w_lo..w_hi, pool, None)
                .or_else(|| {
                    let slack8 = slack.saturating_mul(8);
                    let m_lo = expected.saturating_sub(slack8).max(lo as u64) as usize;
                    let m_hi = (expected + window as u64 + slack8).min(hi as u64) as usize;
                    ((m_lo, m_hi) != (w_lo, w_hi))
                        .then(|| {
                            hit_widen.fetch_add(1, Ordering::Relaxed);
                            self.best_window_match(
                                source,
                                capture,
                                w_off,
                                window,
                                m_lo..m_hi,
                                pool,
                                hier,
                            )
                        })
                        .flatten()
                })
                .or_else(|| {
                    hit_full.fetch_add(1, Ordering::Relaxed);
                    self.best_window_match(source, capture, w_off, window, lo..hi, pool, hier)
                });
            let s1 = pool.stats();
            pool_builds.fetch_add((s1.builds - s0.builds) as usize, Ordering::Relaxed);
            pool_hits.fetch_add((s1.hits - s0.hits) as usize, Ordering::Relaxed);
            result.map(|(capture, score)| Anchor {
                source: base + w_off as u64,
                capture,
                score,
            })
        })
        .into_iter()
        .flatten()
        .collect();
        tracing::info!(
            windows = offsets.len(),
            range = hi - lo,
            ncc_narrow = hit_narrow.load(Ordering::Relaxed),
            ncc_widen = hit_widen.load(Ordering::Relaxed),
            ncc_full = hit_full.load(Ordering::Relaxed),
            pool_builds = pool_builds.load(Ordering::Relaxed),
            pool_hits = pool_hits.load(Ordering::Relaxed),
            hier_calls = hier.map_or(0, |h| h.hier_calls.load(Ordering::Relaxed)),
            exact_fallbacks = hier.map_or(0, |h| {
                h.exact_fallbacks
                    .iter()
                    .map(|a| a.load(Ordering::Relaxed))
                    .sum::<usize>()
            }),
            fb_shape = hier.map_or(0, |h| h.exact_fallbacks[0].load(Ordering::Relaxed)),
            fb_coarse_tie = hier.map_or(0, |h| h.exact_fallbacks[1].load(Ordering::Relaxed)),
            fb_coarse_empty = hier.map_or(0, |h| h.exact_fallbacks[2].load(Ordering::Relaxed)),
            fb_refined_empty = hier.map_or(0, |h| h.exact_fallbacks[3].load(Ordering::Relaxed)),
            fb_refined_tie = hier.map_or(0, |h| h.exact_fallbacks[4].load(Ordering::Relaxed)),
            fb_background = hier.map_or(0, |h| h.exact_fallbacks[5].load(Ordering::Relaxed)),
            fb_near_gate = hier.map_or(0, |h| h.exact_fallbacks[6].load(Ordering::Relaxed)),
            anchors = anchors.len(),
            elapsed_ms = started.elapsed().as_secs_f64() * 1000.0,
            "window_anchors_in"
        );
        anchors
    }

    /// Persist one JSON artifact under `dir` and return its record.
    #[allow(clippy::too_many_arguments)]
    fn persist_json(
        &self,
        dir: &std::path::Path,
        name: &str,
        role: &str,
        body: &serde_json::Value,
        lineage: &[al_core::ArtifactId],
        params: serde_json::Value,
        ctx: &StageContext<'_>,
    ) -> Result<ArtifactRecord, StageError> {
        let job_dir = ctx.registry.job_dir();
        fs::create_dir_all(job_dir.join(dir)).map_err(err)?;
        let rel = dir.join(name);
        al_job::fsutil::write_durable(
            &job_dir.join(&rel),
            serde_json::to_string_pretty(body).map_err(err)?.as_bytes(),
        )
        .map_err(err)?;
        Ok(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: role.into(),
            path: rel,
            produced_by: Some(StageKind::Locate),
            inputs: lineage.to_vec(),
            params,
        })
    }
}

/// Method evidence from one (source channel, capture channel) pair.
#[derive(Debug, Clone, serde::Serialize)]
struct PairEvidence {
    source_channel: usize,
    capture_channel: usize,
    #[serde(flatten)]
    evidence: MethodEvidence,
    /// チャンネルペア共通の候補提案を FullNcc 行に一度だけ記録する。
    #[serde(skip_serializing_if = "Option::is_none")]
    coarse_search: Option<CoarseSearch>,
}

struct MethodTask<'a> {
    method: usize,
    template: &'a [f64],
    signal: &'a [f64],
    rate: u32,
    prepared_template: &'a PreparedChannel,
    prepared_signal: &'a PreparedChannel,
}

struct MethodOutput {
    evidence: MethodEvidence,
    coarse_search: Option<CoarseSearch>,
}

/// Run `(method_index, template, signal, sample_rate)` tasks over a
/// worker pool, returning one [`MethodEvidence`] per task in input
/// order. Locate methods are pure but not `Send`, so each worker
/// builds its own method set and only evidence crosses the thread
/// boundary.
fn run_method_tasks(tasks: &[MethodTask<'_>]) -> Vec<MethodOutput> {
    run_pool(tasks.len(), &CorrelationPool::new, &|idx, pool| {
        let methods = default_methods();
        let task = &tasks[idx];
        if methods[task.method].id() == "full_ncc" {
            let (evidence, coarse_search) = FullNcc.locate_coarse_to_fine(
                task.template,
                task.signal,
                task.rate,
                task.prepared_template,
                task.prepared_signal,
                pool,
            );
            return MethodOutput {
                evidence,
                coarse_search,
            };
        }
        // F3: 候補提案の成否に関係なく、証拠は全レート・元の全探索域から生成。
        let evidence = methods[task.method].locate_prepared(
            task.template,
            task.signal,
            task.rate,
            task.prepared_template,
            task.prepared_signal,
            pool,
        );
        MethodOutput {
            evidence,
            coarse_search: None,
        }
    })
}

/// Keep, per method, the evidence of the strongest channel pair.
/// `pairs` must be grouped by method (see [`LocateStage::locate_channels`]).
/// 非曖昧の証拠は曖昧な証拠より先頭スコアに依らず優先される —
/// merged が曖昧となるのは全ペアが曖昧の場合に限る。
fn merged_votes(pairs: &[PairEvidence]) -> Vec<MethodEvidence> {
    let key = |e: &MethodEvidence| {
        (
            e.validity != Validity::AmbiguousAlignment,
            e.candidates.first().map(|c| c.score).unwrap_or(0.0),
        )
    };
    let mut merged: Vec<MethodEvidence> = Vec::new();
    for pair in pairs {
        match merged.last_mut() {
            Some(m) if m.method == pair.evidence.method => {
                if key(&pair.evidence) > key(m) {
                    *m = pair.evidence.clone();
                }
            }
            _ => merged.push(pair.evidence.clone()),
        }
    }
    merged
}

impl Stage for LocateStage {
    fn kind(&self) -> StageKind {
        StageKind::Locate
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        // Overlay locate config from the job config, when present.
        if let Some(v) = ctx.config.get("locate") {
            self.config = serde_json::from_value(v.clone()).map_err(err)?;
        }

        let sources: Vec<_> = ctx
            .manifest
            .inputs
            .iter()
            .filter(|i| i.role == InputRole::SourceAsset)
            .collect();
        let captures: Vec<_> = ctx
            .manifest
            .inputs
            .iter()
            .filter(|i| matches!(i.role, InputRole::CaptureBug | InputRole::CaptureReference))
            .collect();
        // Nothing to pair (capture-only or source-only job) →
        // complete with empty output so the pipeline can continue to
        // capture-only analysis.
        if sources.is_empty() || captures.is_empty() {
            return Ok(Vec::new());
        }

        // Decode every input once up front; the residual pass needs
        // all sources after the per-pair results exist. A missing
        // decode becomes a per-pair mark below, never an abort for
        // the whole stage.
        let mut decoded_sources = Vec::with_capacity(sources.len());
        for input in &sources {
            decoded_sources.push(decode_input(input, ctx).map_err(err)?);
        }
        let mut decoded_captures = Vec::with_capacity(captures.len());
        for input in &captures {
            decoded_captures.push(decode_input(input, ctx).map_err(err)?);
        }

        let mut records = Vec::new();
        let mut dsp_inputs = InputCache::default();
        for (cap_index, cap_input) in captures.iter().enumerate() {
            // A capture with missing or empty decoded PCM marks every
            // source pair; it never aborts the stage.
            let capture_mark = match &decoded_captures[cap_index] {
                None => Some((
                    Validity::DecodeInputUnavailable,
                    format!("no decoded_pcm artifact for capture {}", cap_input.input_id),
                )),
                Some(c)
                    if c.channel_samples.is_empty()
                        || c.channel_samples.iter().all(|ch| ch.is_empty()) =>
                {
                    Some((
                        Validity::InsufficientSupport,
                        "decoded PCM is empty".to_string(),
                    ))
                }
                Some(_) => None,
            };
            if let Some((validity, reason)) = capture_mark {
                for (src_index, src_input) in sources.iter().enumerate() {
                    let dir = PathBuf::from("locate")
                        .join(src_input.input_id.to_string())
                        .join(cap_input.input_id.to_string());
                    let lineage = decoded_sources[src_index]
                        .as_ref()
                        .map(|s| s.artifact_ids.clone())
                        .unwrap_or_default();
                    let result = self.marked_result(
                        &src_input.input_id.to_string(),
                        &cap_input.input_id.to_string(),
                        validity,
                        reason.clone(),
                    );
                    records.push(self.persist_json(
                        &dir,
                        "result.json",
                        "locate_result",
                        &serde_json::json!({"result": result, "derived": {}}),
                        &lineage,
                        serde_json::json!({
                            "source_input_id": result.source_input_id,
                            "capture_input_id": result.capture_input_id,
                            "accepted": result.accepted,
                            "validity": result.validity,
                            "alignment_id": result.alignment_id,
                        }),
                        ctx,
                    )?);
                }
                continue;
            }
            let capture_original = decoded_captures[cap_index].as_ref().unwrap();
            let prepared_capture = prepare_input(
                capture_original,
                capture_original.sample_rate,
                &mut records,
                ctx,
            )
            .map_err(err)?;
            let capture = &prepared_capture.decoded;
            let prepared_sources = decoded_sources
                .iter()
                .map(|s| {
                    s.as_ref()
                        .map(|s| prepare_input(s, capture.sample_rate, &mut records, ctx))
                        .transpose()
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(err)?;
            let decoded_sources: Vec<Option<&DecodedInput>> = prepared_sources
                .iter()
                .map(|s| s.as_ref().map(|p| &p.decoded))
                .collect();
            // キャプチャと有効 source の全チャンネル準備を一括で並列化する。
            // 下流の prepare_channels 呼出はキャッシュ hit で解決する。
            // mark で弾かれる source(チャンネル無し/全空)は従来どおり
            // 準備対象から外す。
            let specs: Vec<PrepareSpec> = capture
                .channel_samples
                .iter()
                .enumerate()
                .map(|(channel, samples)| PrepareSpec {
                    id: &capture.input_id,
                    channel,
                    rate: capture.sample_rate,
                    version: "original",
                    samples,
                })
                .chain(
                    decoded_sources
                        .iter()
                        .flatten()
                        .filter(|s| {
                            !s.channel_samples.is_empty()
                                && !s.channel_samples.iter().all(|c| c.is_empty())
                        })
                        .flat_map(|s| {
                            s.channel_samples
                                .iter()
                                .enumerate()
                                .map(|(channel, samples)| PrepareSpec {
                                    id: &s.input_id,
                                    channel,
                                    rate: s.sample_rate,
                                    version: "original",
                                    samples,
                                })
                        }),
                )
                .collect();
            dsp_inputs.prepare_many(&specs).map_err(err)?;
            let capture_dsp =
                prepare_channels(&mut dsp_inputs, capture, "original").map_err(err)?;
            let capture_len = capture.channel_samples[0].len() as u64;
            let tolerance = self.config.consensus.position_tolerance;
            // Per-source results kept alive for the residual pass;
            // accepted pairs additionally carry their `locate_result`
            // artifact id for lineage.
            let mut results: Vec<Option<LocateResult>> = Vec::new();
            results.resize_with(decoded_sources.len(), || None);
            let mut accepted: Vec<(usize, al_core::ArtifactId)> = Vec::new();
            for (src_index, src_input) in sources.iter().enumerate() {
                let dir = PathBuf::from("locate")
                    .join(src_input.input_id.to_string())
                    .join(cap_input.input_id.to_string());
                let mut lineage = decoded_sources[src_index]
                    .as_ref()
                    .map(|s| s.artifact_ids.clone())
                    .unwrap_or_default();
                lineage.extend(capture.artifact_ids.iter().cloned());

                // Missing or empty decode → per-pair explicit mark;
                // One bad pair never aborts the others.
                let mark = match &decoded_sources[src_index] {
                    None => Some((
                        Validity::DecodeInputUnavailable,
                        format!("no decoded_pcm artifact for source {}", src_input.input_id),
                        serde_json::json!({}),
                    )),
                    Some(s)
                        if s.channel_samples.is_empty()
                            || s.channel_samples.iter().all(|c| c.is_empty()) =>
                    {
                        Some((
                            Validity::InsufficientSupport,
                            "decoded PCM is empty".to_string(),
                            serde_json::json!({"empty_input": true}),
                        ))
                    }
                    Some(_) => None,
                };
                if let Some((validity, reason, derived)) = mark {
                    let result = self.marked_result(
                        &src_input.input_id.to_string(),
                        &cap_input.input_id.to_string(),
                        validity,
                        reason,
                    );
                    records.push(self.persist_json(
                        &dir,
                        "result.json",
                        "locate_result",
                        &serde_json::json!({"result": result, "derived": derived}),
                        &lineage,
                        serde_json::json!({
                            "source_input_id": result.source_input_id,
                            "capture_input_id": result.capture_input_id,
                            "accepted": result.accepted,
                            "validity": result.validity,
                            "alignment_id": result.alignment_id,
                        }),
                        ctx,
                    )?);
                    results[src_index] = Some(result);
                    continue;
                }
                let source = decoded_sources[src_index].as_ref().unwrap();
                let source_dsp =
                    prepare_channels(&mut dsp_inputs, source, "original").map_err(err)?;

                let pairs = self.locate_channels(
                    &source.channel_samples,
                    &capture.channel_samples,
                    source.sample_rate,
                    &source_dsp,
                    &capture_dsp,
                );
                let evidence = merged_votes(&pairs);
                let source_len = source.channel_samples[0].len() as u64;
                let mut result = evaluate(
                    &evidence,
                    &source.input_id,
                    &capture.input_id,
                    source_len,
                    capture_len,
                    &self.config.consensus,
                    "locate-consensus",
                    "2",
                );

                prepared_sources[src_index]
                    .as_ref()
                    .unwrap()
                    .attach(&prepared_capture, &mut result)
                    .map_err(err)?;

                // Every channel pair's evidence is its own artifact so
                // per-channel detail survives the vote merge.
                records.push(self.persist_json(
                    &dir,
                    "evidence.json",
                    "locate_evidence",
                    &serde_json::json!({"evidence": pairs}),
                    &lineage,
                    serde_json::json!({
                        "source_input_id": source.input_id,
                        "capture_input_id": capture.input_id,
                        "channel_pairs": pairs.len(),
                    }),
                    ctx,
                )?);

                // Additional occurrences: cluster every usable
                // candidate beyond the primary position by width, so
                // one physical pass contributes a single occurrence
                // and multiple legitimate appearances stay separate.
                let mut occurrences: Vec<u64> = Vec::new();
                let mut scores: std::collections::HashMap<u64, f64> =
                    std::collections::HashMap::new();
                if result.accepted {
                    let mut positions: Vec<u64> = Vec::new();
                    for pair in &pairs {
                        if pair.evidence.validity != Validity::Valid {
                            continue;
                        }
                        for c in &pair.evidence.candidates {
                            if c.score >= self.config.consensus.min_score
                                && c.peak_to_background
                                    >= self.config.consensus.min_peak_to_background
                            {
                                positions.push(c.position);
                                let e = scores.entry(c.position).or_insert(c.score);
                                *e = e.max(c.score);
                            }
                        }
                    }
                    positions.sort_unstable();
                    positions.dedup();
                    let primary = result.capture_range.start;
                    let mut cluster: Vec<u64> = Vec::new();
                    for &p in &positions {
                        if p.abs_diff(primary) <= tolerance {
                            continue;
                        }
                        match cluster.first() {
                            Some(&first) if p - first <= tolerance => cluster.push(p),
                            _ => {
                                if !cluster.is_empty() {
                                    occurrences.push(cluster[(cluster.len() - 1) / 2]);
                                }
                                cluster = vec![p];
                            }
                        }
                    }
                    if !cluster.is_empty() {
                        occurrences.push(cluster[(cluster.len() - 1) / 2]);
                    }
                }

                // Derived data for the primary result: mixer fit on the
                // matched window; loop occurrences and the piecewise
                // time map are their own artifacts below.
                let mut derived = serde_json::json!({"occurrences": occurrences, "resampled": prepared_sources[src_index].as_ref().unwrap().params.original_sample_rate != capture.sample_rate});
                let mut time_map_body = None;
                if result.accepted {
                    let primary = result.capture_range.start;
                    let start = primary as usize;
                    let len = source_len as usize;
                    if start + len <= capture.channel_samples[0].len() {
                        let cap_window: Vec<&[f64]> = capture
                            .channel_samples
                            .iter()
                            .map(|c| &c[start..start + len])
                            .collect();
                        let src_cols: Vec<&[f64]> =
                            source.channel_samples.iter().map(Vec::as_slice).collect();
                        if let Some(mix) = fit_mix(&src_cols, &cap_window, self.config.ridge) {
                            derived["mixer"] = serde_json::to_value(&mix).unwrap_or_default();
                        }
                    }

                    // Loop occurrences anchor the pass grid at the
                    // accepted position; time-map anchors then unroll
                    // each pass to `loop_index * body_len` so spacing
                    // deviations surface as real segment rates.
                    let mut occurrence_positions = vec![primary];
                    occurrence_positions.extend(&occurrences);
                    // Sorted order so each occurrence's midpoint cell
                    // covers real capture regions — the primary can
                    // land anywhere in the sorted list.
                    occurrence_positions.sort_unstable();
                    let body_len = source_len.max(1);
                    let loops = identify_loop_occurrences(&occurrence_positions, body_len, primary);
                    let min_index = loops.iter().map(|o| o.loop_index).min().unwrap_or(0);
                    // An occurrence is only real when the body start
                    // window matches at its own position (within
                    // position tolerance); secondary correlation
                    // peaks are not replays. Unverified candidates
                    // are dropped before the pass grid and the
                    // reported occurrences are built.
                    let cap_n = capture.channel_samples[0].len();
                    let detected: Vec<u64> = loops.iter().map(|o| o.occurrence_position).collect();
                    let cell_of = |positions: &[u64], i: usize| -> (usize, usize) {
                        let lo = if i == 0 {
                            0
                        } else {
                            ((positions[i - 1] + positions[i]) / 2) as usize
                        };
                        let hi = if i + 1 == positions.len() {
                            cap_n
                        } else {
                            ((positions[i] + positions[i + 1]) / 2) as usize
                        };
                        (lo, hi)
                    };
                    // The verification template is the body start —
                    // clamped to the body so sources shorter than the
                    // anchor window still verify whole-body replays.
                    let window =
                        (self.config.anchor_window as usize).min(source.channel_samples[0].len());
                    let tol = tolerance as usize;
                    let mut pool = CorrelationPool::new();
                    let verified: Vec<usize> = (0..loops.len())
                        .filter(|&i| {
                            detected[i] == primary || {
                                let pos = detected[i] as usize;
                                let lo = pos.saturating_sub(tol);
                                let hi = (pos + window + tol).min(cap_n);
                                self.best_window_match(
                                    source,
                                    capture,
                                    0,
                                    window,
                                    lo..hi,
                                    &mut pool,
                                    None,
                                )
                                .is_some()
                            }
                        })
                        .collect();
                    let loops: Vec<_> = verified.iter().map(|&i| loops[i].clone()).collect();
                    let positions: Vec<u64> = loops.iter().map(|o| o.occurrence_position).collect();
                    // Reported occurrences are verified replays only
                    // (every position but the primary).
                    occurrences = positions
                        .iter()
                        .copied()
                        .filter(|&p| p != primary)
                        .collect();
                    derived["occurrences"] = serde_json::json!(occurrences);

                    // A source long enough to split into windows gets
                    // one anchor per window, so drift, jitter, skips,
                    // gaps and resumes inside a single occurrence
                    // stay observable. With one verified occurrence
                    // the mapping is tracked in plain source
                    // coordinates; several verified occurrences are
                    // real passes, so their windows unroll onto the
                    // pass grid.
                    let hier = anchor_pyramid(source, capture);
                    let mut anchors = if loops.len() <= 1 {
                        self.window_anchors_in(source, capture, 0, cap_n, 0, primary, hier.as_ref())
                    } else {
                        let min_v = loops.iter().map(|o| o.loop_index).min().unwrap_or(0);
                        loops
                            .iter()
                            .enumerate()
                            .flat_map(|(i, o)| {
                                let base = ((o.loop_index - min_v) * body_len as i64) as u64;
                                let (lo, hi) = cell_of(&positions, i);
                                self.window_anchors_in(
                                    source,
                                    capture,
                                    lo,
                                    hi,
                                    base,
                                    positions[i],
                                    hier.as_ref(),
                                )
                            })
                            .collect()
                    };
                    if anchors.is_empty() {
                        // Short sources (and sources whose windows all
                        // failed to match) keep one anchor per
                        // occurrence on the unrolled pass grid.
                        for o in &loops {
                            anchors.push(Anchor {
                                source: ((o.loop_index - min_index) * body_len as i64) as u64,
                                capture: o.occurrence_position,
                                score: scores.get(&o.occurrence_position).copied().unwrap_or(0.0),
                            });
                        }
                    }
                    anchors.sort_by_key(|a| a.capture);
                    let time_map = build_time_map(
                        &anchors,
                        self.config.rate_tolerance,
                        self.config.anchor_gap,
                        self.config.min_run,
                    );
                    time_map_body = Some(serde_json::json!({
                        "time_map": time_map,
                        "anchors": anchors,
                        "loop_occurrences": loops,
                        "anchor_reference": primary,
                        "source_coordinate_space": "normalized_pcm",
                        "source_pcm": result.source_pcm,
                        "capture_pcm": result.capture_pcm,
                    }));
                }

                if let Some(body) = time_map_body {
                    records.push(self.persist_json(
                        &dir,
                        "time_map.json",
                        "time_map",
                        &body,
                        &lineage,
                        serde_json::json!({
                            "source_input_id": source.input_id,
                            "capture_input_id": capture.input_id,
                        }),
                        ctx,
                    )?);
                }

                let accepted_result = result.accepted;
                let record = self.persist_json(
                    &dir,
                    "result.json",
                    "locate_result",
                    &serde_json::json!({
                        "result": result,
                        "derived": derived,
                    }),
                    &lineage,
                    serde_json::json!({
                        "source_input_id": result.source_input_id,
                        "capture_input_id": result.capture_input_id,
                        "accepted": result.accepted,
                        "validity": result.validity,
                        "alignment_id": result.alignment_id,
                    }),
                    ctx,
                )?;
                if accepted_result {
                    accepted.push((src_index, record.artifact_id.clone()));
                }
                results[src_index] = Some(result);
                records.push(record);
            }

            // Two-stage residual search: for each accepted alignment,
            // subtract the fitted mix from the capture and re-run the
            // locate methods for every source on the residual —
            // quieter sources and repeat occurrences surface once the
            // dominant contribution is removed. Provenance (parent
            // source, alignment, transform, energies, child result) is
            // persisted per (parent, child) pair.
            for &(parent_idx, ref parent_artifact) in &accepted {
                let Some(parent) = &decoded_sources[parent_idx] else {
                    continue;
                };
                let parent_result = results[parent_idx].as_ref().unwrap();
                let start = parent_result.capture_range.start as usize;
                let span =
                    (parent_result.capture_range.end - parent_result.capture_range.start) as usize;
                if span == 0 {
                    continue;
                }
                let mut aligned =
                    vec![vec![0.0; capture_len as usize]; parent.channel_samples.len()];
                for (a, s) in aligned.iter_mut().zip(&parent.channel_samples) {
                    a[start..start + span].copy_from_slice(&s[..span]);
                }
                // The residual depends only on the parent — compute it
                // once, then run every child's method tasks over it in
                // one shared pool.
                let Some(prepared) =
                    prepare_residual(&aligned, &capture.channel_samples, self.config.ridge)
                else {
                    continue;
                };
                let prepared_parent = prepared_sources[parent_idx].as_ref().unwrap();
                let residual_pcm = persist_residual(
                    &prepared,
                    prepared_parent,
                    &prepared_capture,
                    parent_result,
                    parent_artifact,
                    ctx,
                )
                .map_err(err)?;
                records.push(residual_pcm.record.clone());
                let children: Vec<&DecodedInput> = decoded_sources
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|c| c.sample_rate == capture.sample_rate)
                    .collect();
                let n_methods = default_methods().len();
                let n_cap = prepared.channels.len();
                // 親ごとに版を分離し、この残差の全 child が完了した時点で解放する。
                let version = format!("residual:{}", parent_result.alignment_id);
                let mut residual_cache = InputCache::default();
                let residual_specs: Vec<_> = prepared
                    .channels
                    .iter()
                    .enumerate()
                    .map(|(channel, samples)| PrepareSpec {
                        id: &capture.input_id,
                        channel,
                        rate: capture.sample_rate,
                        version: &version,
                        samples,
                    })
                    .collect();
                let residual_dsp = residual_cache.prepare_many(&residual_specs).map_err(err)?;
                // 全 child のチャンネル準備を一括で並列化し、結果は
                // spec と同じ (child, channel) 順で返る。
                let child_specs: Vec<_> = children
                    .iter()
                    .flat_map(|child| {
                        child
                            .channel_samples
                            .iter()
                            .enumerate()
                            .map(|(channel, samples)| PrepareSpec {
                                id: &child.input_id,
                                channel,
                                rate: child.sample_rate,
                                version: "original",
                                samples,
                            })
                    })
                    .collect();
                let child_flat = dsp_inputs.prepare_many(&child_specs).map_err(err)?;
                let mut child_flat = child_flat.into_iter();
                let child_dsp: Vec<_> = children
                    .iter()
                    .map(|child| {
                        (0..child.channel_samples.len())
                            .map(|_| child_flat.next().expect("one spec per channel"))
                            .collect::<Vec<_>>()
                    })
                    .collect();
                let mut tasks = Vec::new();
                for (child, prepared_child) in children.iter().zip(&child_dsp) {
                    for m in 0..n_methods {
                        for (src_ch, prepared_template) in
                            child.channel_samples.iter().zip(prepared_child)
                        {
                            for (cap_ch, prepared_signal) in
                                prepared.channels.iter().zip(&residual_dsp)
                            {
                                tasks.push(MethodTask {
                                    method: m,
                                    template: src_ch,
                                    signal: cap_ch,
                                    rate: child.sample_rate,
                                    prepared_template,
                                    prepared_signal,
                                });
                            }
                        }
                    }
                }
                let mut evidences = run_method_tasks(&tasks).into_iter();
                for child in children {
                    let n_src = child.channel_samples.len();
                    let pairs: Vec<PairEvidence> = (0..n_methods * n_src * n_cap)
                        .map(|idx| {
                            let output = evidences.next().unwrap();
                            PairEvidence {
                                source_channel: (idx / n_cap) % n_src,
                                capture_channel: idx % n_cap,
                                evidence: output.evidence,
                                coarse_search: output.coarse_search,
                            }
                        })
                        .collect();
                    let child_len = child.channel_samples[0].len() as u64;
                    let mut child_result = evaluate(
                        &merged_votes(&pairs),
                        &child.input_id,
                        &capture.input_id,
                        child_len,
                        capture_len,
                        &self.config.consensus,
                        "locate-consensus",
                        "2",
                    );
                    child_result.params["coarse_search"] = serde_json::to_value(
                        pairs
                            .iter()
                            .filter_map(|pair| {
                                pair.coarse_search.as_ref().map(|search| {
                                    serde_json::json!({"source_channel": pair.source_channel,
                                "capture_channel": pair.capture_channel, "search": search})
                                })
                            })
                            .collect::<Vec<_>>(),
                    )
                    .map_err(err)?;
                    let prepared_child = prepared_sources
                        .iter()
                        .flatten()
                        .find(|p| p.decoded.input_id == child.input_id)
                        .unwrap();
                    prepared_child
                        .attach(&residual_pcm, &mut child_result)
                        .map_err(err)?;
                    child_result.params["residual_parent_alignment"] =
                        serde_json::to_value(&parent_result.alignment_id).map_err(err)?;
                    let search = assemble_residual_search(
                        &parent.input_id,
                        parent_result.alignment_id.clone(),
                        &prepared,
                        child_result,
                    );
                    let dir = PathBuf::from("locate")
                        .join(&child.input_id)
                        .join(&capture.input_id);
                    let name = format!("residual_{}.json", parent.input_id);
                    let mut lineage = parent.artifact_ids.clone();
                    lineage.extend(capture.artifact_ids.iter().cloned());
                    lineage.extend(child.artifact_ids.iter().cloned());
                    lineage.push(parent_artifact.clone());
                    lineage.push(residual_pcm.record.artifact_id.clone());
                    records.push(self.persist_json(
                        &dir,
                        &name,
                        "residual",
                        &serde_json::to_value(&search).unwrap_or_default(),
                        &lineage,
                        serde_json::json!({
                            "parent_source": search.parent_source,
                            "parent_alignment": search.parent_alignment,
                            "child_source": search.child.source_input_id,
                            "child_accepted": search.child.accepted,
                            "confidence": search.confidence,
                            "residual_pcm": search.residual_pcm,
                            "channel_transform": search.channel_transform,
                        }),
                        ctx,
                    )?);
                }
            }
            dsp_inputs.release(&capture.input_id, "original");
        }
        Ok(records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_overlays_defaults() {
        let cfg: LocateConfig =
            serde_json::from_str(r#"{"consensus":{"min_agreeing_methods":3,"position_tolerance":100,"min_score":0.3,"min_peak_to_background":2.0}}"#)
                .unwrap();
        assert_eq!(cfg.consensus.min_agreeing_methods, 3);
        assert_eq!(cfg.ridge, default_ridge());
    }

    fn noise(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((s >> 11) as f64) / (1u64 << 53) as f64 * 2.0 - 1.0
            })
            .collect()
    }

    fn input(samples: Vec<f64>, rate: u32) -> DecodedInput {
        DecodedInput {
            input_id: "x".into(),
            sample_rate: rate,
            channels: 1,
            channel_samples: vec![samples],
            artifact_ids: vec![al_core::ArtifactId::new()],
        }
    }

    /// 遠方の埋め込み一致で、階層探索が直接探索と同一位置・同一
    /// スコアを返す(位置と受理の契約は直接探索のもの)。
    #[test]
    fn hierarchical_match_agrees_with_direct_on_embedded() {
        let rate = 48_000;
        let source = noise(8_192, 11);
        let mut capture = noise(300_000, 22);
        let at = 200_000;
        for (i, &s) in source[..512].iter().enumerate() {
            capture[at + i] = s * 0.8;
        }
        let (src, cap) = (input(source, rate), input(capture, rate));
        let hier = anchor_pyramid(&src, &cap).unwrap();
        let stage = LocateStage::default();
        let mut pool = CorrelationPool::new();
        let direct = stage.best_window_match(&src, &cap, 0, 512, 0..300_000, &mut pool, None);
        let hierarch =
            stage.best_window_match(&src, &cap, 0, 512, 0..300_000, &mut pool, Some(&hier));
        let (d_pos, d_score) = direct.unwrap();
        let (h_pos, h_score) = hierarch.unwrap();
        assert_eq!(d_pos, h_pos, "position diverged");
        assert!((d_score - h_score).abs() < 1e-9, "score diverged");
        assert!((h_pos as i64 - at as i64).abs() <= 8);
    }

    /// 一致なしの純ノイズでは受理判定が一致する(どちらも None)。
    #[test]
    fn hierarchical_match_rejects_pure_noise_like_direct() {
        let rate = 48_000;
        let src = input(noise(8_192, 33), rate);
        let cap = input(noise(300_000, 44), rate);
        let hier = anchor_pyramid(&src, &cap).unwrap();
        let stage = LocateStage::default();
        let mut pool = CorrelationPool::new();
        let direct = stage.best_window_match(&src, &cap, 0, 512, 0..300_000, &mut pool, None);
        let hierarch =
            stage.best_window_match(&src, &cap, 0, 512, 0..300_000, &mut pool, Some(&hier));
        assert_eq!(direct, hierarch);
    }

    /// 遠隔オフセットのソース全体を対象とした window_anchors_in で、
    /// 階層探索あり/なしのアンカー列(位置・スコア・件数)が一致する。
    #[test]
    fn window_anchors_identical_with_and_without_hierarchy() {
        let rate = 48_000;
        let source = noise(8_192, 55);
        let mut capture = noise(260_000, 66);
        let embed_at = 150_000;
        for (i, &s) in source.iter().enumerate() {
            capture[embed_at + i] = s * 0.9;
        }
        let (src, cap) = (input(source, rate), input(capture, rate));
        let hier = anchor_pyramid(&src, &cap).unwrap();
        let stage = LocateStage::default();
        let cap_n = cap.channel_samples[0].len();
        let direct = stage.window_anchors_in(&src, &cap, 0, cap_n, 0, 0, None);
        let hierarch = stage.window_anchors_in(&src, &cap, 0, cap_n, 0, 0, Some(&hier));
        assert_eq!(direct.len(), hierarch.len());
        assert!(!direct.is_empty());
        for (d, h) in direct.iter().zip(hierarch.iter()) {
            assert_eq!(d.source, h.source);
            assert_eq!(d.capture, h.capture, "capture diverged");
            assert!((d.score - h.score).abs() < 1e-9, "score diverged");
        }
        // 実位置 ~embed_at + w_off の定数オフセットを確認。
        for a in &hierarch {
            let expected = embed_at as u64 + a.source;
            assert!((a.capture as i64 - expected as i64).abs() <= 16);
        }
    }

    /// 標本背景は全曲線 median の近似に留まる(受理ゲート用の
    /// 推定として許容誤差内)。
    #[test]
    fn sampled_background_approximates_full_curve_median() {
        let template = noise(512, 77);
        let signal = noise(50_000, 88);
        let bg = sampled_ncc_background(&template, &signal, HIER_BG_SAMPLES);
        // 参照: 全ラグ corr0 の median(テストなので直接計算)。
        let max_lag = signal.len() - template.len();
        let mut vals = Vec::with_capacity(max_lag + 1);
        for lag in 0..=max_lag {
            let v = vec::corr0(&template, &signal[lag..lag + template.len()]).unwrap_or(0.0);
            vals.push(v.abs());
        }
        let full = spec::median(&vals).unwrap_or(0.0);
        assert!(
            (bg - full).abs() / full.max(1e-12) < 0.3,
            "bg={bg} full={full}"
        );
    }
}
