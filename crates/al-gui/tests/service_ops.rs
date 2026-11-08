//! `al-gui` の Service op 層を headless 検証する(`UiRuntime` 経由、
//! webview feature 不要)。フロントエンドが発行する intent と同じ
//! `dispatch_op` 経路で、op の契約・エラー分類・`rvf://data` 登録を
//! 確認する。

mod common;

use std::path::PathBuf;
use std::time::Duration;

use al_core::JobId;
use al_gui::AlGuiService;
use al_gui::command;
use ruviform_ui::data::{DataRegistry, DataRequest};
use ruviform_ui::ipc::{Event, JobState, codes};
use ruviform_ui::runtime::UiRuntime;
use serde_json::{Value, json};

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-gui-ops-{tag}-{}", JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn wav_path(dir: &std::path::Path) -> PathBuf {
    let samples: Vec<i16> = (0..48_000).map(|i| (i % 400) as i16 - 200).collect();
    let path = dir.join("cap.wav");
    std::fs::write(&path, al_testkit::media::wav_s16_bytes(1, 48_000, &samples)).unwrap();
    path
}

fn runtime() -> UiRuntime {
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(AlGuiService::new()).unwrap();
    rt
}

/// dispatch → `ok` の data を取り出す。
fn call_ok(rt: &mut UiRuntime, id: &str, op: &str, params: Value) -> Value {
    let events = rt.dispatch_op(id, op, params);
    match events.as_slice() {
        [
            Event::Result {
                ok: true,
                data: Some(data),
                ..
            },
        ] => data.clone(),
        other => panic!("{op} expected result_ok, got {other:?}"),
    }
}

fn call_err(rt: &mut UiRuntime, id: &str, op: &str, params: Value) -> ruviform_ui::ipc::ErrorBody {
    let events = rt.dispatch_op(id, op, params);
    match events.as_slice() {
        [
            Event::Result {
                ok: false,
                error: Some(err),
                ..
            },
        ] => err.clone(),
        other => panic!("{op} expected result_err, got {other:?}"),
    }
}

/// job event が来るまで poll し、終了 state を返す。
fn wait_job(
    rt: &mut UiRuntime,
) -> (
    u64,
    JobState,
    Option<Value>,
    Option<ruviform_ui::ipc::ErrorBody>,
) {
    for _ in 0..600 {
        for ev in rt.poll(Some(Duration::from_millis(50))) {
            if let Event::Job {
                job,
                state,
                data,
                error,
                ..
            } = ev
                && matches!(
                    state,
                    JobState::Finished | JobState::Failed | JobState::Cancelled
                )
            {
                return (job, state, data, error);
            }
        }
    }
    panic!("job did not finish");
}

#[test]
fn capabilities_expose_al_ops() {
    let mut rt = runtime();
    let caps = call_ok(&mut rt, "c1", "sys.capabilities", json!({}));
    let ops: Vec<&str> = caps["ops"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["name"].as_str())
        .collect();
    for expected in [
        "al.app.info",
        "al.job.run",
        "al.job.resume",
        "al.job.inspect",
        "al.job.list",
        "al.ds.open",
        "al.ds.features",
        "al.view.waveform",
        "al.view.spectrogram",
    ] {
        assert!(ops.contains(&expected), "missing op {expected}: {ops:?}");
    }
}

#[test]
fn invalid_params_are_mapped_to_invalid_params_code() {
    let mut rt = runtime();
    // base_dir 未設定 + 必須パラメータ欠落は protocol 的な invalid。
    let err = call_err(&mut rt, "e1", "al.job.list", json!({}));
    assert_eq!(err.code, codes::INVALID_PARAMS);
    let err = call_err(&mut rt, "e2", "al.ds.open", json!({"job_id": "JOB-nope"}));
    assert!(err.code == codes::INVALID_PARAMS || err.code == "not_found");
}

#[test]
fn ds_open_missing_dataset_fails_with_error_result() {
    let mut rt = runtime();
    let err = call_err(
        &mut rt,
        "e1",
        "al.ds.open",
        json!({"root": "C:/definitely/not/here"}),
    );
    assert!(!err.message.is_empty());
}

#[test]
fn job_run_inspect_and_dataset_browse_headless() {
    let dir = temp_dir("e2e");
    let jobs = dir.join("jobs");
    let wav = wav_path(&dir);
    let mut rt = runtime();

    call_ok(
        &mut rt,
        "cfg",
        "al.app.configure",
        json!({"base_dir": jobs.to_string_lossy()}),
    );

    // run: ingest → cheap_analysis → dataset (JobSpec で非同期)。
    let spawn = call_ok(
        &mut rt,
        "run1",
        "al.job.run",
        json!({
            "inputs": [{"role": "capture_bug", "path": wav.to_string_lossy()}],
            "stages": ["ingest", "cheap_analysis", "dataset"],
        }),
    );
    assert!(spawn.get("job").and_then(Value::as_u64).is_some());
    let (_, state, data, error) = wait_job(&mut rt);
    assert_eq!(state, JobState::Finished, "job failed: {error:?}");
    let job_id = data.unwrap()["job_id"].as_str().unwrap().to_string();

    // inspect: manifest + dataset_ready。
    let insp = call_ok(&mut rt, "i1", "al.job.inspect", json!({"job_id": job_id}));
    assert_eq!(insp["dataset_ready"], true);

    // dataset session を開く。
    let opened = call_ok(&mut rt, "d1", "al.ds.open", json!({"job_id": job_id}));
    let session = opened["session"].as_u64().unwrap();

    // overview/features から validity 保持データが取れる。
    let ov = call_ok(&mut rt, "d2", "al.ds.overview", json!({"session": session}));
    assert!(ov["timeline"]["inputs"].as_array().unwrap().len() == 1);
    let feats = call_ok(&mut rt, "d3", "al.ds.features", json!({"session": session}));
    let records = feats["records"].as_array().unwrap();
    assert!(!records.is_empty());
    assert!(records.iter().all(|r| r.get("validity").is_some()));

    // 波形 view → rvf://data 登録済み source が返り、有界 read で読める。
    let input_id = ov["timeline"]["inputs"][0]["input_id"].as_str().unwrap();
    let view = call_ok(
        &mut rt,
        "v1",
        "al.view.waveform",
        json!({
            "session": session,
            "input_id": input_id,
            "channel": 0,
            "start_s": 0.0,
            "end_s": 0.02,
            "bins": 64,
        }),
    );
    let name = view["source"].as_str().unwrap();
    assert_eq!(view["meta"]["format"], "RVBW");
    let chunk = rt
        .data()
        .read(&DataRequest {
            name: name.to_string(),
            offset: 0,
            len: Some(4),
        })
        .unwrap();
    assert_eq!(&chunk.bytes[..], b"RVBW");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn job_cancel_op_is_routed() {
    let mut rt = runtime();
    // 実行中でない job id への cancel は UNKNOWN_JOB。
    let err = call_err(&mut rt, "x1", "job.cancel", json!({"job": 99}));
    assert_eq!(err.code, codes::UNKNOWN_JOB);
}

fn command_params(target: &str, action: &str, params: Value) -> Value {
    json!({
        "command_id": "c",
        "expect_receipt": true,
        "intent": {
            "kind": "action",
            "target": target,
            "control_id": "job-table",
            "sequence": 1,
            "action": action,
            "params": params,
        },
    })
}

fn service_runtime(base: &std::path::Path) -> UiRuntime {
    let mut rt = UiRuntime::new(DataRegistry::new());
    rt.register_service(AlGuiService::with_base_dir(base))
        .unwrap();
    rt
}

#[test]
fn command_select_maps_row_to_job_and_replies_applied() {
    let dir = temp_dir("command-select");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut rt = service_runtime(&base);

    let data = call_ok(
        &mut rt,
        "s1",
        command::COMMAND_INTENT_OP,
        command_params(
            command::JOB_TARGET,
            command::ACTION_SELECT,
            json!({"row": 0}),
        ),
    );
    assert_eq!(data["status"], "applied");
    assert_eq!(data["job_id"], job_id.as_str());

    let err = call_err(
        &mut rt,
        "s2",
        command::COMMAND_INTENT_OP,
        command_params(
            command::JOB_TARGET,
            command::ACTION_SELECT,
            json!({"row": 1}),
        ),
    );
    assert_eq!(err.code, codes::INVALID_PARAMS);
}

#[test]
fn command_resume_starts_the_job_and_replies_accepted() {
    let dir = temp_dir("command-resume");
    let base = dir.join("jobs");
    let job_id = common::make_job(&base, &dir.join("cap.wav"));
    let mut rt = service_runtime(&base);

    let spawn = call_ok(
        &mut rt,
        "r1",
        command::COMMAND_INTENT_OP,
        command_params(
            command::JOB_TARGET,
            command::ACTION_RESUME,
            json!({"job_id": job_id.as_str()}),
        ),
    );
    assert!(spawn.get("job").and_then(Value::as_u64).is_some());
    assert!(spawn.get("status").is_none(), "accepted carries no status");
    let (_, state, data, error) = wait_job(&mut rt);
    assert_eq!(state, JobState::Finished, "resume failed: {error:?}");
    assert_eq!(data.unwrap()["job_id"], job_id.as_str());
}

#[test]
fn command_rejects_unknown_targets_and_actions() {
    let dir = temp_dir("command-reject");
    let base = dir.join("jobs");
    let mut rt = service_runtime(&base);

    let err = call_err(
        &mut rt,
        "x1",
        command::COMMAND_INTENT_OP,
        command_params("al.ds", command::ACTION_SELECT, json!({"row": 0})),
    );
    assert_eq!(err.code, codes::INVALID_PARAMS);
    let err = call_err(
        &mut rt,
        "x2",
        command::COMMAND_INTENT_OP,
        command_params(command::JOB_TARGET, "explode", json!({})),
    );
    assert_eq!(err.code, codes::INVALID_PARAMS);
}
