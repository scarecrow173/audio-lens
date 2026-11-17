//! `al-gui-real-scenario` test-only bin の headless 検証
//! (Issue #219 — `scarecrow173/ruviform#631` consumer lane)。
//!
//! `gpui-real` feature 有効時のみ compile され、`--blocked-probe`
//! の決定的 JSONL・引数検証・fail-closed 記録形状を検査する。
//! 実 window を開く live lane(`--out` のみ)は interactive desktop
//! session 前提の opt-in 実行(`mise run test-ui-gpui-real`)で
//! あり、ここでは実行しない — headless/CI で window を開かない
//! ことがこの file の契約。

#![cfg(feature = "gpui-real")]

use std::path::PathBuf;
use std::process::Command;

use al_testkit::temp::TempDir;
use serde_json::Value;

/// `gpui-real` feature 指定時に Cargo が供給する test-only bin の
/// 絶対 path。`required-features` を満たすため feature 有効な
/// test build では必ず存在する。
const BIN: &str = env!("CARGO_BIN_EXE_al-gui-real-scenario");

/// catalog の case 数 — bin 内 `CASES` と一致することを
/// blocked-probe 出力で検査する(定数の drift は test で検出)。
const EXPECTED_CASES: usize = 7;

struct RunOutput {
    success: bool,
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run_bin(args: &[&str]) -> RunOutput {
    let output = Command::new(BIN)
        .args(args)
        .output()
        .expect("al-gui-real-scenario binary executes");
    RunOutput {
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn jsonl(path: &PathBuf) -> Vec<Value> {
    std::fs::read_to_string(path)
        .expect("jsonl output readable")
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect()
}

#[test]
fn blocked_probe_emits_fail_closed_notrun_jsonl() {
    let dir = TempDir::new("al219-blocked-probe");
    let out = dir.join("blocked.jsonl");
    let run = run_bin(&["--blocked-probe", "--out", out.to_str().expect("utf8 path")]);
    assert!(
        run.success,
        "blocked-probe must exit 0; stderr: {}",
        run.stderr
    );

    let records = jsonl(&out);
    assert_eq!(records.len(), EXPECTED_CASES + 1, "run header + cases");

    let header = &records[0];
    assert_eq!(header["type"], "run");
    assert_eq!(header["run_id"], "al219-gui-real-e2e");
    assert_eq!(header["lane"], "test-ui-gpui-real-headless");

    for record in &records[1..] {
        assert_eq!(record["type"], "case");
        assert_eq!(record["driver"], "real-os");
        assert_eq!(record["verdict"], "not_run");
        assert_eq!(record["observed"]["observed"], "environment_blocked");
        // blocked 記録は実環境を主張しない — `real-window` を名乗る
        // のは実 session が走った場合のみ。
        assert_ne!(
            record["environment"]["environment_kind"], "real-window",
            "blocked record must not claim real-window: {record}"
        );
        assert!(record["scenario"].is_string());
        assert!(record["case"].is_string());
        assert!(record["required"].is_array());
    }
}

#[test]
fn blocked_probe_stdout_matches_file_output_shape() {
    let run = run_bin(&["--blocked-probe"]);
    assert!(run.success, "stderr: {}", run.stderr);
    let records: Vec<Value> = run
        .stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout line is JSON"))
        .collect();
    assert_eq!(records.len(), EXPECTED_CASES + 1);
    assert!(records[1..].iter().all(|r| r["verdict"] == "not_run"));
}

#[test]
fn unknown_argument_exits_2() {
    let run = run_bin(&["--bogus"]);
    assert_eq!(run.code, Some(2));
    assert!(!run.success);
}

#[test]
fn out_without_path_exits_2() {
    let run = run_bin(&["--out"]);
    assert_eq!(run.code, Some(2));
    assert!(!run.success);
}

#[test]
fn case_catalog_ids_are_stable() {
    // catalog drift 検出: 既知の scenario/case id を pinned する。
    // helper 定数由来の 2 件 + real-input/visual の 5 件。
    let dir = TempDir::new("al219-catalog");
    let out = dir.join("catalog.jsonl");
    let run = run_bin(&["--blocked-probe", "--out", out.to_str().expect("utf8 path")]);
    assert!(run.success);
    let records = jsonl(&out);
    let cases: Vec<(&str, &str)> = records[1..]
        .iter()
        .map(|r| {
            (
                r["scenario"].as_str().expect("scenario"),
                r["case"].as_str().expect("case"),
            )
        })
        .collect();
    assert_eq!(
        cases,
        vec![
            ("al.desktop.jobs", "job-select-resume"),
            ("al.source.seek", "seek-commit"),
            ("al.real-os.input", "os-input-preflight"),
            ("al.real-os.pointer", "seek-pointer-click"),
            ("al.real-os.keyboard", "seek-key-end"),
            ("al.real-os.visual", "capture-frame-desktop"),
            ("al.real-os.visual", "capture-frame-source"),
        ]
    );
}
