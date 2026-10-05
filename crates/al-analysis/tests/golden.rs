//! Golden fixture matrix for reference analysis (validation-strategy
//! spec, reference cases).
//!
//! 全ケースは `al-testkit` の決定的 fixture で合成し、実 `LocateStage`
//! で受理させてから `ReferenceAnalysisStage` を走らせ、persisted
//! `features/reference/<src>__<cap>.json` の feature record を
//! 許容誤差付きで検証する。
//!
//! ケース一覧(spec `validation-strategy.md` の reference 行列):
//! - scalar gain / band EQ / phase(all-pass) / fixed delay / jitter /
//!   SRC time-scale / polarity / channel matrix mix / low-coherence
//!   noise / clipping / comb / echo / SRC alias image /
//!   dropped / repeated / stuck blocks
//! - ピン留め失敗モード:
//!   - 非コヒーレントな PSD 比の高域リフトを正当な EQ 変化として
//!     報告しない … pinned_noncoherent_psd_lift_not_valid_eq
//!   - 低コヒーレンス時に optimal lag を水増ししない …
//!     pinned_low_coherence_no_inflated_lag

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_job::job::input;
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};
use al_locate::stage::LocateStage;
use al_testkit::expect::{Tolerance, expect_between, expect_close, expect_true};
use al_testkit::media::pcm_f64le;
use al_testkit::signal::{self, Biquad};

const RATE: u32 = 48_000;

struct Fixture {
    dir: PathBuf,
    manifest: Manifest,
    config: serde_json::Value,
    source_id: String,
    capture_id: String,
}

fn write_pcm(job_dir: &Path, rel: &str, samples: &[f64]) {
    let path = job_dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, pcm_f64le(samples)).unwrap();
}

/// 単一 source / 単一 capture の fixture。チャンネル分割済みの
/// サンプル列をインターリーブ f64le の decoded_pcm として書く。
fn fixture(
    tag: &str,
    source_channels: &[Vec<f64>],
    capture_channels: &[Vec<f64>],
    config: serde_json::Value,
) -> Fixture {
    let dir = std::env::temp_dir().join(format!("al-ref-golden-{tag}-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut src_input = input(InputRole::SourceAsset, dir.join("src.bin"));
    let mut cap_input = input(InputRole::CaptureBug, dir.join("cap.bin"));
    src_input.source_sha256 = Some("aa".repeat(32));
    cap_input.source_sha256 = Some("bb".repeat(32));
    let source_id = src_input.input_id.to_string();
    let capture_id = cap_input.input_id.to_string();
    let inputs = vec![src_input, cap_input];
    let input_ids: Vec<_> = inputs.iter().map(|i| i.input_id.clone()).collect();
    let mut manifest = Manifest::new(
        JobId::new(),
        inputs,
        config.clone(),
        &[
            StageKind::Ingest,
            StageKind::Locate,
            StageKind::CheapAnalysis,
            StageKind::ReferenceAnalysis,
        ],
    );

    for (idx, channels) in [source_channels, capture_channels].into_iter().enumerate() {
        let rel = format!("inputs/{idx}.f64le");
        write_pcm(&dir, &rel, &signal::interleave(channels));
        let declared = manifest.input_artifact(&input_ids[idx]).unwrap();
        manifest.register_artifact(ArtifactRecord {
            artifact_id: al_core::ArtifactId::new(),
            role: "decoded_pcm".into(),
            path: PathBuf::from(&rel),
            produced_by: Some(StageKind::Ingest),
            inputs: vec![declared],
            params: serde_json::json!({
                "sample_rate": RATE,
                "channels": channels.len(),
            }),
        });
    }

    Fixture {
        dir,
        manifest,
        config,
        source_id,
        capture_id,
    }
}

fn ctx<'a>(fx: &'a Fixture, registry: &'a ArtifactRegistry) -> StageContext<'a> {
    StageContext {
        manifest: &fx.manifest,
        registry,
        config: &fx.config,
    }
}

/// locate → (register) → reference analysis → persisted feature
/// records + locate result body を返す。
fn run(fx: &mut Fixture) -> (serde_json::Value, serde_json::Value) {
    let registry = ArtifactRegistry::new(&fx.dir);
    for rec in LocateStage::new().run(&ctx(fx, &registry)).expect("locate") {
        fx.manifest.register_artifact(rec);
    }
    let records = al_analysis::stage::ReferenceAnalysisStage::new()
        .run(&ctx(fx, &registry))
        .expect("reference stage");
    assert!(
        records.iter().any(|r| r.role == "reference_features"),
        "no reference_features artifact"
    );

    let features: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            fx.dir
                .join("analysis")
                .join("features")
                .join("reference")
                .join(format!("{}__{}.json", fx.source_id, fx.capture_id)),
        )
        .expect("feature records persisted"),
    )
    .expect("feature records parse");
    let locate_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            fx.dir
                .join("locate")
                .join(&fx.source_id)
                .join(&fx.capture_id)
                .join("result.json"),
        )
        .expect("locate result.json"),
    )
    .expect("locate result parses");
    (features, locate_body)
}

fn record<'a>(body: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    body["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["feature_id"] == id)
        .unwrap_or_else(|| panic!("missing record {id}"))
}

fn records_with<'a>(body: &'a serde_json::Value, id: &str) -> Vec<&'a serde_json::Value> {
    body["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["feature_id"] == id)
        .collect()
}

fn scalar(r: &serde_json::Value) -> f64 {
    r["value"]["scalar"].as_f64().expect("scalar value")
}

/// ベッドノイズ上に `content` を `embed_at` へ埋め込んだ capture。
fn capture_with(content: &[f64], embed_at: usize, bed_seed: u64) -> Vec<f64> {
    let mut cap = signal::noise(embed_at + content.len() + 2_048, bed_seed, 0.02);
    signal::embed(&mut cap, content, embed_at, 1.0);
    cap
}

fn mono_fixture(
    tag: &str,
    source: Vec<f64>,
    capture: Vec<f64>,
    config: serde_json::Value,
) -> Fixture {
    fixture(tag, &[source], &[capture], config)
}

// ==== 既知のスカラーゲイン ====
#[test]
fn case_scalar_gain() {
    let source = signal::noise(8_192, 7, 1.0);
    let cap_body = signal::apply_gain(&source, 0.45);
    let capture = capture_with(&cap_body, 4_000, 11);
    let mut fx = mono_fixture("gain", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    expect_close(
        "ref.gain_ls",
        0.45,
        scalar(record(&body, "ref.gain_ls")),
        Tolerance::abs_rel(0.03, 0.1),
    );
    expect_close(
        "ref.rms_ratio",
        0.45,
        scalar(record(&body, "ref.rms_ratio")),
        Tolerance::abs_rel(0.03, 0.1),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 帯域別 EQ ====
#[test]
fn case_band_eq() {
    let source = signal::noise(8_192, 13, 1.0);
    // 4 kHz ピーキング +9 dB
    let eq = Biquad::peaking(RATE, 4_000.0, 9.0, 1.4);
    let cap_body = eq.apply(&source);
    let capture = capture_with(&cap_body, 4_000, 17);
    let mut fx = mono_fixture("band-eq", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);

    let bands = records_with(&body, "ref.band_gain_m2");
    assert!(!bands.is_empty());
    // 4 kHz を含む band: ゲインが持ち上がる
    let boost = bands
        .iter()
        .find(|r| {
            r["freq_range"]["start"].as_f64().unwrap_or(0.0) < 4_000.0
                && r["freq_range"]["end"].as_f64().unwrap_or(0.0) >= 4_000.0
        })
        .expect("band containing 4 kHz");
    expect_between("boosted band gain", 1.3, 6.0, scalar(boost));
    // 遠い帯域(200 Hz 前後)はほぼ 1.0
    let flat = bands
        .iter()
        .find(|r| {
            r["freq_range"]["start"].as_f64().unwrap_or(0.0) < 200.0
                && r["freq_range"]["end"].as_f64().unwrap_or(0.0) >= 200.0
        })
        .expect("band containing 200 Hz");
    expect_close(
        "flat band gain",
        1.0,
        scalar(flat),
        Tolerance::abs_rel(0.15, 0.2),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 位相 / all-pass ====
#[test]
fn case_phase_allpass() {
    let source = signal::noise(8_192, 19, 1.0);
    // 1次 all-pass 2段: 位相だけが回る。エネルギーと帯域ゲインは不変。
    let ap = Biquad::allpass(0.6);
    let cap_body = ap.apply(&ap.apply(&source));
    let capture = capture_with(&cap_body, 4_000, 23);
    let mut fx = mono_fixture("allpass", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);

    // レベルは不変
    expect_close(
        "ref.rms_ratio",
        1.0,
        scalar(record(&body, "ref.rms_ratio")),
        Tolerance::abs_rel(0.1, 0.15),
    );
    // 帯域ゲイン中央値もほぼ 1.0
    let bands = records_with(&body, "ref.band_gain_m2");
    let mut gs: Vec<f64> = bands
        .iter()
        .map(|r| scalar(r))
        .filter(|g| g.is_finite())
        .collect();
    gs.sort_by(|a, b| a.total_cmp(b));
    let median = gs[gs.len() / 2];
    expect_close(
        "median band gain",
        1.0,
        median,
        Tolerance::abs_rel(0.2, 0.25),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 固定遅延 ====
#[test]
fn case_fixed_delay() {
    // 定数遅延は locate の capture 位置として観測され、
    // 整列後の残差 lag には残らない(二重計上しない)。
    let delay = 120usize;
    let embed_at = 4_000usize;
    let source = signal::noise(16_384, 29, 1.0);
    let cap_body = signal::delay(&source, delay);
    let capture = capture_with(&cap_body, embed_at, 31);
    let mut fx = mono_fixture("fixed-delay", source, capture, serde_json::json!({}));
    let (body, locate) = run(&mut fx);
    // 遅延は位置オフセットとして回収される
    let pos = locate["result"]["capture_range"]["start"]
        .as_u64()
        .expect("capture_range.start");
    al_testkit::expect::expect_close_u64("delayed position", (embed_at + delay) as u64, pos, 64);
    // 整列後の残差 lag は 0
    expect_close(
        "ref.lag_median",
        0.0,
        scalar(record(&body, "ref.lag_median")),
        Tolerance::abs(8.0),
    );
    expect_true(
        "similarity stays high",
        scalar(record(&body, "ref.similarity_fullband")) > 0.9,
        format!(
            "similarity {}",
            scalar(record(&body, "ref.similarity_fullband"))
        ),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ジッタ遅延 ====
#[test]
fn case_jitter_delay() {
    // 帯域制限してブロック単位ジッタの相関を確保する。
    // ベース遅延は locate の位置に吸収され、整列後は
    // ジッタのばらつきだけが残る。
    let raw = signal::noise(16_384, 37, 1.0);
    let source = Biquad::lowpass(RATE, 4_000.0, std::f64::consts::FRAC_1_SQRT_2).apply(&raw);
    let cap_body = signal::jitter_delay(&source, 60.0, 25.0, 1_024, 41);
    let capture = capture_with(&cap_body, 4_000, 43);
    let mut fx = mono_fixture("jitter", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    expect_between(
        "ref.lag_median",
        -30.0,
        60.0,
        scalar(record(&body, "ref.lag_median")),
    );
    // ジッタのばらつきが MAD/分散として観測される
    expect_true(
        "jitter spread visible",
        scalar(record(&body, "ref.lag_mad")) > 2.0
            || scalar(record(&body, "ref.lag_p95")) - scalar(record(&body, "ref.lag_p05")) > 8.0,
        format!(
            "mad={} p05={} p95={}",
            scalar(record(&body, "ref.lag_mad")),
            scalar(record(&body, "ref.lag_p05")),
            scalar(record(&body, "ref.lag_p95"))
        ),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== サンプルレート不一致 / 時間伸縮ドリフト ====
#[test]
fn case_src_time_scale() {
    // 低速再生(1.002 倍速)相当: cap[i] = src[i * 1.002]
    let ratio = 1.002f64;
    let raw = signal::noise(16_384, 47, 1.0);
    let source = Biquad::lowpass(RATE, 500.0, std::f64::consts::FRAC_1_SQRT_2).apply(&raw);
    let cap_body = signal::resample_linear(&source, ratio);
    let capture = capture_with(&cap_body, 4_000, 53);
    let mut fx = mono_fixture("src-scale", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    // cap[i] = src[i * ratio] → 窓開始位置 s で lag ≈ -(ratio-1)*s
    // → 1 ウィンドウあたり -(ratio-1)*lag_hop の負のドリフト。
    // (locate の整列点は伸長内の任意点に落ちるので median ではなく
    //  slope を pin する)
    let drift = scalar(record(&body, "ref.lag_drift"));
    expect_close(
        "ref.lag_drift",
        -(ratio - 1.0) * 4_096.0,
        drift,
        Tolerance::abs(6.0),
    );
    let spread = scalar(record(&body, "ref.lag_p95")) - scalar(record(&body, "ref.lag_p05"));
    expect_true(
        "lag spread grows along window",
        spread > 8.0,
        format!("lag spread {spread}"),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 極性反転 ====
#[test]
fn case_polarity_inversion() {
    let source = signal::noise(8_192, 59, 1.0);
    let cap_body = signal::invert(&source);
    let capture = capture_with(&cap_body, 4_000, 61);
    let mut fx = mono_fixture("polarity", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    expect_close(
        "ref.polarity",
        -1.0,
        scalar(record(&body, "ref.polarity")),
        Tolerance::abs(0.01),
    );
    expect_close(
        "ref.gain_ls",
        -1.0,
        scalar(record(&body, "ref.gain_ls")),
        Tolerance::abs_rel(0.1, 0.15),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== チャンネル入れ替え / 行列ミックス ====
#[test]
fn case_channel_swap_matrix() {
    let ch_l = signal::noise(8_192, 67, 1.0);
    let ch_r = signal::noise(8_192, 71, 1.0);
    // キャプチャは [R, L] に入れ替え
    let mut cap_l = signal::noise(16_384, 73, 0.02);
    let mut cap_r = signal::noise(16_384, 79, 0.02);
    signal::embed(&mut cap_l, &ch_r, 4_000, 1.0);
    signal::embed(&mut cap_r, &ch_l, 4_000, 1.0);
    let mut fx = fixture(
        "ch-swap",
        &[ch_l, ch_r],
        &[cap_l, cap_r],
        serde_json::json!({}),
    );
    let (body, _) = run(&mut fx);
    let mix = record(&body, "ref.mixer_fit");
    let m = &mix["params"]["matrix"];
    // matrix[cap_ch][src_ch]: L = src R, R = src L
    expect_close(
        "mix[0][0]",
        0.0,
        m[0][0].as_f64().unwrap(),
        Tolerance::abs(0.15),
    );
    expect_close(
        "mix[0][1]",
        1.0,
        m[0][1].as_f64().unwrap(),
        Tolerance::abs(0.15),
    );
    expect_close(
        "mix[1][0]",
        1.0,
        m[1][0].as_f64().unwrap(),
        Tolerance::abs(0.15),
    );
    expect_close(
        "mix[1][1]",
        0.0,
        m[1][1].as_f64().unwrap(),
        Tolerance::abs(0.15),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 低コヒーレンス加算ノイズ ====
#[test]
fn case_added_noise() {
    let source = signal::noise(8_192, 83, 1.0);
    let mut cap_body = signal::noise(source.len(), 89, 0.5);
    for (i, v) in cap_body.iter_mut().enumerate() {
        *v += source[i];
    }
    let capture = capture_with(&cap_body, 4_000, 97);
    let mut fx = mono_fixture("added-noise", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    // 加算ノイズで類似度は落ちるが崩壊しない: 1/sqrt(1+0.25) ≈ 0.89
    expect_between(
        "ref.similarity_fullband",
        0.7,
        0.97,
        scalar(record(&body, "ref.similarity_fullband")),
    );
    // LS ゲインは加算ノイズに無頓着(≈1)
    expect_close(
        "ref.gain_ls",
        1.0,
        scalar(record(&body, "ref.gain_ls")),
        Tolerance::abs_rel(0.1, 0.15),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== クリッピング / リミッタ ====
#[test]
fn case_clipping() {
    let source = signal::noise(8_192, 101, 1.0);
    let cap_body = signal::clip(&signal::apply_gain(&source, 2.0), 0.8);
    let capture = capture_with(&cap_body, 4_000, 103);
    let mut fx = mono_fixture("clip", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    // 高相関だが完全ではない(クリップの非線形性)
    expect_between(
        "ref.similarity_fullband",
        0.5,
        0.98,
        scalar(record(&body, "ref.similarity_fullband")),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 既知遅延のコムフィルタ ====
#[test]
fn case_comb_filter() {
    let source = signal::noise(8_192, 107, 1.0);
    let cap_body = signal::comb(&source, 64, 0.6);
    let capture = capture_with(&cap_body, 4_000, 109);
    let mut fx = mono_fixture("comb", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    // 直接波が支配的 → lag ≈ 0
    expect_close(
        "ref.lag_median",
        0.0,
        scalar(record(&body, "ref.lag_median")),
        Tolerance::abs(8.0),
    );
    // コムリップルで帯域ゲインにばらつきが出る
    let bands = records_with(&body, "ref.band_gain_m2");
    let gs: Vec<f64> = bands
        .iter()
        .map(|r| scalar(r))
        .filter(|g| g.is_finite())
        .collect();
    let (lo, hi) = (
        gs.iter().cloned().fold(f64::MAX, f64::min),
        gs.iter().cloned().fold(f64::MIN, f64::max),
    );
    expect_true(
        "comb ripple in band gains",
        hi / lo.max(1e-9) > 1.15,
        format!("band gains {lo}..{hi}"),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 既知遅延のエコー ====
#[test]
fn case_echo() {
    let source = signal::noise(8_192, 113, 1.0);
    let cap_body = signal::echo(&source, 200, 0.5);
    let capture = capture_with(&cap_body, 4_000, 127);
    let mut fx = mono_fixture("echo", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    // 直接波が支配的 → lag ≈ 0、類似度は 1/sqrt(1+g^2) ≈ 0.89
    expect_close(
        "ref.lag_median",
        0.0,
        scalar(record(&body, "ref.lag_median")),
        Tolerance::abs(8.0),
    );
    expect_between(
        "ref.similarity_fullband",
        0.7,
        0.97,
        scalar(record(&body, "ref.similarity_fullband")),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 意図的な SRC エイリアス / イメージ ====
#[test]
fn case_src_alias_image() {
    // 2x デシメート → ゼロ詰めアップサンプル = 再構成フィルタ無しの
    // SRC。帯域制限したソース(≤6 kHz)のベースバンドはコヒーレントに
    // 残り、18 kHz 以上に折り返しイメージが載る。イメージは位相不一致
    // なので高域 coherence が落ち、PSD 比のリフトはフラグが立つはず。
    // (Welch 平均回数を稼ぐため stft を小窓にする)
    let raw = signal::noise(16_384, 131, 1.0);
    let lp = Biquad::lowpass(RATE, 6_000.0, std::f64::consts::FRAC_1_SQRT_2);
    let source = lp.apply(&lp.apply(&lp.apply(&lp.apply(&raw))));
    let dec = signal::resample_linear(&source, 2.0);
    let cap_body = signal::zero_stuff(&dec, 2); // 16384 サンプル
    let capture = capture_with(&cap_body, 4_000, 137);
    let mut fx = mono_fixture(
        "src-alias",
        source,
        capture,
        serde_json::json!({
            "analysis": { "reference": { "stft_window": 256, "stft_hop": 128 } }
        }),
    );
    let (body, locate) = run(&mut fx);
    assert_eq!(
        locate["result"]["accepted"], true,
        "locate should accept: {}",
        locate["result"]
    );
    // 高域(≥8 kHz)はイメージまたは空 — いずれも「妥当な EQ 変化」
    // として報告されない(low_coherence / insufficient_*)。
    let hf: Vec<_> = records_with(&body, "ref.band_gain_m2")
        .into_iter()
        .filter(|r| r["freq_range"]["start"].as_f64().unwrap_or(0.0) >= 8_000.0)
        .collect();
    assert!(!hf.is_empty(), "no HF band records");
    for r in &hf {
        expect_true(
            "HF image flagged non-coherent",
            r["validity"] != "valid",
            format!("band {:?} reported {:?}", r["freq_range"], r["validity"]),
        );
    }
    // ベースバンド(1 kHz)はコヒーレントに残り valid のまま
    let mid = records_with(&body, "ref.band_gain_m2")
        .into_iter()
        .find(|r| {
            r["freq_range"]["start"].as_f64().unwrap_or(0.0) < 1_000.0
                && r["freq_range"]["end"].as_f64().unwrap_or(0.0) >= 1_000.0
        })
        .expect("band containing 1 kHz");
    assert_eq!(mid["validity"], "valid", "1k band: {mid}");
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ドロップしたブロック(バッファアンダーラン = 無音化)====
#[test]
fn case_dropped_block_silence() {
    let source = signal::noise(16_384, 139, 1.0);
    // バッファ欠落が無音として出る形: 時間軸は保たれる
    let cap_body = signal::zero_block(&source, 6_144, 256);
    let capture = capture_with(&cap_body, 4_000, 149);
    let mut fx = mono_fixture("drop-silence", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    // タイムラインにジャンプなし → lag はフラット、step なし
    expect_close(
        "ref.lag_median",
        0.0,
        scalar(record(&body, "ref.lag_median")),
        Tolerance::abs(8.0),
    );
    expect_close(
        "ref.lag_step_candidates",
        0.0,
        scalar(record(&body, "ref.lag_step_candidates")),
        Tolerance::abs(0.5),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ドロップしたブロック(内容が本当に欠落 = lag ステップ)====
#[test]
fn case_dropped_block_step() {
    let source = signal::noise(16_384, 151, 1.0);
    // src[..6144] + src[6400..] — 256 サンプル欠落で後続が早まる
    let cap_body = signal::drop_block(&source, 6_144, 256);
    let capture = capture_with(&cap_body, 4_000, 157);
    let mut fx = mono_fixture(
        "drop-step",
        source,
        capture,
        serde_json::json!({ "analysis": { "reference": { "lag_window": 2048 } } }),
    );
    let (body, _) = run(&mut fx);
    // lag は 0 → -256 へステップする → p05 と p95 が大きく開く
    let p05 = scalar(record(&body, "ref.lag_p05"));
    let p95 = scalar(record(&body, "ref.lag_p95"));
    expect_true(
        "lag step visible",
        p95 - p05 > 128.0,
        format!("lag p05={p05} p95={p95}"),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== バッファ反復 ====
#[test]
fn case_repeated_buffer() {
    let source = signal::noise(16_384, 163, 1.0);
    // src[..4096] + src[3840..4096] + src[4096..] → 後続が +256 遅れる
    let cap_body = signal::repeat_block(&source, 3_840, 256);
    let capture = capture_with(&cap_body, 4_000, 167);
    let mut fx = mono_fixture(
        "repeat-buf",
        source,
        capture,
        serde_json::json!({ "analysis": { "reference": { "lag_window": 2048 } } }),
    );
    let (body, _) = run(&mut fx);
    let p05 = scalar(record(&body, "ref.lag_p05"));
    let p95 = scalar(record(&body, "ref.lag_p95"));
    expect_true(
        "lag step visible",
        p95 - p05 > 128.0,
        format!("lag p05={p05} p95={p95}"),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== スタックサンプル ====
#[test]
fn case_stuck_samples() {
    let source = signal::noise(16_384, 173, 1.0);
    // バッファ張り付き: その場で値が保持される(時間軸は保つ)
    let cap_body = signal::stuck_block(&source, 6_144, 512);
    let capture = capture_with(&cap_body, 4_000, 179);
    let mut fx = mono_fixture("stuck", source, capture, serde_json::json!({}));
    let (body, _) = run(&mut fx);
    expect_close(
        "ref.lag_median",
        0.0,
        scalar(record(&body, "ref.lag_median")),
        Tolerance::abs(8.0),
    );
    expect_close(
        "ref.lag_step_candidates",
        0.0,
        scalar(record(&body, "ref.lag_step_candidates")),
        Tolerance::abs(0.5),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ピン留め: 非コヒーレント PSD 比の高域リフト ====
#[test]
fn pinned_noncoherent_psd_lift_not_valid_eq() {
    // 低域はコヒーレント、8 kHz 以上に無相関ノイズを載せる。
    // PSD 比だけ見ると高域が持ち上がるが、コヒーレンスが無いので
    // 「妥当な EQ 変化」として報告されてはならない。
    let raw = signal::noise(16_384, 181, 1.0);
    let source = raw.clone();
    // 急峻な LP(6 kHz, x4 段)でコヒーレントな高域を消し、
    // 無相関ノイズを載せる
    let lp = Biquad::lowpass(RATE, 6_000.0, std::f64::consts::FRAC_1_SQRT_2);
    let mut low = lp.apply(&lp.apply(&lp.apply(&lp.apply(&raw))));
    let hiss = Biquad::highpass(RATE, 8_000.0, std::f64::consts::FRAC_1_SQRT_2)
        .apply(&signal::noise(16_384, 191, 1.0));
    for (i, v) in low.iter_mut().enumerate() {
        *v += 0.4 * hiss[i];
    }
    let cap_body = low;
    let capture = capture_with(&cap_body, 4_000, 193);
    // Welch 平均回数を稼ぐため stft を小窓にする
    // (既定 4096/2048 では ~7 frame で非相関でも coherence ≈ 0.4
    //  に偏り、フラグが立たない — 十分な平均下での pin とする)
    let mut fx = mono_fixture(
        "pin-psd-lift",
        source,
        capture,
        serde_json::json!({
            "analysis": { "reference": { "stft_window": 512, "stft_hop": 256 } }
        }),
    );
    let (body, locate) = run(&mut fx);
    // コヒーレントな低域で locate は受理される前提で pin する
    assert_eq!(
        locate["result"]["accepted"], true,
        "locate should accept: {}",
        locate["result"]
    );
    let hf: Vec<_> = records_with(&body, "ref.band_gain_m2")
        .into_iter()
        .filter(|r| r["freq_range"]["start"].as_f64().unwrap_or(0.0) >= 8_000.0)
        .collect();
    assert!(!hf.is_empty(), "no HF band records");
    for r in &hf {
        expect_true(
            "HF lift flagged non-coherent",
            r["validity"] != "valid",
            format!(
                "band {:?} gain {} reported {:?}",
                r["freq_range"],
                scalar(r),
                r["validity"]
            ),
        );
    }
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ピン留め: 低コヒーレンス時の optimal-lag 水増し ====
#[test]
fn pinned_low_coherence_no_inflated_lag() {
    // 埋め込みが相関的に埋もれるキャプチャ: locate が拒否し、
    // reference analysis は「自信ありの lag」を捏造しない。
    let source = signal::noise(8_192, 197, 1.0);
    let mut capture = signal::noise(24_576, 199, 1.0);
    signal::embed(&mut capture, &source, 8_000, 0.03);
    let mut fx = mono_fixture("pin-lag", source, capture, serde_json::json!({}));
    let (body, locate) = run(&mut fx);
    assert_eq!(
        locate["result"]["accepted"], false,
        "locate should reject buried content: {}",
        locate["result"]
    );
    // upstream invalid → lag/gain 等は有効な計測として出ない
    let lag = record(&body, "ref.lag_median");
    expect_true(
        "lag not reported valid",
        lag["validity"] != "valid",
        format!("lag_median validity {}", lag["validity"]),
    );
    fs::remove_dir_all(&fx.dir).ok();
}
