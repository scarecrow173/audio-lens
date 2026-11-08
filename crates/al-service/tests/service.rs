//! `al-service` の headless 回帰試験。GUI(al-gui)と CLI(al-cli)が
//! 共有する JobService / DatasetSession の契約を検証する:
//! run/resume/inspect/list、キャンセル→Pending 保持→resume 完走、
//! dataset open の失敗分類、range read と validity 保持。

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use al_core::{JobId, StageKind, StageStatus};
use al_service::dataset::{DatasetSession, FeatureQuery};
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::{ServiceError, parse_input_spec, parse_stage};
use ruviform_viz::source::WaveformSource;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-service-{tag}-{}", JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wav_path(dir: &Path, name: &str, frames: usize) -> PathBuf {
    let samples: Vec<i16> = (0..frames).map(|i| (i % 400) as i16 - 200).collect();
    let path = dir.join(name);
    std::fs::write(&path, al_testkit::media::wav_s16_bytes(1, 48_000, &samples)).unwrap();
    path
}

fn run_spec(path: &Path, stages: Option<Vec<StageKind>>) -> RunSpec {
    RunSpec {
        inputs: vec![parse_input_spec(&format!("capture_bug:{}", path.display())).unwrap()],
        stages,
        config: serde_json::json!({}),
    }
}

#[test]
fn run_ingest_then_inspect_and_list() {
    let dir = temp_dir("inspect");
    let wav = wav_path(&dir, "cap.wav", 960);
    let service = JobService::new(dir.join("jobs"));

    let (job_id, manifest) = service
        .run(
            run_spec(&wav, Some(vec![StageKind::Ingest])),
            &RunControl::default(),
        )
        .unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed)
    );

    let inspection = service.inspect(&job_id).unwrap();
    assert_eq!(inspection.manifest.job_id, job_id);
    assert!(!inspection.dataset_ready);

    let jobs = service.list().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_id, job_id);
    assert_eq!(jobs[0].stages.len(), 1);
    assert!(!jobs[0].dataset_ready);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn inspect_missing_job_is_not_found() {
    let dir = temp_dir("missing");
    let service = JobService::new(dir.join("jobs"));
    let err = service.inspect(&JobId::new()).unwrap_err();
    assert_eq!(err.code(), "not_found");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn cancel_leaves_pending_stage_and_resume_completes() {
    let dir = temp_dir("cancel");
    let wav = wav_path(&dir, "cap.wav", 960);
    let service = JobService::new(dir.join("jobs"));

    // ingest → cheap_analysis → dataset の先頭でキャンセル: 全 stage が
    // Pending のまま残り、resume で完走できることを見る。
    let cancel = || true;
    let ctl = RunControl {
        on_stage: None,
        is_cancelled: Some(&cancel),
    };
    let err = service
        .run(
            run_spec(
                &wav,
                Some(vec![
                    StageKind::Ingest,
                    StageKind::CheapAnalysis,
                    StageKind::Dataset,
                ]),
            ),
            &ctl,
        )
        .unwrap_err();
    assert_eq!(err.code(), "cancelled");

    let jobs = service.list().unwrap();
    assert_eq!(jobs.len(), 1);
    let job_id = jobs[0].job_id.clone();
    assert!(
        jobs[0]
            .stages
            .iter()
            .all(|(_, status, _)| *status == StageStatus::Pending)
    );

    // resume で完走 → dataset が開ける。
    let manifest = service.resume(&job_id, &RunControl::default()).unwrap();
    assert!(
        manifest
            .stages
            .iter()
            .all(|s| s.status == StageStatus::Completed)
    );
    assert!(service.inspect(&job_id).unwrap().dataset_ready);
    let session = service.open_dataset(&job_id).unwrap();
    assert_eq!(session.root(), service.dataset_dir(&job_id).as_path());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn run_to_dataset_then_query_features_and_range_reads() {
    let dir = temp_dir("dataset");
    let wav = wav_path(&dir, "cap.wav", 48_000);
    let service = JobService::new(dir.join("jobs"));
    let (job_id, _) = service
        .run(
            run_spec(
                &wav,
                Some(vec![
                    StageKind::Ingest,
                    StageKind::CheapAnalysis,
                    StageKind::Dataset,
                ]),
            ),
            &RunControl::default(),
        )
        .unwrap();

    let session = service.open_dataset(&job_id).unwrap();

    // overview/timeline は validity 付き構造で返る。
    let overview = session.overview().unwrap();
    assert!(overview.get("artifact_count").is_some() || overview.get("artifacts").is_some());
    let timeline = session.timeline().unwrap();
    let inputs = timeline["inputs"].as_array().expect("timeline inputs");
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0]["sample_rate"], 48_000);
    let input_id = inputs[0]["input_id"].as_str().unwrap().to_string();

    // features: FeatureQuery 経由で validity/confidence 保持のまま取れる。
    let features = session
        .features(&FeatureQuery {
            roles: Some(vec!["features/cheap".to_string()]),
            ..FeatureQuery::default()
        })
        .unwrap();
    let records = features["records"].as_array().expect("feature records");
    assert!(!records.is_empty());
    // 全レコードが validity を持つ(unavailable/invalid でも欠落しない)。
    for r in records.iter().take(50) {
        assert!(r.get("validity").is_some(), "record missing validity: {r}");
    }

    // artifacts 列挙と詳細。
    let artifacts = session.artifacts(None);
    assert!(!artifacts.is_empty());
    let first = artifacts[0]["path"].as_str().unwrap().to_string();
    let detail = session.artifact_detail(&first).unwrap();
    assert_eq!(detail["artifact"]["path"], first);

    // 波形 view: 範囲指定で RVBW エンコード済みバイト列が返る。
    let (lod, meta) = session.waveform_view(&input_id, 0, 0.0, 0.02, 64).unwrap();
    assert_eq!(&lod.bytes.0[..4], b"RVBW");
    assert_eq!(meta["sample_rate"], 48_000);
    assert!(lod.bytes.0.len() >= 16 + 44); // header + >=1 bin

    // 有界 read: 波形 source の先頭しか読まない(全量ロードしない)。
    let (lod2, _) = session.waveform_view(&input_id, 0, 0.0, 0.005, 32).unwrap();
    assert!(lod2.bytes.0.len() <= lod.bytes.0.len());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn waveform_source_yields_samples_normalized_to_unit_range() {
    let dir = temp_dir("wave-norm");
    let wav = wav_path(&dir, "cap.wav", 4_800);
    let service = JobService::new(dir.join("jobs"));
    let (job_id, _) = service
        .run(
            run_spec(
                &wav,
                Some(vec![
                    StageKind::Ingest,
                    StageKind::CheapAnalysis,
                    StageKind::Dataset,
                ]),
            ),
            &RunControl::default(),
        )
        .unwrap();

    let session = service.open_dataset(&job_id).unwrap();
    let timeline = session.timeline().unwrap();
    let input_id = timeline["inputs"][0]["input_id"]
        .as_str()
        .unwrap()
        .to_string();

    // pcm_s16 の整数スケール(±32768)ではなく [-1,1] に正規化されて
    // 返ること。ruviform-viz の renderer は単位振幅を前提とする。
    let source = session.waveform(&input_id).unwrap();
    let block = source.read(0, 256).unwrap();
    assert_eq!(block.frames, 256);
    assert!(!block.samples.is_empty());
    let max_abs = block.samples.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    // 入力は (i % 400) - 200 の s16 サンプル。最大絶対値は 200。
    let expected = 200.0 / 32768.0;
    assert!(
        (max_abs - expected).abs() < 1e-9,
        "expected normalized peak {expected}, got {max_abs}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn dataset_open_failures_are_classified() {
    let dir = temp_dir("dsfail");

    // 存在しない root → not_found 系。
    let err = DatasetSession::open(dir.join("nonexistent"))
        .err()
        .expect("open must fail");
    assert!(
        err.code() == "not_found" || err.code() == "io",
        "unexpected code {}",
        err.code()
    );

    // manifest.json が破損 → io。
    let corrupt = dir.join("corrupt");
    std::fs::create_dir_all(&corrupt).unwrap();
    std::fs::write(corrupt.join("manifest.json"), b"{ not json").unwrap();
    let err = DatasetSession::open(&corrupt)
        .err()
        .expect("open must fail");
    assert_eq!(err.code(), "io");

    std::fs::remove_dir_all(&dir).ok();
}

/// `JobService::export` の単独 export 経路(`al export` の実体):
/// plan に export が無い job へ record を追加して実行し、完了済み
/// なら再利用、依存 stage 未完了なら失敗する。
#[test]
fn export_runs_export_stage_on_an_existing_job() {
    let dir = temp_dir("export");
    let wav = wav_path(&dir, "cap.wav", 960);
    let service = JobService::new(dir.join("jobs"));

    // dataset までの plan — export record は無い。
    let (job_id, _) = service
        .run(
            run_spec(&wav, Some(vec![StageKind::Ingest, StageKind::Dataset])),
            &RunControl::default(),
        )
        .unwrap();

    let manifest = service.export(&job_id, &RunControl::default()).unwrap();
    let record = manifest.stage(StageKind::Export).expect("export record");
    assert_eq!(record.status, StageStatus::Completed);
    assert!(
        service
            .job_dir(&job_id)
            .join("export/analysis/export_manifest.json")
            .is_file()
    );
    // 2 度目は completed record の再利用 — エラーにならない。
    service.export(&job_id, &RunControl::default()).unwrap();

    // ingest のみの job では Dataset 依存が未完了 → stage 失敗。
    let (partial_id, _) = service
        .run(
            run_spec(&wav, Some(vec![StageKind::Ingest])),
            &RunControl::default(),
        )
        .unwrap();
    assert!(service.export(&partial_id, &RunControl::default()).is_err());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn parse_stage_and_input_spec_round_trip() {
    assert_eq!(parse_stage("ingest").unwrap(), StageKind::Ingest);
    assert_eq!(parse_stage("export").unwrap(), StageKind::Export);
    // #106: the visualization stage is gone.
    assert!(parse_stage("visualization").is_err());
    assert!(parse_stage("nope").is_err());
    let input = parse_input_spec("capture_bug:/tmp/x.wav").unwrap();
    assert_eq!(input.role, al_core::InputRole::CaptureBug);
    assert!(parse_input_spec("badrole:/tmp/x.wav").is_err());
    assert!(parse_input_spec("no-separator").is_err());
}

#[test]
fn assign_input_ids_validates_count_and_uniqueness() {
    let mut inputs = vec![
        parse_input_spec("capture_bug:/a.wav").unwrap(),
        parse_input_spec("source_asset:/b.wav").unwrap(),
    ];
    let ids = vec![al_core::InputId::new(), al_core::InputId::new()];
    JobService::assign_input_ids(&mut inputs, ids.clone()).unwrap();
    assert_eq!(inputs[0].input_id, ids[0]);

    let too_few = vec![al_core::InputId::new()];
    assert!(JobService::assign_input_ids(&mut inputs, too_few).is_err());
    let dup = vec![ids[0].clone(), ids[0].clone()];
    assert!(JobService::assign_input_ids(&mut inputs, dup).is_err());
}

#[test]
fn corrupt_manifest_makes_inspect_fail_not_silent() {
    let dir = temp_dir("corrupt");
    let service = JobService::new(dir.join("jobs"));
    let job_id = JobId::new();
    let job_dir = service.job_dir(&job_id);
    std::fs::create_dir_all(&job_dir).unwrap();
    std::fs::write(job_dir.join("manifest.json"), b"{{{{").unwrap();
    assert!(service.inspect(&job_id).is_err());
    // list には壊れた manifest の job が要約として残る(不可視化しない)。
    let jobs = service.list().unwrap();
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].stages.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

/// `RunControl::on_stage` が進捗として使えることの確認
/// (GUI の JobSpec progress 橋渡しと同じ形状)。
#[test]
fn progress_events_report_index_and_total() {
    let dir = temp_dir("progress");
    let wav = wav_path(&dir, "cap.wav", 960);
    let service = JobService::new(dir.join("jobs"));
    let seen = Rc::new(Cell::new(0u32));
    let seen2 = seen.clone();
    let on_stage = move |ev: al_job::StageEvent| {
        assert_eq!(ev.total, 1);
        seen2.set(seen2.get() + 1);
    };
    let ctl = RunControl {
        on_stage: Some(&on_stage),
        is_cancelled: None,
    };
    service
        .run(run_spec(&wav, Some(vec![StageKind::Ingest])), &ctl)
        .unwrap();
    // Started + Completed の 2 イベント。
    assert_eq!(seen.get(), 2);

    std::fs::remove_dir_all(&dir).ok();
}

#[allow(dead_code)]
fn assert_error_kind(err: &ServiceError, code: &str) {
    assert_eq!(err.code(), code);
}
