//! `al-ctl` (Issue #255) の CLI レベル回帰テスト。
//! `CARGO_BIN_EXE_al-ctl` / `CARGO_BIN_EXE_al` の実バイナリ spawn で
//! doctor(read-only 健全性)と launch(`al` spawn + PID 報告)の
//! 外部から観測できる契約を検証する。

use std::path::PathBuf;
use std::process::Command;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-ctl-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// `al-ctl` バイナリへの `Command`。`AL_LOG`/`AL_LOG_FORMAT` は
/// launch 経由で `al` 子プロセスへ継承されるため、親側で除去して
/// logging env の有無で結果が変わらないことを保証する
/// (`tests/cli.rs` の `al_command` と同じ規約)。
fn al_ctl_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_al-ctl"));
    command.env_remove("AL_LOG");
    command.env_remove("AL_LOG_FORMAT");
    command
}

const AL_BIN: &str = env!("CARGO_BIN_EXE_al");

/// `samples/` + `dictionaries/` を持つ最小の偽 repo(root)を作る。
/// git 管理下ではないため `git` check は fail になる — fixture 系
/// check は repo 解決とは独立に評価されることを検証する足場。
fn fake_repo(tag: &str) -> PathBuf {
    let repo = temp_dir(tag);
    let samples = repo.join("samples").join("media");
    std::fs::create_dir_all(&samples).unwrap();
    std::fs::write(samples.join("tone.wav"), b"RIFF-fake-wav-bytes").unwrap();
    let dicts = repo.join("dictionaries");
    std::fs::create_dir_all(&dicts).unwrap();
    std::fs::write(
        dicts.join("capture-artifacts.v1.json"),
        r#"{"version":"1","entries":[]}"#,
    )
    .unwrap();
    repo
}

fn check<'a>(report: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("check {name} missing in {report}"))
}

#[test]
fn help_lists_doctor_and_launch() {
    let out = al_ctl_command().arg("--help").output().unwrap();
    assert!(out.status.success(), "--help failed: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("doctor"), "usage lacks doctor: {stdout}");
    assert!(stdout.contains("launch"), "usage lacks launch: {stdout}");
}

#[test]
fn doctor_json_reports_fixture_health() {
    let repo = fake_repo("doctor");
    let out = al_ctl_command()
        .args([
            "doctor",
            "--repo",
            repo.to_str().unwrap(),
            "--al-path",
            AL_BIN,
            "--json",
        ])
        .output()
        .unwrap();
    // 偽 repo は git 管理下ではないため git check が fail → 全体 fail。
    assert_eq!(
        out.status.code(),
        Some(1),
        "doctor should exit 1 with a failed check: {out:?}"
    );
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("doctor --json emits JSON");
    assert_eq!(report["command"], "doctor");
    assert_eq!(report["ok"], false);
    assert_eq!(report["version"], env!("CARGO_PKG_VERSION"));

    assert_eq!(check(&report, "fixtures.samples")["status"], "ok");
    assert_eq!(check(&report, "fixtures.samples")["detail"]["files"], 1);
    assert_eq!(check(&report, "fixtures.dictionaries")["status"], "ok");
    assert_eq!(
        check(&report, "fixtures.dictionaries")["detail"]["entries"],
        0
    );
    assert_eq!(check(&report, "git")["status"], "fail");
    assert_eq!(check(&report, "base_dir")["status"], "skipped");
    assert_eq!(check(&report, "al_binary")["status"], "ok");
    assert_eq!(check(&report, "al_binary")["detail"]["source"], "explicit");
    assert_eq!(check(&report, "al_binary")["detail"]["usage_seen"], true);
}

#[test]
fn doctor_flags_unmaterialized_lfs_pointer() {
    let repo = fake_repo("lfs");
    let pointer = repo.join("samples").join("media").join("clip.mp4");
    std::fs::write(
        &pointer,
        "version https://git-lfs.github.com/spec/v1\n\
         oid sha256:0000000000000000000000000000000000000000000000000000000000000000\n\
         size 12345\n",
    )
    .unwrap();
    let out = al_ctl_command()
        .args(["doctor", "--repo", repo.to_str().unwrap(), "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let samples = check(&report, "fixtures.samples");
    assert_eq!(samples["status"], "fail");
    let pointers = samples["detail"]["pointers"].as_array().unwrap();
    assert!(
        pointers
            .iter()
            .any(|p| p.as_str().unwrap().ends_with("clip.mp4")),
        "pointer file not reported: {samples}"
    );
    assert!(
        samples["guidance"]
            .as_str()
            .unwrap()
            .contains("git lfs pull"),
        "guidance should name the fix: {samples}"
    );
}

#[test]
fn doctor_base_dir_reports_job_dirs() {
    let repo = fake_repo("basedir");
    let base = temp_dir("basedir-jobs");
    std::fs::create_dir_all(base.join("JOB-0000000000000001")).unwrap();
    std::fs::write(base.join("note.txt"), b"x").unwrap();
    let out = al_ctl_command()
        .args([
            "doctor",
            "--repo",
            repo.to_str().unwrap(),
            "--base-dir",
            base.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let base_dir = check(&report, "base_dir");
    assert_eq!(base_dir["status"], "ok");
    assert_eq!(base_dir["detail"]["job_dirs"], 1);
    assert_eq!(base_dir["detail"]["entries"], 2);
}

#[test]
fn launch_dry_run_does_not_spawn() {
    let out = al_ctl_command()
        .args([
            "launch",
            "--al-path",
            AL_BIN,
            "--dry-run",
            "--json",
            "--",
            "run",
            "--base-dir",
            "unused",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "dry-run failed: {out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["spawned"], false);
    assert_eq!(report["source"], "explicit");
    assert_eq!(report["args"][0], "run");
    // base-dir "unused" は spawn されないので生成されない。
    assert!(!PathBuf::from("unused").exists());
}

#[test]
fn launch_spawns_al_and_reports_process_identity() {
    let out = al_ctl_command()
        .args(["launch", "--al-path", AL_BIN, "--json", "--", "--help"])
        .output()
        .unwrap();
    assert!(out.status.success(), "launch failed: {out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["spawned"], true);
    assert_eq!(report["exit_code"], 0);
    assert!(report["pid"].as_u64().unwrap() > 0, "pid not reported");
    assert!(
        report["stdout"]
            .as_str()
            .unwrap()
            .contains("AudioLens job runner"),
        "launched `al --help` stdout not captured: {report}"
    );
}

#[test]
fn launch_requires_al_args_after_separator() {
    let out = al_ctl_command().args(["launch"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("after `--`"),
        "error should name the fix: {stderr}"
    );
}

#[test]
fn launch_errors_when_al_path_missing() {
    let out = al_ctl_command()
        .args([
            "launch",
            "--al-path",
            "definitely-not-a-real-al-binary.exe",
            "--",
            "--help",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("mise run build al-cli"),
        "error should name the fix: {stderr}"
    );
}
