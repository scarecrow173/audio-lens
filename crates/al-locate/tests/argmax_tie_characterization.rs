//! T1 特性計測(TASK-034Fo9cS08WsbNrLoq6t6a / design §7 / test-spec
//! `locate-argmax-tie-eps-calibration.md` Rule A):
//! fixture 毎・メソッド毎の raw 相関曲線に対し
//! `M − max_{i≠argmax}|corr[i]|`(rank-1 の大きさと全サンプル中の
//! 第二順位大きさの差 — argmax ピーク自身のショルダーを含む)を
//! 実測・記録し、eps 候補窓での同点集合 |T| の大きさを列挙する。
//!
//! 本テストは計測専用で本番挙動を変えない。曲線生成は production
//! 経路と同一の式を使う:
//!
//! - `full_ncc` / `gcc_phat`: `dsp::ncc` / `dsp::gcc_phat`(
//!   `signal.len() - template.len()` の max_lag)
//! - `band_ncc`: `InputCache::prepare` 経由の prepared bands
//!   (`locate_prepared` と同一の帯域信号)+ `evidence_from_bands`
//!   と同じ累積(テンプレート帯域エネルギー `MIN_ENERGY` フィルタ)
//! - `spectral_envelope`: `methods.rs` `SpectralEnvelope::locate` の
//!   現行逐次式(pre-T3、`methods.rs:539-547`)を複製 — `dsp::corr0`
//!   置換後とは浮動小数点丸めが異なり得るため逐語で再現する
//!
//! `cargo test -p al-locate --test argmax_tie_characterization --
//! --nocapture` で計測表を出力する。

use al_locate::dsp;
use al_locate::methods::{SpectralEnvelope, normalize_gain, peaks_from_correlation};
use al_locate::prepared::InputCache;
use al_testkit::signal::{self, Biquad};

/// `methods.rs` と同じ閾値(テンプレート帯域エネルギー・preflight)。
const MIN_ENERGY: f64 = 1e-9;
/// `methods.rs` の `MAX_CANDIDATES`。
const MAX_CANDIDATES: usize = 8;
/// `methods.rs` の `MIN_PEAK_TO_BACKGROUND`。
const MIN_PEAK_TO_BACKGROUND: f64 = 2.0;
/// 計測で列挙する `TIE_REL_EPS` 候補(`abs_floor` は 1e-9 固定で
/// `window = max(rel * M, floor)`)。
const REL_EPS_CANDIDATES: &[f64] = &[1e-9, 1e-8, 1e-7, 1e-6];
const ABS_FLOOR: f64 = 1e-9;

/// T1 計測で確定した eps 値(T2 が `methods.rs` へ mirror する
/// `TIE_REL_EPS`/`TIE_ABS_FLOOR` の較正結果)。
///
/// 実測まとめ:
/// - al-math scalar↔dispatch 偏差上限: `Correlation` n=65536 で
///   ビット一致(0.0)、n=32768 で ≤3.5e-18、`Envelope` 実運用
///   fft_size で ≤4.5e-16 → 曲線レベルでも ~1e-15 級まで
/// - 受理系 fixture の最小マージン `M − max_{i≠argmax}|corr[i]|`:
///   `case_loop_material` の `spectral_envelope` で 1.65e-8
///   (ループ素材の正当な複数出現が準同点包絡を持つ)。
///   `full_ncc` 同 fixture で 5.52e-7、`case_repeated_source` で
///   1.75e-6
/// - 採用値 1e-11 は偏差(~1e-15)から ~4 桁上、最小受理マージン
///   (1.65e-8)から ~1.7e3 倍下にあり、両側の「<<」を満たす。
///   spec 候補レンジ(1e-9..1e-6)の下端 1e-9 は最小マージンに対し
///   16倍しか離れず薄弱なため、実測に基づきレンジ外を採用した。
const TIE_REL_EPS: f64 = 1e-11;
const TIE_ABS_FLOOR: f64 = 1e-11;

struct Fixture {
    name: &'static str,
    template: Vec<f64>,
    signal: Vec<f64>,
    rate: u32,
    /// 受理系なら true(マージンが eps 窓の実質上限を拘束する側)。
    acceptance: bool,
}

/// production `preflight`(`methods.rs:157`)の再現。
/// 棄却された場合は reason を返す。
fn preflight_reject(template: &[f64], signal: &[f64]) -> Option<&'static str> {
    if template.is_empty() || signal.len() < template.len() {
        return Some("signal shorter than template");
    }
    if dsp::sum_sq(template) < MIN_ENERGY {
        return Some("template has no usable energy");
    }
    if dsp::sum_sq(signal) < MIN_ENERGY {
        return Some("signal has no usable energy");
    }
    None
}

/// production `evidence_from_bands`(`methods.rs:351`)の累積部を
/// prepared bands で再現。全帯域が非活性なら None(production の
/// `OutOfSupportedFrequencyRange` 棄却相当)。
fn band_ncc_curve(
    template: &[f64],
    signal: &[f64],
    rate: u32,
    cache: &mut InputCache,
) -> Option<Vec<f64>> {
    let t = cache
        .prepare("measure-src", 0, rate, "v1", template)
        .unwrap();
    let s = cache.prepare("measure-cap", 0, rate, "v1", signal).unwrap();
    let mut acc = vec![0.0; signal.len() - template.len() + 1];
    let mut bands_used = 0usize;
    for ((band_t, t_band), (band_s, s_band)) in t.bands().iter().zip(s.bands()) {
        assert_eq!(band_t, band_s);
        if dsp::sum_sq(t_band) < MIN_ENERGY {
            continue;
        }
        for (a, c) in acc
            .iter_mut()
            .zip(dsp::ncc(t_band, s_band, s_band.len() - t_band.len()))
        {
            *a += c;
        }
        bands_used += 1;
    }
    (bands_used > 0).then_some(acc)
}

/// `SpectralEnvelope::locate` の現行逐次式(pre-T3)を逐語再現した
/// hop 疎化曲線。`methods.rs:524-548` と同一の式・同一の hop 規則。
fn spectral_envelope_curve(template: &[f64], signal: &[f64]) -> Vec<f64> {
    let method = SpectralEnvelope::default();
    let t_env = dsp::spectral_envelope(&normalize_gain(template), method.fft_size, method.bins);
    const MAX_WINDOWS: usize = 2048;
    let win = template.len();
    let max_lag = signal.len() - win;
    let hop = (win / 16).max(1).max(max_lag / MAX_WINDOWS);
    let mut corr = vec![0.0; max_lag + 1];
    for lag in (0..=max_lag).step_by(hop) {
        let s_env = dsp::spectral_envelope(
            &normalize_gain(&signal[lag..lag + win]),
            method.fft_size,
            method.bins,
        );
        // pre-T3 逐次式(corr0 置換前の丸め挙動を保持するため複製)。
        let dot: f64 = t_env.iter().zip(&s_env).map(|(a, b)| a * b).sum();
        let tn: f64 = t_env.iter().map(|v| v * v).sum::<f64>().sqrt();
        let sn: f64 = s_env.iter().map(|v| v * v).sum::<f64>().sqrt();
        corr[lag] = if tn * sn > 0.0 { dot / (tn * sn) } else { 0.0 };
    }
    corr
}

struct Margin {
    m: f64,
    argmax: usize,
    second: f64,
    second_at: usize,
    /// `M − max_{i≠argmax}|corr[i]|`(T1 の拘束マージン)。
    delta: f64,
    /// argmax 隣接サンプルの delta(ショルダー曝露)。
    shoulder_left: f64,
    shoulder_right: f64,
    /// `|corr[i]|` 降順の上位 (index, 値)。
    top: Vec<(usize, f64)>,
    /// 候補 eps 窓毎の同点集合 |T|。
    ties: Vec<(f64, f64, usize)>,
    /// 採用 eps での `tie_window` と |T|。
    chosen_window: f64,
    chosen_tie_count: usize,
    /// 採用 eps で T に入る index。
    tied_positions: Vec<usize>,
    /// production の `peaks_from_correlation` が top 候補を返し、
    /// 曖昧判定に到達する(Valid 相当の曲線)か。
    reaches_tie_eval: bool,
}

fn measure(corr: &[f64]) -> Margin {
    let mut indexed: Vec<(usize, f64)> =
        corr.iter().enumerate().map(|(i, v)| (i, v.abs())).collect();
    indexed.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (argmax, m) = indexed[0];
    let (second_at, second) = indexed.get(1).copied().unwrap_or((usize::MAX, 0.0));
    let shoulder_left = if argmax > 0 {
        m - corr[argmax - 1].abs()
    } else {
        f64::NAN
    };
    let shoulder_right = if argmax + 1 < corr.len() {
        m - corr[argmax + 1].abs()
    } else {
        f64::NAN
    };
    let ties = REL_EPS_CANDIDATES
        .iter()
        .map(|&rel| {
            let window = (rel * m).max(ABS_FLOOR);
            let count = corr.iter().filter(|v| v.abs() >= m - window).count();
            (rel, window, count)
        })
        .collect();
    let chosen_window = (TIE_REL_EPS * m).max(TIE_ABS_FLOOR);
    let tied_positions: Vec<usize> = corr
        .iter()
        .enumerate()
        .filter(|(_, v)| v.abs() >= m - chosen_window)
        .map(|(i, _)| i)
        .collect();
    let candidates = peaks_from_correlation(corr, MAX_CANDIDATES);
    let reaches_tie_eval = candidates.first().is_some_and(|top| {
        top.score.is_finite()
            && top.score.abs() > 1e-12
            && (corr.len() == 1 || top.peak_to_background >= MIN_PEAK_TO_BACKGROUND)
    });
    Margin {
        m,
        argmax,
        second,
        second_at,
        delta: m - second,
        shoulder_left,
        shoulder_right,
        top: indexed.into_iter().take(6).collect(),
        ties,
        chosen_window,
        chosen_tie_count: tied_positions.len(),
        tied_positions,
        reaches_tie_eval,
    }
}

fn report(fixture: &str, method: &str, corr: &[f64]) -> Margin {
    let r = measure(corr);
    eprintln!(
        "[{fixture}] {method:17} len={:>6} M={:.6e} argmax={:>6} second={:.6e}@{:>6} \
         margin={:.6e} (rel {:.3e}) shoulder=(-1:{:+.3e},+1:{:+.3e}) valid-path={}",
        corr.len(),
        r.m,
        r.argmax,
        r.second,
        r.second_at,
        r.delta,
        r.delta / r.m.max(f64::MIN_POSITIVE),
        r.shoulder_left,
        r.shoulder_right,
        r.reaches_tie_eval,
    );
    eprint!("[{fixture}] {method:17} ties:");
    for (rel, window, count) in &r.ties {
        eprint!(" rel={rel:.0e}/w={window:.2e}->|T|={count}");
    }
    eprintln!(
        " | chosen(w={:.2e})->|T|={}",
        r.chosen_window, r.chosen_tie_count
    );
    eprint!("[{fixture}] {method:17} top|corr|:");
    for (i, v) in &r.top {
        eprint!(" {i}:{v:.6e}");
    }
    eprintln!();
    r
}

fn run_fixture(fx: &Fixture, cache: &mut InputCache) -> [(&'static str, Option<Margin>); 4] {
    if let Some(reason) = preflight_reject(&fx.template, &fx.signal) {
        eprintln!("[{}] preflight-reject: {reason}", fx.name);
        return [
            ("full_ncc", None),
            ("band_ncc", None),
            ("gcc_phat", None),
            ("spectral_envelope", None),
        ];
    }
    let max_lag = fx.signal.len() - fx.template.len();
    let full = dsp::ncc(&fx.template, &fx.signal, max_lag);
    let band = band_ncc_curve(&fx.template, &fx.signal, fx.rate, cache);
    let gcc = dsp::gcc_phat(&fx.template, &fx.signal, max_lag);
    let env = spectral_envelope_curve(&fx.template, &fx.signal);
    [
        ("full_ncc", Some(report(fx.name, "full_ncc", &full))),
        (
            "band_ncc",
            band.as_ref().map(|c| report(fx.name, "band_ncc", c)),
        ),
        ("gcc_phat", Some(report(fx.name, "gcc_phat", &gcc))),
        (
            "spectral_envelope",
            Some(report(fx.name, "spectral_envelope", &env)),
        ),
    ]
}

/// `tests/locate.rs` / `tests/prepared.rs` と同一の LCG ノイズ
/// (al-testkit `signal::noise` の amp=1.0 と同じ系列)。
fn noise(len: usize, seed: u64) -> Vec<f64> {
    signal::noise(len, seed, 1.0)
}

fn embedded(
    bed_seed: u64,
    capture_len: usize,
    source: &[f64],
    offset: usize,
    gain: f64,
) -> Vec<f64> {
    let mut cap = signal::noise(capture_len, bed_seed, 0.02);
    signal::embed(&mut cap, source, offset, gain);
    cap
}

fn fixtures() -> Vec<Fixture> {
    let mut out = Vec::new();

    // ==== 受理系 ====
    // golden `pin-periodic-control`(非周期対照, `golden.rs:644-654`)
    let control = noise(4_096, 149);
    let mut control_capture = vec![0.0; 16_000];
    signal::embed(&mut control_capture, &control, 6_000, 1.0);
    out.push(Fixture {
        name: "pin-periodic-control",
        template: control,
        signal: control_capture,
        rate: 48_000,
        acceptance: true,
    });

    // golden `case_exact_insert`(代表受理, `golden.rs:219-227`)
    let source = noise(4_096, 7);
    let capture = embedded(11, 24_576, &source, 9_600, 1.0);
    out.push(Fixture {
        name: "case_exact_insert",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // golden `case_repeated_source`(`golden.rs:330-358`)
    let source = noise(2_048, 53);
    let mut capture = signal::noise(40_000, 59, 0.02);
    for &at in &[5_000usize, 18_000, 33_000] {
        signal::embed(&mut capture, &source, at, 1.0);
    }
    out.push(Fixture {
        name: "case_repeated_source",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // golden `case_loop_material`(`golden.rs:296-326`)
    let source = noise(4_000, 43);
    let mut capture = signal::noise(40_000, 47, 0.02);
    for k in 0..3 {
        signal::embed(&mut capture, &source, 8_000 + k * 4_000, 1.0);
    }
    out.push(Fixture {
        name: "case_loop_material",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // golden `case_long_source_linear_drift`(ショルダーデルタ曝露,
    // `golden.rs:393-401`: fc=1kHz/48kHz Biquad lowpass + 1.005x)
    let src = noise(8_192, 71);
    let source = Biquad::lowpass(48_000, 1_000.0, std::f64::consts::FRAC_1_SQRT_2).apply(&src);
    let stretched = signal::resample_linear(&source, 1.005);
    let mut capture = signal::noise(24_576, 73, 0.02);
    signal::embed(&mut capture, &stretched, 8_000, 1.0);
    out.push(Fixture {
        name: "case_long_source_linear_drift",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // `tests/locate.rs` `polarity_inverted_source_is_still_located`(:420)
    let source = noise(1_500, 13);
    let mut capture = noise(8_000, 17);
    for (i, &v) in source.iter().enumerate() {
        capture[3_000 + i] -= v;
    }
    out.push(Fixture {
        name: "polarity_inverted_source_is_still_located",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // `tests/locate.rs` `repeated_source_reports_second_occurrence`(:440)
    let source = noise(1_200, 23);
    let mut capture = noise(12_000, 29);
    for (i, &v) in source.iter().enumerate() {
        capture[2_000 + i] += v;
        capture[9_000 + i] += v;
    }
    out.push(Fixture {
        name: "repeated_source_reports_second_occurrence",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // `tests/locate.rs` `different_rates_locate_without_false_drift`
    // (:488)。stage は source を capture レート 48000 へ正規化する
    // (`stage.rs:578` `prepare_input(s, capture.sample_rate, ..)`)。
    let chirp = |t: f64| (std::f64::consts::TAU * (500.0 * t + 40_000.0 * t * t)).sin() * 0.7;
    let source_44k: Vec<f64> = (0..4_410).map(|i| chirp(i as f64 / 44_100.0)).collect();
    let plan = al_math::resample::ResamplePlan::new(44_100, 48_000).unwrap();
    let template = al_math::resample::resample(&source_44k, &plan).unwrap();
    let mut capture = vec![0.0; 9_600];
    for i in 0..4_800 {
        capture[2_400 + i] = chirp(i as f64 / 48_000.0);
    }
    out.push(Fixture {
        name: "different_rates_locate_without_false_drift",
        template,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // golden `case_periodic_source_locates_at_phase_consistent_position`
    // (`golden.rs:277-292`): 周期素材の単一出現は受理系 —
    // 周期オフセットの近同点が受理系側の曖昧リスクになる。
    let period = 1_024usize;
    let body_noise = noise(period, 37);
    let mut source = Vec::with_capacity(2_048);
    source.extend_from_slice(&body_noise);
    source.extend_from_slice(&body_noise);
    let capture = embedded(41, 24_576, &source, 10_000, 1.0);
    out.push(Fixture {
        name: "case_periodic_source",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: true,
    });

    // ==== 拒否系 / 混同リスク ====
    // golden `pin-periodic`(周期拒否, `golden.rs:598-617`)
    let period = 1_024usize;
    let unit = noise(period, 139);
    let mut source = Vec::with_capacity(2_048);
    source.extend_from_slice(&unit);
    source.extend_from_slice(&unit);
    let mut capture = Vec::with_capacity(30_000);
    while capture.len() < 30_000 {
        capture.extend_from_slice(&unit);
    }
    capture.truncate(30_000);
    out.push(Fixture {
        name: "pin-periodic",
        template: source,
        signal: capture,
        rate: 48_000,
        acceptance: false,
    });

    // golden `pinned_low_coherence_no_inflated_lag`(`golden.rs:659-692`)
    out.push(Fixture {
        name: "pinned_low_coherence_no_inflated_lag",
        template: noise(4_096, 149),
        signal: noise(16_384, 151),
        rate: 48_000,
        acceptance: false,
    });

    // golden `pinned_silence_no_false_peak`(`golden.rs:560-594`):
    // 両方向とも preflight で棄却され同点判定には到達しない。
    out.push(Fixture {
        name: "pinned_silence_no_false_peak/silent-source",
        template: signal::silence(4_096),
        signal: noise(16_384, 131),
        rate: 48_000,
        acceptance: false,
    });
    out.push(Fixture {
        name: "pinned_silence_no_false_peak/silent-capture",
        template: noise(4_096, 137),
        signal: signal::silence(16_384),
        rate: 48_000,
        acceptance: false,
    });

    // `tests/prepared.rs` `prepared_methods_keep_all_global_evidence_exactly`
    // (:51-98): `template` / `-template` の2出現で |corr| 厳密同点。
    let mut state = 17u64;
    let template: Vec<f64> = (0..2_049)
        .map(|_| {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (state >> 33) as f64 / (1u64 << 31) as f64 - 0.5
        })
        .collect();
    let mut sig = vec![0.0; 9_001];
    sig[..template.len()].copy_from_slice(&template);
    for (o, &v) in sig[6_952..].iter_mut().zip(&template) {
        *o = -v;
    }
    out.push(Fixture {
        name: "prepared_exact_tie(+/-template)",
        template,
        signal: sig,
        rate: 512,
        acceptance: false,
    });

    // `tests/prepared.rs`
    // `coarse_fallback_preserves_periodic_broad_and_threshold_adjacent_evidence`
    // (:101-141): 周期 sin テンプレートの同点構造。
    for rate in [512u32, 513] {
        for frequency in [0.002f64, 0.12] {
            let template: Vec<f64> = (0..2_051).map(|i| (i as f64 * frequency).sin()).collect();
            let sig: Vec<f64> = (0..7_001)
                .map(|i| (i as f64 * frequency + 0.7).sin() + 0.3 * (i as f64 * 0.713).sin())
                .collect();
            out.push(Fixture {
                name: if rate == 512 {
                    "prepared_periodic_sin"
                } else {
                    "prepared_periodic_sin/r513"
                },
                template,
                signal: sig,
                rate,
                acceptance: false,
            });
        }
    }

    out
}

/// T1 Rule A の計測本体: 全 fixture × 全メソッドの raw 曲線マージン
/// と候補 eps 窓での |T| を `--nocapture` で出力する。
///
/// アサーションは確定 eps(`TIE_REL_EPS`/`TIE_ABS_FLOOR`)に基づく
/// 構造的事実を pin する:
///
/// - 受理系 fixture: 全メソッドの曲線が曖昧判定に到達し、採用窓で
///   非曖昧(|T| == 1)。かつマージンが窓の 10 倍を上回ることで
///   `eps << margin` の余裕を pin する
/// - `pin-periodic`: `full_ncc`/`spectral_envelope` が |T| >= 2 の
///   厳密同点構造(period 倍数位置)を持つこと。実測では `band_ncc`
///   (margin ~2.6e-4)と `gcc_phat`(margin ~1.1e-1)は採用窓で
///   同点とならない — 帯域フィルタの有限区間エッジ効果と位相
///   白色化の全信号変換により厳密周期が崩れるため(混同リスクの
///   実形記録)
/// - `prepared_exact_tie(+/-template)`: `full_ncc`/`gcc_phat` の
///   |corr| 厳密同点(margin ~3e-15)が |T| == 2({0, 6952})
/// - `prepared_periodic_sin`: 周期 sin は近同点「分離ピーク」構造
///   (margin ~4e-6 級)で、採用窓では同点とならない
/// - `pinned_low_coherence`/`pinned_silence`: 前者は全メソッド
///   |T| == 1(棄却は従来の弱ピーク経路)、後者は preflight で棄却
///   され同点判定へ到達しない
#[test]
fn argmax_tie_margin_distribution() {
    let mut cache = InputCache::default();
    for fx in fixtures() {
        eprintln!("==== {} (accept={}) ====", fx.name, fx.acceptance);
        let results = run_fixture(&fx, &mut cache);
        for (method, margin) in &results {
            let Some(r) = margin else {
                continue;
            };
            // 構造的 sanity: 曲線が有限の最大値を持つこと。
            assert!(r.m.is_finite(), "{} {method}: non-finite curve", fx.name);
            assert!(r.delta >= 0.0);
            if fx.acceptance {
                assert!(
                    r.reaches_tie_eval,
                    "{} {method}: accepted fixture must reach tie evaluation",
                    fx.name
                );
                assert_eq!(
                    r.chosen_tie_count, 1,
                    "{} {method}: accepted fixture must be unambiguous at \
                     TIE_REL_EPS={TIE_REL_EPS} (margin={:.3e} vs window={:.3e})",
                    fx.name, r.delta, r.chosen_window
                );
                assert!(
                    r.delta > 10.0 * r.chosen_window,
                    "{} {method}: margin {:.3e} lacks headroom over window {:.3e}",
                    fx.name,
                    r.delta,
                    r.chosen_window
                );
            }
        }

        // 同点構造の実形 pin(拒否系 fixture)。
        let tie = |m: &(&'static str, Option<Margin>)| m.1.as_ref().map(|r| r.chosen_tie_count);
        match fx.name {
            "pin-periodic" => {
                // 厳密同点は full_ncc / spectral_envelope のみ。
                assert_eq!(tie(&results[0]), Some(28));
                assert_eq!(tie(&results[1]), Some(1)); // band_ncc
                assert_eq!(tie(&results[2]), Some(1)); // gcc_phat
                assert_eq!(tie(&results[3]), Some(219)); // spectral_envelope
                // 同点位置は周期倍数。
                for &pos in &results[0].1.as_ref().unwrap().tied_positions {
                    assert_eq!(pos % 1_024, 0, "full_ncc tied at {pos}");
                }
            }
            "prepared_exact_tie(+/-template)" => {
                let full = results[0].1.as_ref().unwrap();
                assert_eq!(full.chosen_tie_count, 2);
                assert_eq!(full.tied_positions, [0, 6_952]);
                assert_eq!(tie(&results[2]), Some(2)); // gcc_phat
            }
            name if name.starts_with("prepared_periodic_sin") => {
                // 周期 sin は採用窓では同点にならない(margin ~4e-6 の
                // 近同点分離ピーク構造)。
                for &(method, ref margin) in &results {
                    assert_eq!(
                        margin.as_ref().unwrap().chosen_tie_count,
                        1,
                        "{} {method}",
                        fx.name
                    );
                }
            }
            "pinned_low_coherence_no_inflated_lag" => {
                for &(method, ref margin) in &results {
                    assert_eq!(
                        margin.as_ref().unwrap().chosen_tie_count,
                        1,
                        "{} {method}",
                        fx.name
                    );
                }
            }
            _ => {}
        }
    }
}
