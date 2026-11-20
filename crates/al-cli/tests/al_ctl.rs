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
fn help_lists_doctor_launch_drive_and_evidence() {
    let out = al_ctl_command().arg("--help").output().unwrap();
    assert!(out.status.success(), "--help failed: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for sub in ["doctor", "launch", "drive", "evidence"] {
        assert!(stdout.contains(sub), "usage lacks {sub}: {stdout}");
    }
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

// ------------------------------------------------------------------
// drive
// ------------------------------------------------------------------

/// `crates/al-gui/tests/scenario_real.rs` の
/// `case_catalog_ids_are_stable` が pin する (scenario, case) 組を
/// repo 内 file から読む — al-ctl の静的 catalog の drift 検査正本。
fn pinned_scenario_cases() -> Vec<(String, String)> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../al-gui/tests/scenario_real.rs");
    let text = std::fs::read_to_string(&path).expect("scenario_real.rs readable");
    let start = text
        .find("fn case_catalog_ids_are_stable")
        .expect("catalog pin test present");
    let mut rest = &text[start..];
    let mut pairs = Vec::new();
    while let Some(open) = rest.find("(\"al.") {
        let after = &rest[open + 2..];
        let Some(end) = after.find('"') else { break };
        let scenario = after[..end].to_owned();
        let tail = &after[end + 1..];
        let Some(case) = tail.strip_prefix(", \"") else {
            break;
        };
        let Some(cend) = case.find('"') else { break };
        pairs.push((scenario, case[..cend].to_owned()));
        rest = &case[cend + 1..];
    }
    assert_eq!(
        pairs.len(),
        9,
        "expected 9 pinned cases in scenario_real.rs: {pairs:?}"
    );
    pairs
}

#[test]
fn drive_list_matches_pinned_scenario_catalog() {
    let out = al_ctl_command()
        .args(["drive", "--list", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "drive --list failed: {out:?}");
    let report: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("drive --list emits JSON");
    assert_eq!(report["command"], "drive");
    let listed: Vec<(String, String)> = report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["scenario"].as_str().unwrap().to_owned(),
                c["case"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        pinned_scenario_cases(),
        "al-ctl static catalog drifted from scenario_real.rs pins"
    );
}

#[test]
fn drive_list_filters_exist_in_lane_test_sources() {
    // `drive scenario` が使う filter 文字列は lane 側 test 名の
    // substring — 実在しなければ lane は何も実行しない。catalog と
    // 同じく repo 内 file を読んで drift を検査する。
    let out = al_ctl_command()
        .args(["drive", "--list", "--json"])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let gui_tests = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../al-gui/tests");
    let fake_src =
        std::fs::read_to_string(gui_tests.join("scenario.rs")).expect("scenario.rs readable");
    let gpui_src = std::fs::read_to_string(gui_tests.join("scenario_gpui.rs"))
        .expect("scenario_gpui.rs readable");
    for case in report["cases"].as_array().unwrap() {
        let scenario = case["scenario"].as_str().unwrap();
        if let Some(filter) = case["fake_filter"].as_str() {
            assert!(
                fake_src.contains(&format!("fn {filter}")),
                "{scenario}: fake filter {filter} has no test fn in tests/scenario.rs"
            );
        }
        if let Some(filter) = case["gpui_filter"].as_str() {
            assert!(
                gpui_src.contains(&format!("fn {filter}")),
                "{scenario}: gpui filter {filter} has no test fn in tests/scenario_gpui.rs"
            );
        }
    }
}

#[test]
fn drive_scenario_dry_run_resolves_fake_lane() {
    let out = al_ctl_command()
        .args([
            "drive",
            "scenario",
            "al.desktop.jobs",
            "--driver",
            "fake",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "dry-run failed: {out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["spawned"], false);
    assert_eq!(report["dry_run"], true);
    assert_eq!(report["driver"], "fake");
    assert_eq!(report["scenario"], "al.desktop.jobs");
    let argv: Vec<&str> = report["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert_eq!(
        argv[..7],
        [
            "cargo", "test", "--locked", "-p", "al-gui", "--test", "scenario"
        ],
        "fake lane argv: {argv:?}"
    );
    assert_eq!(argv[7..], ["--", "fake_desktop_jobs"]);
}

#[test]
fn drive_rejects_per_case_real_os_with_guidance() {
    // real-os lane は catalog 全体実行 — per-scenario filter はない。
    let out = al_ctl_command()
        .args([
            "drive",
            "scenario",
            "al.desktop.jobs",
            "--driver",
            "real-os",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("whole catalog"),
        "error should explain real-os has no per-scenario filter: {stderr}"
    );
}

#[test]
fn drive_rejects_fake_lane_for_real_os_only_scenario() {
    let out = al_ctl_command()
        .args(["drive", "scenario", "al.real-os.input", "--driver", "fake"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("real-os"),
        "error should name the driver that can run it: {stderr}"
    );
}

#[test]
fn drive_rejects_unknown_scenario() {
    let out = al_ctl_command()
        .args(["drive", "scenario", "al.bogus", "--driver", "fake"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--list"),
        "error should point at the catalog listing: {stderr}"
    );
}

#[test]
fn drive_rejects_trailing_separator_args() {
    let out = al_ctl_command()
        .args(["drive", "--driver", "fake", "--", "--out", "x"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

// ------------------------------------------------------------------
// evidence
// ------------------------------------------------------------------

#[test]
fn evidence_scaffold_and_append_roundtrip() {
    let repo = temp_dir("evidence");
    let issue = "777";
    let ledger_dir = repo.join("evidence").join(format!("al{issue}"));
    let actions = ledger_dir.join("actions.jsonl");

    // scaffold: dir + actions.jsonl + verification.md を作る。
    let out = al_ctl_command()
        .args([
            "evidence",
            "scaffold",
            "--repo",
            repo.to_str().unwrap(),
            "--issue",
            issue,
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "scaffold failed: {out:?}");
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["subcommand"], "scaffold");
    assert!(actions.is_file(), "actions.jsonl not created");
    assert!(ledger_dir.join("verification.md").is_file());
    let statuses: Vec<&str> = report["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["created", "created"]);

    // 再 scaffold は既存 file を上書きしない。
    let out = al_ctl_command()
        .args([
            "evidence",
            "scaffold",
            "--repo",
            repo.to_str().unwrap(),
            "--issue",
            issue,
            "--json",
        ])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let statuses: Vec<&str> = report["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["exists", "exists"]);

    // append: 1 行 1 record の JSONL 追記。
    for action in ["probe-a", "probe-b"] {
        let out = al_ctl_command()
            .args([
                "evidence",
                "append",
                "--repo",
                repo.to_str().unwrap(),
                "--issue",
                issue,
                "--action",
                action,
                "--command",
                "al-ctl doctor --json",
                "--exit-code",
                "0",
                "--observed",
                "ok:true",
                "--json",
            ])
            .output()
            .unwrap();
        assert!(out.status.success(), "append failed: {out:?}");
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(report["appended"], true);
        assert_eq!(report["record"]["action"], action);
    }
    let lines: Vec<serde_json::Value> = std::fs::read_to_string(&actions)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("each line is JSON"))
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0]["action"], "probe-a");
    assert_eq!(lines[0]["command"], "al-ctl doctor --json");
    assert_eq!(lines[0]["exit_code"], 0);
    assert_eq!(lines[0]["observed"], "ok:true");
    assert_eq!(lines[1]["action"], "probe-b");
}

#[test]
fn evidence_append_fails_without_scaffold() {
    let repo = temp_dir("evidence-missing");
    let out = al_ctl_command()
        .args([
            "evidence",
            "append",
            "--repo",
            repo.to_str().unwrap(),
            "--issue",
            "778",
            "--action",
            "probe",
            "--command",
            "x",
            "--exit-code",
            "0",
            "--observed",
            "y",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("evidence scaffold"),
        "error should name the fix: {stderr}"
    );
}

#[test]
fn evidence_scaffold_dry_run_writes_nothing() {
    let repo = temp_dir("evidence-dry");
    let out = al_ctl_command()
        .args([
            "evidence",
            "scaffold",
            "--repo",
            repo.to_str().unwrap(),
            "--issue",
            "779",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["dry_run"], true);
    assert!(
        !repo.join("evidence").join("al779").exists(),
        "dry-run must not create the ledger dir"
    );
}

#[test]
fn evidence_rejects_non_numeric_issue() {
    let out = al_ctl_command()
        .args(["evidence", "scaffold", "--issue", "abc"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("digits"),
        "error should name digits: {stderr}"
    );
}

#[test]
fn evidence_append_rejects_missing_field() {
    let repo = temp_dir("evidence-fields");
    let out = al_ctl_command()
        .args([
            "evidence",
            "append",
            "--repo",
            repo.to_str().unwrap(),
            "--issue",
            "780",
            "--action",
            "probe",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--command"),
        "error should name the missing field: {stderr}"
    );
}
