//! Golden fixture matrix for locate (validation-strategy spec, locate cases).
//!
//! 全ケースは `al-testkit` の決定的 fixture(シード固定)で生成し、
//! 実 `LocateStage` を走らせて persisted `result.json` /
//! `time_map.json` / `residual_*.json` を許容誤差付きで検証する。
//!
//! ケース一覧(spec `validation-strategy.md` の locate 行列):
//! - exact / gain / EQ-mismatch insert … case_*_insert
//! - silence boundary … case_silence_boundary
//! - strong-periodic false peak … case_periodic_source /
//!   pinned_periodic_everywhere_not_accepted
//! - loop material … case_loop_material
//! - multiple legitimate occurrences … case_repeated_source
//! - long-source constant offset … case_long_source_constant_offset
//! - long-source linear drift … case_long_source_linear_drift
//! - skip + resume … case_skip_and_resume
//! - multichannel downmix … case_multichannel_downmix
//! - dominant BGM masking … case_bgm_masks_quiet_source
//! - residual search after BGM subtraction … case_bgm_residual_search
//! - ピン留め失敗モード:
//!   - 無音での偽相関ピーク受理 … pinned_silence_no_false_peak
//!   - 強周期での偽相関ピーク受理 … pinned_periodic_*
//!   - 低コヒーレンス時の optimal-lag 過大推定 …
//!     pinned_low_coherence_no_inflated_lag

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_job::job::input;
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};
use al_locate::stage::LocateStage;
use al_testkit::expect::{Tolerance, expect_close, expect_close_u64, expect_true};
use al_testkit::media::pcm_f64le;
use al_testkit::signal::{self, Biquad};

const RATE: u32 = 48_000;

struct JobFixture {
    dir: PathBuf,
    manifest: Manifest,
    config: serde_json::Value,
    source_ids: Vec<String>,
    capture_ids: Vec<String>,
}

fn write_pcm(job_dir: &Path, rel: &str, samples: &[f64]) {
    let path = job_dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, pcm_f64le(samples)).unwrap();
}

/// `sources`/`captures` は (チャンネル分割済みサンプル列, sample rate)。
/// decoded_pcm artifact はインターリーブ済み f64le として書く。
fn fixture(
    tag: &str,
    sources: &[(&[Vec<f64>], u32)],
    captures: &[(&[Vec<f64>], u32)],
    config: serde_json::Value,
) -> JobFixture {
    let dir = std::env::temp_dir().join(format!("al-locate-golden-{tag}-{}", JobId::new()));
    fs::create_dir_all(&dir).unwrap();

    let mut inputs = Vec::new();
    for i in 0..sources.len() {
        inputs.push(input(
            InputRole::SourceAsset,
            dir.join(format!("src{i}.bin")),
        ));
    }
    for i in 0..captures.len() {
        inputs.push(input(
            InputRole::CaptureBug,
            dir.join(format!("cap{i}.bin")),
        ));
    }
    let mut manifest = Manifest::new(
        JobId::new(),
        inputs.clone(),
        config.clone(),
        &[StageKind::Ingest, StageKind::Locate],
    );

    let mut source_ids = Vec::new();
    let mut capture_ids = Vec::new();
    let mut idx = 0;
    for (id_list, entries) in [(&mut source_ids, sources), (&mut capture_ids, captures)] {
        for (channels, rate) in entries {
            let inp = &inputs[idx];
            let rel = format!("inputs/{idx}.f64le");
            write_pcm(&dir, &rel, &signal::interleave(channels));
            let declared = manifest.input_artifact(&inp.input_id).unwrap();
            manifest.register_artifact(ArtifactRecord {
                artifact_id: al_core::ArtifactId::new(),
                role: "decoded_pcm".into(),
                path: PathBuf::from(&rel),
                produced_by: Some(StageKind::Ingest),
                inputs: vec![declared],
                params: serde_json::json!({
                    "sample_rate": rate,
                    "channels": channels.len(),
                }),
            });
            id_list.push(inp.input_id.to_string());
            idx += 1;
        }
    }

    JobFixture {
        dir,
        manifest,
        config,
        source_ids,
        capture_ids,
    }
}

/// 単一 source / 単一 capture の簡易形。
fn mono_fixture(
    tag: &str,
    source: &[f64],
    capture: &[f64],
    config: serde_json::Value,
) -> JobFixture {
    fixture(
        tag,
        &[(&[source.to_vec()][..], RATE)],
        &[(&[capture.to_vec()][..], RATE)],
        config,
    )
}

fn run_locate(fx: &JobFixture) -> Vec<ArtifactRecord> {
    let registry = ArtifactRegistry::new(&fx.dir);
    let ctx = StageContext {
        manifest: &fx.manifest,
        registry: &registry,
        config: &fx.config,
    };
    LocateStage::new().run(&ctx).expect("locate stage")
}

fn read_json(fx: &JobFixture, dir_parts: &[&str], name: &str) -> serde_json::Value {
    let path = dir_parts
        .iter()
        .fold(fx.dir.clone(), |p, d| p.join(d))
        .join(name);
    serde_json::from_str(
        &fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing artifact {}", path.display())),
    )
    .unwrap()
}

fn result_json(fx: &JobFixture, src: usize, cap: usize) -> serde_json::Value {
    read_json(
        fx,
        &["locate", &fx.source_ids[src], &fx.capture_ids[cap]],
        "result.json",
    )
}

fn time_map_json(fx: &JobFixture, src: usize, cap: usize) -> serde_json::Value {
    read_json(
        fx,
        &["locate", &fx.source_ids[src], &fx.capture_ids[cap]],
        "time_map.json",
    )
}

fn position(body: &serde_json::Value) -> u64 {
    body["result"]["capture_range"]["start"]
        .as_u64()
        .expect("capture_range.start")
}

fn assert_accepted_at(body: &serde_json::Value, expected_pos: u64, tol: u64) {
    assert_eq!(
        body["result"]["accepted"], true,
        "expected accepted result: {}",
        body["result"]
    );
    expect_close_u64("capture_range.start", expected_pos, position(body), tol);
}

fn assert_not_accepted(body: &serde_json::Value) {
    assert_eq!(
        body["result"]["accepted"], false,
        "expected rejected result: {}",
        body["result"]
    );
}

fn noise_source(len: usize, seed: u64) -> Vec<f64> {
    signal::noise(len, seed, 1.0)
}

/// 小さなベッドノイズ上にソースを `gain` で埋め込んだ capture。
fn embedded_capture(
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

// ==== 厳密な挿入 ====
#[test]
fn case_exact_insert() {
    let source = noise_source(4_096, 7);
    let capture = embedded_capture(11, 24_576, &source, 9_600, 1.0);
    let fx = mono_fixture("exact", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, 9_600, 64);
    assert!(body["result"]["agreeing_methods"].as_u64().unwrap() >= 3);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ゲイン変更された挿入 ====
#[test]
fn case_gain_insert() {
    let source = noise_source(4_096, 13);
    let capture = embedded_capture(17, 24_576, &source, 12_000, 0.4);
    let fx = mono_fixture("gain", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, 12_000, 64);
    // ミキサーが埋め込みゲインを概ね回復する
    let m = body["derived"]["mixer"]["matrix"][0][0]
        .as_f64()
        .expect("mixer matrix");
    expect_close("mixer gain", 0.4, m, Tolerance::abs(0.1));
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== EQ フィルタ済み挿入 ====
#[test]
fn case_eq_filtered_insert() {
    let source = noise_source(4_096, 19);
    // 4 kHz ピーキング -9 dB を掛けたものを埋め込む
    let eq = Biquad::peaking(RATE, 4_000.0, -9.0, 1.4);
    let filtered = eq.apply(&source);
    let capture = embedded_capture(23, 24_576, &filtered, 8_000, 1.0);
    let fx = mono_fixture("eq", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, 8_000, 64);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 無音境界 ====
#[test]
fn case_silence_boundary() {
    // 無音に挟まれたアクティブ区間の中に埋め込み。無音区間が
    // 偽ピークを生まず、実位置に定位することを確認する。
    let source = noise_source(4_096, 29);
    let mut capture = signal::silence(60_000);
    let island = signal::noise(24_000, 31, 0.05);
    signal::embed(&mut capture, &island, 20_000, 1.0);
    signal::embed(&mut capture, &source, 25_000, 1.0);
    let fx = mono_fixture("silence-boundary", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, 25_000, 64);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 強周期信号(偽ピークを持つ素材)====
#[test]
fn case_periodic_source_locates_at_phase_consistent_position() {
    // 周期 1024 の繰り返し素材を埋め込む。受理されるなら
    // 位相整合する位置(周期オフセットを含む)のみ許容し、
    // 位相のずれた偽ピークは受理されないことを pin する。
    let period = 1_024usize;
    let mut source = Vec::with_capacity(2_048);
    let body_noise = signal::noise(period, 37, 1.0);
    source.extend_from_slice(&body_noise);
    source.extend_from_slice(&body_noise);
    let embed_at = 10_000usize;
    let capture = embedded_capture(41, 24_576, &source, embed_at, 1.0);
    let fx = mono_fixture("periodic", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, embed_at as u64, 64);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ループ素材 ====
#[test]
fn case_loop_material() {
    let body_len = 4_000usize;
    let source = noise_source(body_len, 43);
    let mut capture = signal::noise(40_000, 47, 0.02);
    let first = 8_000usize;
    for k in 0..3 {
        signal::embed(&mut capture, &source, first + k * body_len, 1.0);
    }
    let fx = mono_fixture("loop", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, first as u64, 64);
    let occurrences: Vec<u64> = body["derived"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    // 残り 2 パスが検証済み出現として報告される
    assert_eq!(occurrences.len(), 2, "occurrences: {occurrences:?}");
    let mut all = occurrences.clone();
    all.push(position(&body));
    all.sort_unstable();
    let expected: Vec<u64> = (0..3).map(|k| (first + k * body_len) as u64).collect();
    for (got, want) in all.iter().zip(expected) {
        expect_close_u64("loop occurrence", want, *got, 64);
    }
    // loop_occurrences に全パスが残る
    let tm = time_map_json(&fx, 0, 0);
    assert_eq!(tm["loop_occurrences"].as_array().unwrap().len(), 3);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 複数の正当な出現 ====
#[test]
fn case_repeated_source() {
    let source = noise_source(2_048, 53);
    let mut capture = signal::noise(40_000, 59, 0.02);
    for &at in &[5_000usize, 18_000, 33_000] {
        signal::embed(&mut capture, &source, at, 1.0);
    }
    let fx = mono_fixture("repeated", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_eq!(
        body["result"]["accepted"], true,
        "result: {}",
        body["result"]
    );
    let occurrences: Vec<u64> = body["derived"]["occurrences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    // プライマリ以外の 2 か所が出現として報告される
    assert_eq!(occurrences.len(), 2, "occurrences: {occurrences:?}");
    let mut all = occurrences.clone();
    all.push(position(&body));
    all.sort_unstable();
    for (got, expected) in all.iter().zip([5_000u64, 18_000, 33_000]) {
        expect_close_u64("occurrence position", expected, *got, 64);
    }
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 一定オフセットの長尺ソース ====
#[test]
fn case_long_source_constant_offset() {
    // anchor_window(512) より十分長いソース → ウィンドウアンカーで
    // 連続区間がタイムマップに残る。
    let source = noise_source(8_192, 61);
    let embed_at = 12_000usize;
    let capture = embedded_capture(67, 30_000, &source, embed_at, 1.0);
    let fx = mono_fixture("long-offset", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, embed_at as u64, 64);

    let tm = time_map_json(&fx, 0, 0);
    let segments = tm["time_map"]["segments"].as_array().unwrap();
    let continuous: Vec<_> = segments
        .iter()
        .filter(|s| s["kind"] == "continuous")
        .collect();
    assert!(!continuous.is_empty(), "time_map: {tm}");
    // 連続区間がオフセットを表す: capture_to_source で逆引きできる
    // アンカー列を検査する
    let anchors = tm["anchors"].as_array().unwrap();
    assert!(anchors.len() >= 8, "anchors: {}", anchors.len());
    for a in anchors.iter().take(anchors.len() / 2).step_by(4) {
        let s = a["source"].as_u64().unwrap();
        let c = a["capture"].as_u64().unwrap();
        expect_close_u64("anchor offset", s + embed_at as u64, c, 96);
    }
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 線形ドリフトの長尺ソース ====
#[test]
fn case_long_source_linear_drift() {
    // 帯域制限ノイズを 1.005 倍速(= 1/1.005 のタイムスケール)で
    // 埋め込む。resample_linear は y[i] = x[i*ratio] なので
    // dcapture/dsource = 1/ratio。rate_tolerance を絞ると
    // ウィンドウアンカー列が drift segment として観測されるはず。
    let ratio = 1.005f64;
    let expected_rate = 1.0 / ratio;
    let src = signal::noise(8_192, 71, 1.0);
    let source = Biquad::lowpass(RATE, 1_000.0, 0.7071).apply(&src);
    let stretched = signal::resample_linear(&source, ratio);
    let embed_at = 8_000usize;
    let mut capture = signal::noise(24_576, 73, 0.02);
    signal::embed(&mut capture, &stretched, embed_at, 1.0);
    let fx = mono_fixture(
        "long-drift",
        &source,
        &capture,
        serde_json::json!({ "locate": { "rate_tolerance": 0.002 } }),
    );
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, embed_at as u64, 128);

    let tm = time_map_json(&fx, 0, 0);
    let drifts: Vec<f64> = tm["time_map"]["segments"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["kind"] == "drift")
        .filter_map(|s| s["rate"].as_f64())
        .collect();
    assert!(!drifts.is_empty(), "expected drift segment: {tm}");
    expect_close(
        "drift rate",
        expected_rate,
        drifts[0],
        Tolerance::abs_rel(0.002, 0.005),
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== スキップ + 再開 ====
#[test]
fn case_skip_and_resume() {
    // capture は source[0..4096] を再生後、source[5632..8192] へ
    // ジャンプして続きを再生する → Skip segment + 連続区間。
    let source = noise_source(8_192, 79);
    let embed_at = 6_000usize;
    let mut played = source[..4_096].to_vec();
    played.extend_from_slice(&source[5_632..8_192]);
    let mut capture = signal::noise(24_576, 83, 0.02);
    signal::embed(&mut capture, &played, embed_at, 1.0);
    let fx = mono_fixture("skip-resume", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, embed_at as u64, 128);

    let tm = time_map_json(&fx, 0, 0);
    let kinds: Vec<&str> = tm["time_map"]["segments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["kind"].as_str().unwrap())
        .collect();
    assert!(
        kinds.contains(&"skip"),
        "expected skip segment in {kinds:?} ({tm})"
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== マルチチャンネルダウンミックス ====
#[test]
fn case_multichannel_downmix() {
    // モノソースを L=+0.6 / R=-0.4(極性反転)でステレオに埋め込む。
    let source = noise_source(4_096, 89);
    let mut cap_l = signal::noise(16_384, 97, 0.02);
    let mut cap_r = signal::noise(16_384, 101, 0.02);
    signal::embed(&mut cap_l, &source, 4_000, 0.6);
    signal::embed(&mut cap_r, &source, 4_000, -0.4);
    let fx = fixture(
        "downmix",
        &[(&[source][..], RATE)],
        &[(&[cap_l, cap_r][..], RATE)],
        serde_json::json!({}),
    );
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_accepted_at(&body, 4_000, 64);
    // ミキサー行列がチャンネル別ゲインを回復する
    let matrix = body["derived"]["mixer"]["matrix"].as_array().unwrap();
    let m00 = matrix[0][0].as_f64().unwrap();
    let m10 = matrix[1][0].as_f64().unwrap();
    expect_close("mixer L", 0.6, m00, Tolerance::abs(0.15));
    expect_close("mixer R", -0.4, m10, Tolerance::abs(0.15));
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== 支配的 BGM に覆われた静かな素材 ====
#[test]
fn case_bgm_masks_quiet_source() {
    let quiet = noise_source(4_096, 103);
    let mut capture = signal::noise(16_384, 107, 1.0); // 支配的 BGM 床
    signal::embed(&mut capture, &quiet, 8_000, 0.03);
    let fx = mono_fixture("bgm-mask", &quiet, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    // 直接検索では受理されない(埋め込みは相関的に埋もれる)
    assert_not_accepted(&body);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== BGM 減算後の残差探索 ====
#[test]
fn case_bgm_residual_search() {
    // source0 = 支配的 BGM, source1 = 静かな素材。BGM が受理 →
    // 減算 → 残差で source1 が見つかるはず。埋め込みは BGM 領域の
    // 内側に置き、直接検索では埋もれることを確認する。
    let bgm = noise_source(16_384, 109);
    let quiet = noise_source(4_096, 113);
    let embed_at = 8_000usize;
    let mut capture = signal::noise(24_576, 127, 0.005);
    signal::embed(&mut capture, &bgm, 4_000, 1.0);
    signal::embed(&mut capture, &quiet, embed_at, 0.03);

    let fx = fixture(
        "bgm-residual",
        &[(&[bgm][..], RATE), (&[quiet][..], RATE)],
        &[(&[capture][..], RATE)],
        serde_json::json!({}),
    );
    run_locate(&fx);

    // BGM ペアは受理される
    let bgm_body = result_json(&fx, 0, 0);
    assert_eq!(
        bgm_body["result"]["accepted"], true,
        "bgm result: {}",
        bgm_body["result"]
    );
    // 直接検索では quiet は埋もれて受理されない
    let quiet_direct = result_json(&fx, 1, 0);
    assert_not_accepted(&quiet_direct);

    // 残差探索の子結果が quiet 側のディレクトリに残る
    let residual = read_json(
        &fx,
        &["locate", &fx.source_ids[1], &fx.capture_ids[0]],
        &format!("residual_{}.json", fx.source_ids[0]),
    );
    assert_eq!(
        residual["child"]["accepted"], true,
        "residual child: {}",
        residual["child"]
    );
    expect_close_u64(
        "residual position",
        embed_at as u64,
        residual["child"]["capture_range"]["start"]
            .as_u64()
            .unwrap(),
        128,
    );
    assert!(
        residual["energy"]["after"].as_f64().unwrap()
            < residual["energy"]["before"].as_f64().unwrap()
    );
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ピン留め: 無音での偽相関ピーク受理 ====
#[test]
fn pinned_silence_no_false_peak() {
    // 無音テンプレート: 全メソッドが SilenceInsufficientEnergy で
    // 棄却され、受理されない。
    let silent_source = signal::silence(4_096);
    let capture = signal::noise(16_384, 131, 1.0);
    let fx = mono_fixture(
        "pin-sil-src",
        &silent_source,
        &capture,
        serde_json::json!({}),
    );
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_not_accepted(&body);
    let validity = body["result"]["validity"].as_str().unwrap();
    expect_true(
        "silent template marked invalid",
        validity != "valid",
        format!("validity {validity}"),
    );

    // 無音キャプチャ: ノイズテンプレートを無音に探させても
    // 受理されない。
    let source = noise_source(4_096, 137);
    let silent_capture = signal::silence(16_384);
    let fx = mono_fixture(
        "pin-sil-cap",
        &source,
        &silent_capture,
        serde_json::json!({}),
    );
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_not_accepted(&body);
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ピン留め: 強周期信号での偽相関ピーク受理 ====
#[test]
fn pinned_periodic_everywhere_not_accepted_at_wrong_phase() {
    // キャプチャ全体が周期 1024 の繰り返し: 全ての位相整合位置が
    // 等価なピークになる。受理されるなら位相整合位置に限る
    // (位相ずれ偽ピークの受理は回帰とみなす)。
    let period = 1_024usize;
    let mut source = Vec::with_capacity(2_048);
    let unit = signal::noise(period, 139, 1.0);
    source.extend_from_slice(&unit);
    source.extend_from_slice(&unit);
    let mut capture = Vec::with_capacity(30_000);
    while capture.len() < 30_000 {
        capture.extend_from_slice(&unit);
    }
    capture.truncate(30_000);
    let fx = mono_fixture("pin-periodic", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    let result = &body["result"];
    if result["accepted"] == true {
        let pos = position(&body);
        expect_true(
            "periodic accept is phase-consistent",
            pos % period as u64 == 0,
            format!("accepted at non-period-aligned position {pos}"),
        );
    }
    fs::remove_dir_all(&fx.dir).ok();
}

// ==== ピン留め: 低コヒーレンス時の optimal-lag 過大推定 ====
#[test]
fn pinned_low_coherence_no_inflated_lag() {
    // 無相関な source/capture: 相関ピークの argmax が水増しされて
    // 受理されることはない。
    let source = noise_source(4_096, 149);
    let capture = signal::noise(16_384, 151, 1.0);
    let fx = mono_fixture("pin-lowcoh", &source, &capture, serde_json::json!({}));
    run_locate(&fx);
    let body = result_json(&fx, 0, 0);
    assert_not_accepted(&body);
    // コンセンサスは弱いピークをまとめ上げない: 合意メソッドが
    // 閾値未満であること。相関系メソッドのトップピークが
    // 自信ありげなスコアに水増しされていないことを pin する。
    // (spectral_envelope は包絡類似度なので白色ノイズ同士では
    // 高スコアになり得る — それを相殺するのがコンセンサスの役目)
    expect_true(
        "no consensus formed",
        body["result"]["agreeing_methods"].as_u64().unwrap() < 2,
        format!("agreeing_methods {}", body["result"]["agreeing_methods"]),
    );
    let evidence = body["result"]["evidence"].as_array().unwrap();
    for e in evidence {
        if e["method"] == "spectral_envelope" {
            continue;
        }
        if let Some(c) = e["candidates"].as_array().and_then(|c| c.first()) {
            let score = c["score"].as_f64().unwrap().abs();
            expect_true(
                "weak correlation peak stays weak",
                score < 0.85,
                format!("method {} peak score {score}", e["method"]),
            );
        }
    }
    fs::remove_dir_all(&fx.dir).ok();
}
