//! RF#628 consumer oracle: file-based 解析の独立 Oracle 検証(AL#127 slice)。
//!
//! `ruviform-audio-test` の deterministic PCM fixture(`PcmFixture`/
//! `PcmScenario`)を入力生成に使い、f32→f64 exact widen →
//! `al_testkit::media::pcm_f64le` → `Manifest`/`ArtifactRegistry` →
//! 実 `CheapAnalysisStage`/`LocateStage`/`ReferenceAnalysisStage` の
//! file-based 経路を駆動する。期待値は ruviform `oracle::
//! pcm_reference_block` の独立再実装出力を f64 へ widen した統計量と、
//! pattern の解析的参照(sine 周波数・constant level)からテスト側で
//! 独立に導出し、被験実装と同じコード経路を期待値計算に使わない。
//!
//! - AL-1: f64 fidelity(dc/rms/peak が f32 相当に縮退しない)
//! - AL-2: 長尺 scenario の locate/reference 非回帰
//! - AL-3: 非連続 scenario の gap → NaN → Validity/Provenance
//!
//! AL-4(clock drift)は live clock consumer 不在で Deferred、
//! AL-5(device/pipeline/midi legs)は device boundary 非消費で
//! NotApplicable。詳細は docs/test-specs/issue-127-rf628-consumer-oracle.md。

use std::fs;
use std::path::{Path, PathBuf};

use al_core::{InputRole, JobId, StageKind};
use al_job::job::input;
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::registry::ArtifactRegistry;
use al_job::stage::{Stage, StageContext};
use al_locate::stage::LocateStage;
use al_testkit::media::pcm_f64le;
use al_testkit::signal;
use al_testkit::temp::TempDir;
use ruviform_audio_core::SampleRate;
use ruviform_audio_test::oracle::{PcmCompare, pcm_assert_eq, pcm_reference_block};
use ruviform_audio_test::pcm::{PcmFixture, PcmPattern, PcmScenario, PcmSegment};

const RATE: u32 = 48_000;
/// level trajectory の window/hop(CheapConfig 既定 20ms/10ms)。
const LEVEL_WINDOW: usize = 960;
const LEVEL_HOP: usize = 480;

fn sample_rate() -> SampleRate {
    SampleRate::new(RATE).expect("48kHz sample rate")
}

fn fixture(pattern: PcmPattern, channels: u16) -> PcmFixture {
    PcmFixture::new(sample_rate(), channels, pattern).expect("valid fixture")
}

/// `fixture` の `[start, start+frames)` を channel 別 f64 へ展開する。
/// f32→f64 widen は `f64::from` で exact。block 単位で生成する。
fn render_fixture(fixture: &PcmFixture, start: u64, frames: u64) -> Vec<Vec<f64>> {
    const CHUNK: u64 = 65_536;
    let channels = usize::from(fixture.channels);
    let mut out = vec![Vec::with_capacity(frames as usize); channels];
    let mut pos = start;
    let end = start + frames;
    while pos < end {
        let n = (end - pos).min(CHUNK) as u32;
        let block = fixture.block(pos, n).expect("fixture block");
        for (ch, dst) in out.iter_mut().enumerate() {
            let base = ch * block.frames as usize;
            dst.extend(
                block.data[base..base + block.frames as usize]
                    .iter()
                    .map(|&s| f64::from(s)),
            );
        }
        pos += u64::from(n);
    }
    out
}

/// `scenario` の `[0, total_frames)` を channel 別 f64 へ展開する。
/// segment が覆わない frame は dropped/missing frames として
/// `f64::NAN` を置く(ingest/decode の正規化は介さず、decoded_pcm
/// artifact の表現として直接 NaN を書く)。
fn render_scenario(scenario: &PcmScenario, total_frames: u64) -> Vec<Vec<f64>> {
    let channels = scenario
        .segments()
        .iter()
        .map(|s| usize::from(s.fixture.channels))
        .max()
        .expect("non-empty scenario");
    let mut out = vec![vec![f64::NAN; total_frames as usize]; channels];
    for seg in scenario.segments() {
        let rendered = render_fixture(&seg.fixture, seg.frame_start, seg.frame_len);
        let lo = seg.frame_start as usize;
        let hi = lo + seg.frame_len as usize;
        for (ch, samples) in rendered.iter().enumerate() {
            out[ch][lo..hi].copy_from_slice(samples);
        }
    }
    out
}

/// oracle 参照ブロックを f64 へ widen した channel サンプル列。
/// `render_fixture` の出力と一致するはずだが、期待値導出は
/// `pcm_reference_block`(spec の独立再実装)を直接使う。
fn oracle_channel_f64(fixture: &PcmFixture, frames: u64, channel: u16) -> Vec<f64> {
    const CHUNK: u64 = 65_536;
    let mut out = Vec::with_capacity(frames as usize);
    let mut pos = 0u64;
    while pos < frames {
        let n = (frames - pos).min(CHUNK) as u32;
        out.extend(
            pcm_reference_block(fixture, pos, n, channel)
                .iter()
                .map(|&s| f64::from(s)),
        );
        pos += u64::from(n);
    }
    out
}

struct OracleJob {
    dir: TempDir,
    manifest: Manifest,
    config: serde_json::Value,
    input_ids: Vec<al_core::InputId>,
}

fn write_pcm(job_dir: &Path, rel: &str, samples: &[f64]) {
    let path = job_dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, pcm_f64le(samples)).unwrap();
}

fn new_job(tag: &str, roles: &[InputRole], config: serde_json::Value) -> OracleJob {
    let dir = TempDir::new(&format!("al-rf628-{tag}"));
    let inputs: Vec<_> = roles
        .iter()
        .enumerate()
        .map(|(i, role)| {
            let mut inp = input(*role, dir.join(format!("in{i}.bin")));
            inp.source_sha256 = Some(format!("{:02x}", i + 1).repeat(32));
            inp
        })
        .collect();
    let input_ids: Vec<_> = inputs.iter().map(|i| i.input_id.clone()).collect();
    let manifest = Manifest::new(
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
    OracleJob {
        dir,
        manifest,
        config,
        input_ids,
    }
}

/// `channels`(channel 別 f64)を interleave して decoded_pcm
/// artifact として `inputs[idx]` に登録する。
fn add_decoded_pcm(job: &mut OracleJob, idx: usize, rel: &str, channels: &[Vec<f64>]) {
    write_pcm(&job.dir, rel, &signal::interleave(channels));
    let declared = job.manifest.input_artifact(&job.input_ids[idx]).unwrap();
    job.manifest.register_artifact(ArtifactRecord {
        artifact_id: al_core::ArtifactId::new(),
        role: "decoded_pcm".into(),
        path: PathBuf::from(rel),
        produced_by: Some(StageKind::Ingest),
        inputs: vec![declared],
        params: serde_json::json!({
            "sample_rate": RATE,
            "channels": channels.len(),
        }),
    });
}

fn ctx<'a>(job: &'a OracleJob, registry: &'a ArtifactRegistry) -> StageContext<'a> {
    StageContext {
        manifest: &job.manifest,
        registry,
        config: &job.config,
    }
}

/// 実 `CheapAnalysisStage` を駆動し、入力ごとの persisted
/// `analysis/features/cheap/<input_id>.json` の records を返す。
fn run_cheap(job: &mut OracleJob) -> Vec<Vec<serde_json::Value>> {
    let registry = ArtifactRegistry::new(job.dir.as_ref());
    let produced = al_analysis::stage::CheapAnalysisStage::new()
        .run(&ctx(job, &registry))
        .expect("cheap analysis stage");
    for rec in produced {
        job.manifest.register_artifact(rec);
    }
    job.input_ids
        .iter()
        .map(|id| {
            let body: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(
                    job.dir
                        .join("analysis")
                        .join("features")
                        .join("cheap")
                        .join(format!("{id}.json")),
                )
                .expect("cheap feature records persisted"),
            )
            .expect("cheap records parse");
            body["records"].as_array().expect("records array").clone()
        })
        .collect()
}

/// locate → reference analysis を駆動し、reference feature records
/// と locate result.json を返す(golden.rs の `run` と同型)。
fn run_locate_reference(job: &mut OracleJob) -> (serde_json::Value, serde_json::Value) {
    let registry = ArtifactRegistry::new(job.dir.as_ref());
    for rec in LocateStage::new()
        .run(&ctx(job, &registry))
        .expect("locate")
    {
        job.manifest.register_artifact(rec);
    }
    let records = al_analysis::stage::ReferenceAnalysisStage::new()
        .run(&ctx(job, &registry))
        .expect("reference analysis stage");
    assert!(
        records.iter().any(|r| r.role == "reference_features"),
        "no reference_features artifact"
    );
    let src = job.input_ids[0].to_string();
    let cap = job.input_ids[1].to_string();
    let features: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            job.dir
                .join("analysis")
                .join("features")
                .join("reference")
                .join(format!("{src}__{cap}.json")),
        )
        .expect("reference feature records persisted"),
    )
    .expect("reference records parse");
    let locate_body: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            job.dir
                .join("locate")
                .join(&src)
                .join(&cap)
                .join("result.json"),
        )
        .expect("locate result.json"),
    )
    .expect("locate result parses");
    (features, locate_body)
}

fn find<'a>(records: &'a [serde_json::Value], id: &str, channel: u64) -> &'a serde_json::Value {
    records
        .iter()
        .find(|r| r["feature_id"] == id && r["channels"]["channel"].as_u64() == Some(channel))
        .unwrap_or_else(|| panic!("missing record {id} ch{channel}"))
}

fn scalar(rec: &serde_json::Value) -> f64 {
    rec["value"]["scalar"]
        .as_f64()
        .unwrap_or_else(|| panic!("record {} is not a finite scalar", rec["feature_id"]))
}

/// level 系 record の独立期待値。`measure_level` と同じ定義を
/// テスト側で再実装する(f64 統計量・4x 線形 true-peak)。
struct LevelExpect {
    dc: f64,
    peak: f64,
    rms: f64,
    true_peak: f64,
}

fn level_expect(samples: &[f64]) -> LevelExpect {
    let n = samples.len() as f64;
    let dc = samples.iter().sum::<f64>() / n;
    let peak = samples.iter().fold(0.0f64, |m, &v| m.max(v.abs()));
    let rms = (samples.iter().map(|v| v * v).sum::<f64>() / n).sqrt();
    let mut true_peak = peak;
    for w in samples.windows(2) {
        for k in 1..4 {
            let t = k as f64 / 4.0;
            true_peak = true_peak.max((w[0] + (w[1] - w[0]) * t).abs());
        }
    }
    LevelExpect {
        dc,
        peak,
        rms,
        true_peak,
    }
}

fn dbfs(rms: f64) -> f64 {
    20.0 * rms.max(1e-12).log10()
}

fn assert_level_records(records: &[serde_json::Value], expect: &LevelExpect, channel: u64) {
    let dc = find(records, "level.dc_offset", channel);
    assert!(
        (scalar(dc) - expect.dc).abs() < 1e-9,
        "dc_offset {} != expected {}",
        scalar(dc),
        expect.dc
    );
    let peak = find(records, "level.sample_peak", channel);
    assert!(
        (scalar(peak) - expect.peak).abs() < 1e-9,
        "sample_peak {} != expected {}",
        scalar(peak),
        expect.peak
    );
    let tp = find(records, "level.true_peak_est", channel);
    assert!(
        (scalar(tp) - expect.true_peak).abs() < 1e-9,
        "true_peak_est {} != expected {}",
        scalar(tp),
        expect.true_peak
    );
    let rms_db = find(records, "level.rms_dbfs", channel);
    assert!(
        (scalar(rms_db) - dbfs(expect.rms)).abs() < 1e-6,
        "rms_dbfs {} != expected {}",
        scalar(rms_db),
        dbfs(expect.rms)
    );
    let crest = find(records, "level.crest_factor", channel);
    let expected_crest = if expect.rms > 0.0 {
        expect.peak / expect.rms
    } else {
        0.0
    };
    assert!(
        (scalar(crest) - expected_crest).abs() < 1e-9,
        "crest_factor {} != expected {}",
        scalar(crest),
        expected_crest
    );
}

// ==== R1: fixture 出力と独立 oracle の一致(入力データの妥当性) ====

#[test]
fn rf628_fixture_matches_independent_oracle() {
    let cases: &[(&str, PcmPattern, PcmCompare)] = &[
        ("silence", PcmPattern::Silence, PcmCompare::Exact),
        (
            "constant",
            PcmPattern::Constant { value: 0.25 },
            PcmCompare::Exact,
        ),
        (
            "impulse",
            PcmPattern::Impulse {
                period_frames: 48,
                amplitude: 0.9,
            },
            PcmCompare::Exact,
        ),
        (
            "ramp",
            PcmPattern::Ramp {
                amplitude: 0.8,
                period_frames: 96,
            },
            PcmCompare::Exact,
        ),
        (
            "square",
            PcmPattern::Square {
                amplitude: 0.6,
                period_frames: 48,
            },
            PcmCompare::Exact,
        ),
        (
            "sine",
            PcmPattern::Sine {
                freq_hz: 1500.0,
                amplitude: 0.5,
            },
            PcmCompare::AbsEpsilon(1e-5),
        ),
        (
            "sweep",
            PcmPattern::Sweep {
                f0_hz: 100.0,
                f1_hz: 4000.0,
                duration_frames: 9600,
                amplitude: 0.4,
            },
            PcmCompare::AbsEpsilon(1e-5),
        ),
        (
            "noise",
            PcmPattern::Noise {
                seed: 42,
                amplitude: 0.8,
            },
            PcmCompare::Exact,
        ),
    ];
    for (name, pattern, mode) in cases {
        for channels in [1u16, 2] {
            let fx = fixture(*pattern, channels);
            let frames = 8192u64;
            let block = fx.block(0, frames as u32).expect("fixture block");
            assert_eq!(block.channels, channels);
            for ch in 0..usize::from(channels) {
                let got = &block.data[ch * frames as usize..(ch + 1) * frames as usize];
                let want = pcm_reference_block(&fx, 0, frames as u32, ch as u16);
                pcm_assert_eq(got, &want, *mode).unwrap_or_else(|e| panic!("{name} ch{ch}: {e}"));
            }
        }
    }
}

// ==== AL-1: f64 fidelity ====

/// Constant: 解析的参照(dc=peak=rms=|value|)と f64 統計量の両方で照合。
/// f32 累積なら dc_offset が ~1e-4 単位でずれ、1e-9 許容差で検出できる。
#[test]
fn rf628_f64_fidelity_constant() {
    let fx = fixture(PcmPattern::Constant { value: 0.25 }, 1);
    let frames = RATE as u64; // 1 s
    let channels = render_fixture(&fx, 0, frames);
    let mut job = new_job(
        "al1-const",
        &[InputRole::SourceAsset],
        serde_json::json!({}),
    );
    add_decoded_pcm(&mut job, 0, "inputs/0.f64le", &channels);
    let records = run_cheap(&mut job).remove(0);

    let expected_samples = oracle_channel_f64(&fx, frames, 0);
    let expect = level_expect(&expected_samples);
    assert_level_records(&records, &expect, 0);
    // 解析的参照: 定数信号の厳密値。
    assert!((expect.dc - 0.25).abs() < 1e-12);
    assert!((expect.peak - 0.25).abs() < 1e-12);
    assert!((expect.rms - 0.25).abs() < 1e-12);
    assert!((scalar(find(&records, "level.rms_dbfs", 0)) - 20.0 * 0.25f64.log10()).abs() < 1e-6);
    // ruviform provenance stamp: 解決 SHA と precision=f64。
    for id in ["level.dc_offset", "level.rms_dbfs", "level.trajectory"] {
        let stamp = &find(&records, id, 0)["params"]["ruviform"];
        assert_eq!(stamp["precision"], "f64", "{id} precision stamp");
        assert!(
            stamp["rev"].as_str().is_some_and(|s| s.len() == 40),
            "{id} rev stamp: {stamp}"
        );
    }
}

/// Sine: noise.tonal_peak_hz が既知周波数を返し、level 系が oracle
/// 参照統計量と一致する。Square: 高調波を含む既知 rms/peak。
#[test]
fn rf628_f64_fidelity_periodic() {
    for (name, pattern, want_freq) in [
        (
            "sine",
            PcmPattern::Sine {
                freq_hz: 1500.0,
                amplitude: 0.5,
            },
            1500.0,
        ),
        (
            "square",
            PcmPattern::Square {
                amplitude: 0.5,
                period_frames: 48, // 1000 Hz fundamental
            },
            1000.0,
        ),
    ] {
        let fx = fixture(pattern, 1);
        let frames = 4 * RATE as u64; // 4 s — tonal 解像度を確保
        let channels = render_fixture(&fx, 0, frames);
        let mut job = new_job(
            &format!("al1-{name}"),
            &[InputRole::SourceAsset],
            serde_json::json!({}),
        );
        add_decoded_pcm(&mut job, 0, "inputs/0.f64le", &channels);
        let records = run_cheap(&mut job).remove(0);

        let expect = level_expect(&oracle_channel_f64(&fx, frames, 0));
        assert_level_records(&records, &expect, 0);

        // 解析的参照: tonal peak は既知 fundamental 周波数近傍。
        // STFT bin 分解能(rate/2048 ≈ 23.4 Hz)より緩いが、
        // 隣接 bin 誤同定を排除できる程度に絞る。
        let tonal = find(&records, "noise.tonal_peak_hz", 0);
        let got_freq = scalar(tonal);
        assert!(
            (got_freq - want_freq).abs() <= RATE as f64 / 2048.0,
            "{name}: tonal_peak_hz {got_freq} != {want_freq}±{}",
            RATE as f64 / 2048.0
        );
    }
}

/// Noise: oracle 参照ブロックの f64 統計量と level record を照合。
/// seed 決定的な広帯域信号で dc/rms の f64 蓄積精度を検証する。
#[test]
fn rf628_f64_fidelity_noise() {
    let fx = fixture(
        PcmPattern::Noise {
            seed: 42,
            amplitude: 0.8,
        },
        1,
    );
    let frames = 4 * RATE as u64;
    let channels = render_fixture(&fx, 0, frames);
    let mut job = new_job(
        "al1-noise",
        &[InputRole::SourceAsset],
        serde_json::json!({}),
    );
    add_decoded_pcm(&mut job, 0, "inputs/0.f64le", &channels);
    let records = run_cheap(&mut job).remove(0);

    let expect = level_expect(&oracle_channel_f64(&fx, frames, 0));
    assert_level_records(&records, &expect, 0);
    // Noise はゼロ平均近似: |dc| は amplitude/√n 程度に収まる。
    assert!(expect.dc.abs() < 0.02, "noise dc {}", expect.dc);
}

/// Stereo Noise: channel 別 record が channel 独立 stream の
/// oracle 統計量と一致する(interleave→deinterleave の分離を検証)。
#[test]
fn rf628_f64_fidelity_stereo_channels() {
    let fx = fixture(
        PcmPattern::Noise {
            seed: 7,
            amplitude: 0.6,
        },
        2,
    );
    let frames = 2 * RATE as u64;
    let channels = render_fixture(&fx, 0, frames);
    assert_eq!(channels.len(), 2);
    let mut job = new_job(
        "al1-stereo",
        &[InputRole::SourceAsset],
        serde_json::json!({}),
    );
    add_decoded_pcm(&mut job, 0, "inputs/0.f64le", &channels);
    let records = run_cheap(&mut job).remove(0);

    for ch in 0..2u64 {
        let expect = level_expect(&oracle_channel_f64(&fx, frames, ch as u16));
        assert_level_records(&records, &expect, ch);
    }
    // channel 間で統計量が独立であること(hash に channel を含む)。
    let dc0 = scalar(find(&records, "level.dc_offset", 0));
    let dc1 = scalar(find(&records, "level.dc_offset", 1));
    assert!(
        (dc0 - dc1).abs() > 1e-4,
        "stereo channels collapsed: dc0={dc0} dc1={dc1}"
    );
}

// ==== AL-2: 長尺 scenario 非回帰 ====

/// contiguous `PcmScenario`(Noise→Sine→Square、48kHz・計 30 秒)を
/// dataset 形状 artifact として流し、locate→reference の参照再計算が
/// golden 許容帯域で非回帰であること、長大 trajectory が破綻しない
/// ことを確認する。golden fixture(~0.3s)に対し frame 数は ~2000 倍。
/// 支配コストは locate の相互相関(O(N)級)で、debug で 60s→150s、
/// 45s→130s、30s→~90s とほぼ線形。tensor chunk 境界と trajectory
/// 長は 30s で十分に複数 chunk を踏むため、この規模を採用する。
#[test]
fn rf628_long_scenario_non_regression() {
    let seg_frames = 10 * u64::from(RATE); // 10 s per segment
    let patterns = [
        PcmPattern::Noise {
            seed: 42,
            amplitude: 0.7,
        },
        PcmPattern::Sine {
            freq_hz: 1500.0,
            amplitude: 0.5,
        },
        PcmPattern::Square {
            amplitude: 0.5,
            period_frames: 48,
        },
    ];
    let mut segments = Vec::new();
    let mut pos = 0u64;
    for pattern in patterns {
        segments.push(PcmSegment {
            frame_start: pos,
            frame_len: seg_frames,
            fixture: fixture(pattern, 1),
        });
        pos += seg_frames;
    }
    let scenario = PcmScenario::new(segments).expect("contiguous scenario");
    assert!(scenario.is_contiguous());
    let source_channels = render_scenario(&scenario, pos);
    assert_eq!(source_channels.len(), 1);
    let source = &source_channels[0];

    // capture: source を 0.5 gain で quiet bed に埋め込む。
    let embed_at = 4_096usize;
    let mut capture = signal::noise(source.len() + 8_192, 99, 0.01);
    signal::embed(&mut capture, source, embed_at, 0.5);

    let mut job = new_job(
        "al2-long",
        &[InputRole::SourceAsset, InputRole::CaptureBug],
        serde_json::json!({}),
    );
    add_decoded_pcm(&mut job, 0, "inputs/0.f64le", &source_channels);
    add_decoded_pcm(&mut job, 1, "inputs/1.f64le", &[capture]);

    // cheap: 長大 trajectory tensor が duration/hop と整合する。
    let t0 = std::time::Instant::now();
    let cheap = run_cheap(&mut job);
    eprintln!("rf628_long: cheap stage {:?}", t0.elapsed());
    let src_records = &cheap[0];
    let expected_windows = (source.len() - LEVEL_WINDOW) / LEVEL_HOP + 1;
    let traj = find(src_records, "level.trajectory", 0);
    assert_eq!(
        traj["support_count"].as_u64().unwrap() as usize,
        expected_windows,
        "trajectory window count"
    );
    assert_eq!(traj["validity"], "valid");
    assert!(scalar(find(src_records, "level.rms_dbfs", 0)).is_finite());

    // locate → reference: golden と同じ許容帯域で非回帰。
    let t1 = std::time::Instant::now();
    let (body, locate) = run_locate_reference(&mut job);
    eprintln!("rf628_long: locate+reference {:?}", t1.elapsed());
    let recs = body["records"].as_array().unwrap();
    let gain = scalar(
        recs.iter()
            .find(|r| r["feature_id"] == "ref.gain_ls")
            .expect("ref.gain_ls"),
    );
    assert!((gain - 0.5).abs() <= 0.05, "ref.gain_ls {gain} != 0.5±0.05");
    let rms_ratio = scalar(
        recs.iter()
            .find(|r| r["feature_id"] == "ref.rms_ratio")
            .expect("ref.rms_ratio"),
    );
    assert!(
        (rms_ratio - 0.5).abs() <= 0.05,
        "ref.rms_ratio {rms_ratio} != 0.5±0.05"
    );
    let lag_median = scalar(
        recs.iter()
            .find(|r| r["feature_id"] == "ref.lag_median")
            .expect("ref.lag_median"),
    );
    assert!(
        lag_median.abs() <= 8.0,
        "ref.lag_median {lag_median} != 0±8"
    );
    let similarity = scalar(
        recs.iter()
            .find(|r| r["feature_id"] == "ref.similarity_fullband")
            .expect("ref.similarity_fullband"),
    );
    assert!(
        similarity > 0.9,
        "ref.similarity_fullband {similarity} <= 0.9"
    );
    // embed 位置が locate の capture_range.start として回収される。
    let pos = locate["result"]["capture_range"]["start"]
        .as_u64()
        .expect("capture_range.start");
    assert!(
        (pos as i64 - embed_at as i64).unsigned_abs() <= 512,
        "capture_range.start {pos} != embed_at {embed_at}±512"
    );
}

// ==== AL-3: gap → NaN → Validity/Provenance ====

/// 非連続 `PcmScenario` の gap frame を `f64::NAN` として
/// decoded_pcm に展開し、segment 範囲由来の期待 validity map と
/// AL 出力を照合する。`decode_input` は plain decoded_pcm の
/// non-finite を拒否しない(normalized_pcm 経路のみ拒否)ため、
/// NaN は channel_samples へ届き、既存の NaN 伝播意味論が観測される。
#[test]
fn rf628_gap_nan_validity() {
    let rate = u64::from(RATE);
    // seg1: Sine 2 s [0, 96000)、gap 0.5 s [96000, 120000)、
    // seg2: Constant 2 s [120000, 216000)。
    let (seg1_len, gap_len, seg2_len) = (2 * rate, rate / 2, 2 * rate);
    let total = seg1_len + gap_len + seg2_len;
    let gap_start = seg1_len;
    let gap_end = gap_start + gap_len;
    let scenario = PcmScenario::new(vec![
        PcmSegment {
            frame_start: 0,
            frame_len: seg1_len,
            fixture: fixture(
                PcmPattern::Sine {
                    freq_hz: 1500.0,
                    amplitude: 0.5,
                },
                1,
            ),
        },
        PcmSegment {
            frame_start: gap_end,
            frame_len: seg2_len,
            fixture: fixture(PcmPattern::Constant { value: 0.4 }, 1),
        },
    ])
    .expect("gapped scenario");
    assert!(!scenario.is_contiguous());
    assert!(scenario.segment_at(gap_start).is_none());
    assert!(scenario.segment_at(gap_start + gap_len - 1).is_none());
    assert!(scenario.segment_at(gap_end).is_some());

    let channels = render_scenario(&scenario, total);
    assert!(
        channels[0]
            .iter()
            .skip(gap_start as usize)
            .take(gap_len as usize)
            .all(|v| v.is_nan())
    );
    assert!(
        channels[0][..gap_start as usize]
            .iter()
            .all(|v| v.is_finite())
    );

    let mut job = new_job("al3-gap", &[InputRole::SourceAsset], serde_json::json!({}));
    add_decoded_pcm(&mut job, 0, "inputs/0.f64le", &channels);
    let records = run_cheap(&mut job).remove(0);

    // --- 期待 validity map: window/hop 幾何と segment 範囲から独立導出。
    let n_windows = (total as usize - LEVEL_WINDOW) / LEVEL_HOP + 1;
    let mut expected_nan = vec![false; n_windows];
    for (i, is_nan) in expected_nan.iter_mut().enumerate() {
        let w_start = i * LEVEL_HOP;
        let w_end = w_start + LEVEL_WINDOW;
        // window が gap と 1 frame でも重なれば rms は NaN を伝播する。
        *is_nan = (w_start..w_end).any(|f| f >= gap_start as usize && f < gap_end as usize);
    }
    let expected_finite = expected_nan.iter().filter(|&&b| !b).count() as u64;

    // trajectory tensor: NaN 窓位置が期待 map と一致。
    let traj_rec = find(&records, "level.trajectory", 0);
    assert_eq!(
        traj_rec["support_count"].as_u64().unwrap() as usize,
        n_windows
    );
    let array = traj_rec["value"]["tensor"]["array"]
        .as_str()
        .expect("trajectory tensor array");
    let name = array
        .strip_prefix("features/cheap/")
        .expect("dataset-relative array path");
    let reader =
        al_dataset::reader::DatasetReader::open(&job.dir.join("analysis")).expect("dataset reader");
    let (data, _) = reader
        .read_tensor("features/cheap", name, &[[0, 1], [0, n_windows as u64]])
        .expect("trajectory tensor");
    assert_eq!(data.len(), n_windows);
    for (i, (&v, &want_nan)) in data.iter().zip(&expected_nan).enumerate() {
        assert_eq!(
            v.is_nan(),
            want_nan,
            "trajectory[{i}] = {v} (expected_nan={want_nan})"
        );
    }
    // 有限窓の rms は seg の既知水準(sine≈0.3536, const=0.4)に近い。
    let finite_vals: Vec<f64> = data.iter().copied().filter(|v| v.is_finite()).collect();
    assert!(!finite_vals.is_empty());
    assert!(finite_vals.iter().all(|v| (0.3..=0.45).contains(v)));

    // robust summary record: support_count = 有限窓数。
    for id in [
        "level.rms_median_dbfs",
        "level.rms_mad_db",
        "level.rms_p05_dbfs",
        "level.rms_p95_dbfs",
    ] {
        let rec = find(&records, id, 0);
        assert_eq!(
            rec["support_count"].as_u64(),
            Some(expected_finite),
            "{id} support_count"
        );
    }

    // --- NaN 伝播の pin(既存意味論、test-spec R6)。
    // dc は sum 伝播で NaN → JSON null。
    let dc = find(&records, "level.dc_offset", 0);
    assert!(
        dc["value"]["scalar"].is_null(),
        "dc_offset should persist NaN as null: {}",
        dc["value"]
    );
    // peak/true_peak は f64::max の NaN 無視で有限(valid 区間の最大)。
    let peak = scalar(find(&records, "level.sample_peak", 0));
    assert!(
        peak.is_finite() && peak > 0.45 && peak <= 0.5 + 1e-3,
        "sample_peak {peak}"
    );
    let tp = scalar(find(&records, "level.true_peak_est", 0));
    assert!(tp.is_finite() && tp >= peak, "true_peak_est {tp}");
    // rms_all は NaN 伝播 → db floor -240。
    let rms_db = scalar(find(&records, "level.rms_dbfs", 0));
    assert!(
        (rms_db - (-240.0)).abs() < 1e-3,
        "rms_dbfs {rms_db} != -240 floor"
    );
    // crest_factor は rms_all NaN で 0.0。
    assert_eq!(scalar(find(&records, "level.crest_factor", 0)), 0.0);
    // integrated loudness も -240 floor。
    let integrated = scalar(find(&records, "level.integrated_dbfs", 0));
    assert!(
        (integrated - (-240.0)).abs() < 1e-3,
        "integrated_dbfs {integrated} != -240 floor"
    );

    // --- Provenance: 全 record の params.ruviform stamp。
    for rec in &records {
        let stamp = &rec["params"]["ruviform"];
        assert_eq!(stamp["precision"], "f64", "record {:?}", rec["feature_id"]);
        assert!(
            stamp["rev"].as_str().is_some_and(|s| s.len() == 40),
            "record {:?} rev stamp: {stamp}",
            rec["feature_id"]
        );
    }
}
