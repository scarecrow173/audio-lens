//! `al-ctl` — AudioLens verification control CLI (Issue #255).
//!
//! Thin control surface for the `verify-audio-lens` skill: a read-only
//! `doctor` that reports environment/fixture/binary health, a `launch`
//! entry that spawns the product `al` binary and reports the spawned
//! process identity (PID) plus captured output, a `drive` entry that
//! resolves and spawns GUI scenario lanes (`cargo test`/`cargo run` on
//! `al-gui`) as child processes, and `evidence` ledger scaffold/append.
//!
//! Headless boundary: std + serde_json only — this bin must not pull
//! GUI/GPU/WebView dependencies (`mise run check-cli-headless`). `drive`
//! keeps that boundary by spawning the lanes as subprocesses instead of
//! linking `al-gui`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde_json::{Value, json};

const USAGE: &str = "\
al-ctl — AudioLens verification control CLI

USAGE:
  al-ctl doctor [--repo <dir>] [--base-dir <dir>] [--al-path <path>]
                [--json]
  al-ctl launch [--repo <dir>] [--al-path <path>] [--dry-run] [--json]
                -- <args for al>
  al-ctl drive --list [--json]
  al-ctl drive [scenario] <scenario-id> --driver <fake|gpui>
                [--repo <dir>] [--dry-run] [--json]
  al-ctl drive --driver <fake|gpui|real-os> [--repo <dir>]
                [--blocked-probe] [--out <path>] [--dry-run] [--json]
  al-ctl evidence scaffold --issue <N> [--repo <dir>] [--dry-run]
                [--json]
  al-ctl evidence append --issue <N> --action <name> --command <cmd>
                --exit-code <n> --observed <text> [--repo <dir>]
                [--dry-run] [--json]
  al-ctl --help | -h

SUBCOMMANDS:
  doctor   Read-only health report: tool version, git revision,
           samples/ LFS materialization, dictionaries/, base_dir
           state, and the resolved `al` binary. Exit 0 when every
           check is ok or skipped, 1 on any failed check.
  launch   Spawn the product `al` binary with the arguments after
           `--`, wait for it, and report the spawned PID, exit code,
           and captured stdout/stderr. `--dry-run` prints the resolved
           command without spawning.
  drive    Resolve a GUI scenario lane to a `cargo` invocation and
           spawn it as a child process (the subprocess boundary keeps
           GUI crates out of al-ctl). `--list` prints the pinned
           scenario catalog. Drivers: fake (`cargo test -p al-gui
           --test scenario`), gpui (`--features gpui-test --test
           scenario_gpui`), real-os (`--features gpui-real --bin
           al-gui-real-scenario`; always drives the whole catalog in
           one session — no per-scenario filter; `--blocked-probe`
           and `--out` apply to it). `scenario <id>` or `--scenario
           <id>` restricts a fake/gpui lane to that scenario's test
           filter. Receipt reports the spawned PID, exit code,
           captured stdout/stderr, and the real-os `--out` path.
  evidence Manage `evidence/al<N>/` ledgers. `scaffold` creates the
           ledger directory plus `actions.jsonl`/`verification.md`
           seeds (existing files are left untouched); `append` writes
           one {action, command, exit_code, observed} JSON line to
           `actions.jsonl`.

OPTIONS:
  --repo <dir>     Repository root to inspect / run lanes in. Default:
                   discovered via `git rev-parse --show-toplevel` from
                   the current directory.
  --base-dir <dir> Job data dir to report on (the `--base-dir` used by
                   `al run`). Optional; skipped when not given.
  --al-path <path> Explicit path to the `al` binary. Default: sibling
                   of the al-ctl executable, then <repo>/target/debug,
                   then <repo>/target/release, then PATH.
  --driver <name>  (drive) Lane driver: fake | gpui | real-os.
  --scenario <id>  (drive) Restrict a fake/gpui lane to one scenario's
                   test filter. See `al-ctl drive --list` for ids.
  --blocked-probe  (drive --driver real-os) Deterministic headless
                   self-check; no window is opened.
  --out <path>     (drive --driver real-os) JSONL report destination.
  --issue <N>      (evidence) Issue number — ledger dir is
                   `evidence/al<N>/`.
  --action <name>  (evidence append) Record `action` field.
  --command <cmd>  (evidence append) Record `command` field.
  --exit-code <n>  (evidence append) Record `exit_code` field.
  --observed <txt> (evidence append) Record `observed` field.
  --json           Emit a single JSON object instead of text lines.
  --dry-run        (launch/drive/evidence) resolve and print only —
                   no spawn, no write.
  --help, -h       Show this text.

EXIT CODES:
  0  success (doctor: all checks ok/skipped; launch/drive: child
     exited 0; evidence: write applied)
  1  doctor found a failed check, a spawned child exited non-zero,
     or an evidence write could not be applied
  2  usage error or unresolvable binary/command
";

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}

fn parse_flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn flag_present(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

// ------------------------------------------------------------------
// Repository / binary resolution
// ------------------------------------------------------------------

/// `git` を `dir` で実行し、成功時の stdout(trim 済み)を返す。
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// doctor/launch が共通して使う検査対象の repo root。
/// `--repo` があればそれを起点に、無ければ cwd から、
/// `git rev-parse --show-toplevel` で正規化する。git が解けない
/// (非 repo 指定・cwd 不在)ときは起点をそのまま返す —
/// 各 fixture check がその location について正直に報告する。
fn repo_root(args: &[String]) -> (PathBuf, Option<String>) {
    let start = parse_flag(args, "--repo")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    match git(&start, &["rev-parse", "--show-toplevel"]) {
        Ok(root) => (PathBuf::from(root), None),
        Err(e) => (start, Some(e)),
    }
}

/// `al` binary の解決先。`Bare` は PATH 解決に委ねる(`al` 名のみ)。
enum AlBin {
    Explicit(PathBuf),
    Sibling(PathBuf),
    RepoPath(PathBuf),
    OnPath,
}

impl AlBin {
    /// Command::new に渡す名前・パスと、人間向けの表示文字列。
    fn program(&self) -> (String, String) {
        match self {
            Self::Explicit(p) | Self::Sibling(p) | Self::RepoPath(p) => {
                (p.display().to_string(), p.display().to_string())
            }
            Self::OnPath => ("al".to_owned(), "al (PATH)".to_owned()),
        }
    }

    fn source(&self) -> &'static str {
        match self {
            Self::Explicit(_) => "explicit",
            Self::Sibling(_) => "sibling",
            Self::RepoPath(_) => "repo",
            Self::OnPath => "path",
        }
    }
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

/// `al` の解決順: --al-path > al-ctl 同階層 > <repo>/target/debug >
/// <repo>/target/release > PATH。実在しない explicit path はエラー。
fn resolve_al(args: &[String], repo: &Path) -> Result<AlBin, String> {
    if let Some(path) = parse_flag(args, "--al-path") {
        let path = PathBuf::from(path);
        if !path.is_file() {
            return Err(format!(
                "--al-path {} does not exist; build `al` with \
                 `mise run build al-cli` or fix the path",
                path.display()
            ));
        }
        return Ok(AlBin::Explicit(path));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let candidate = dir.join(exe_name("al"));
        if candidate.is_file() {
            return Ok(AlBin::Sibling(candidate));
        }
    }
    for profile in ["debug", "release"] {
        let candidate = repo.join("target").join(profile).join(exe_name("al"));
        if candidate.is_file() {
            return Ok(AlBin::RepoPath(candidate));
        }
    }
    Ok(AlBin::OnPath)
}

// ------------------------------------------------------------------
// doctor
// ------------------------------------------------------------------

/// 1 件の doctor 観測。`status` は ok / fail / skipped。
struct Check {
    name: &'static str,
    status: &'static str,
    detail: Value,
    guidance: Option<String>,
}

impl Check {
    fn new(name: &'static str, status: &'static str, detail: Value) -> Self {
        Self {
            name,
            status,
            detail,
            guidance: None,
        }
    }

    fn fail(name: &'static str, detail: Value, guidance: impl Into<String>) -> Self {
        Self {
            name,
            status: "fail",
            detail,
            guidance: Some(guidance.into()),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "status": self.status,
            "detail": self.detail,
            "guidance": self.guidance,
        })
    }
}

/// `path` が LFS ポインタ(未実体化)かを先頭バイトで判定する。
/// 実体化済みメディアはこのマジックを先頭に持たない。
fn is_lfs_pointer(path: &Path) -> bool {
    const POINTER_MAGIC: &[u8] = b"version https://git-lfs";
    let mut head = [0u8; 64];
    let read = std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
        .unwrap_or(0);
    head[..read].starts_with(POINTER_MAGIC)
}

/// `samples/` 配下の通常ファイルを走査し、LFS ポインタ数を数える。
/// `_runs/`(生成物置き場)は対象外。
fn check_samples(samples: &Path) -> Check {
    if !samples.is_dir() {
        return Check::fail(
            "fixtures.samples",
            json!({ "dir": samples.display().to_string(), "exists": false }),
            format!(
                "samples/ not found at {}; run inside the audio-lens \
                 checkout or pass --repo <dir>",
                samples.display()
            ),
        );
    }
    let mut files = 0u64;
    let mut pointers: Vec<String> = Vec::new();
    let mut stack = vec![samples.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if entry.file_name() != "_runs" {
                    stack.push(path);
                }
            } else {
                files += 1;
                if is_lfs_pointer(&path) {
                    pointers.push(path.display().to_string());
                }
            }
        }
    }
    let detail = json!({
        "dir": samples.display().to_string(),
        "files": files,
        "materialized": files - pointers.len() as u64,
        "pointers": pointers,
    });
    if pointers.is_empty() {
        Check::new("fixtures.samples", "ok", detail)
    } else {
        Check::fail(
            "fixtures.samples",
            detail,
            "LFS objects are not materialized; run `mise run setup` or \
             `git lfs pull` before driving real media",
        )
    }
}

fn check_dictionaries(repo: &Path) -> Check {
    let path = repo.join("dictionaries").join("capture-artifacts.v1.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            return Check::fail(
                "fixtures.dictionaries",
                json!({ "path": path.display().to_string(), "error": e.to_string() }),
                format!("capture-artifact dictionary missing at {}", path.display()),
            );
        }
    };
    let parsed: Result<Value, _> = serde_json::from_str(&text);
    match parsed {
        Ok(value) => {
            let entries = value
                .get("entries")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let versioned = value.get("version").is_some();
            Check::new(
                "fixtures.dictionaries",
                if versioned { "ok" } else { "fail" },
                json!({
                    "path": path.display().to_string(),
                    "parsed": true,
                    "version": value.get("version").cloned().unwrap_or(Value::Null),
                    "entries": entries,
                }),
            )
        }
        Err(e) => Check::fail(
            "fixtures.dictionaries",
            json!({ "path": path.display().to_string(), "parsed": false, "error": e.to_string() }),
            format!("dictionary at {} is not valid JSON", path.display()),
        ),
    }
}

fn check_git(repo: &Path) -> Check {
    match git(repo, &["rev-parse", "HEAD"]) {
        Ok(sha) => {
            let dirty = git(repo, &["status", "--porcelain"])
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            Check::new(
                "git",
                "ok",
                json!({ "sha": sha, "dirty": dirty, "dir": repo.display().to_string() }),
            )
        }
        Err(e) => Check::fail(
            "git",
            json!({ "dir": repo.display().to_string(), "error": e }),
            "git revision unavailable; run inside the audio-lens worktree \
             or pass --repo <dir>",
        ),
    }
}

fn check_base_dir(args: &[String]) -> Check {
    let Some(raw) = parse_flag(args, "--base-dir") else {
        return Check::new(
            "base_dir",
            "skipped",
            json!({ "reason": "pass --base-dir <dir> to check a job data dir" }),
        );
    };
    let dir = PathBuf::from(raw);
    let meta = std::fs::metadata(&dir);
    match meta {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Check::new(
            "base_dir",
            "ok",
            json!({
                "path": dir.display().to_string(),
                "exists": false,
                "note": "absent — `al run` creates it on first job",
            }),
        ),
        Err(e) => Check::fail(
            "base_dir",
            json!({ "path": dir.display().to_string(), "error": e.to_string() }),
            format!("cannot stat {}", dir.display()),
        ),
        Ok(meta) if !meta.is_dir() => Check::fail(
            "base_dir",
            json!({ "path": dir.display().to_string(), "is_dir": false }),
            format!("{} exists but is not a directory", dir.display()),
        ),
        Ok(_) => {
            let mut entries = 0u64;
            let mut job_dirs = 0u64;
            if let Ok(read) = std::fs::read_dir(&dir) {
                for entry in read.flatten() {
                    entries += 1;
                    if entry.file_name().to_string_lossy().starts_with("JOB-")
                        && entry.path().is_dir()
                    {
                        job_dirs += 1;
                    }
                }
            }
            Check::new(
                "base_dir",
                "ok",
                json!({
                    "path": dir.display().to_string(),
                    "exists": true,
                    "is_dir": true,
                    "entries": entries,
                    "job_dirs": job_dirs,
                }),
            )
        }
    }
}

/// `al` バイナリを解決し `--help` で probe する(read-only: usage を
/// 表示して終了する無副作用呼出し)。
fn check_al_binary(args: &[String], repo: &Path) -> Check {
    let bin = match resolve_al(args, repo) {
        Ok(bin) => bin,
        Err(e) => {
            return Check::fail("al_binary", json!({ "resolved": Value::Null }), e);
        }
    };
    let (program, display) = bin.program();
    let probe = Command::new(&program).arg("--help").output();
    match probe {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let usage_seen = stdout.contains("AudioLens job runner");
            let ok = out.status.success() && usage_seen;
            Check::new(
                "al_binary",
                if ok { "ok" } else { "fail" },
                json!({
                    "path": display,
                    "source": bin.source(),
                    "probe_exit": out.status.code(),
                    "usage_seen": usage_seen,
                }),
            )
        }
        Err(e) => Check::fail(
            "al_binary",
            json!({ "path": display, "source": bin.source(), "error": e.to_string() }),
            format!(
                "cannot run `{display} --help`; build `al` with \
                 `mise run build al-cli`"
            ),
        ),
    }
}

fn cmd_doctor(args: &[String]) -> ExitCode {
    let (repo, discover_error) = repo_root(args);
    let json_out = flag_present(args, "--json");

    let mut checks = vec![
        Check::new(
            "environment",
            "ok",
            json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "pid": std::process::id(),
            }),
        ),
        Check::new(
            "tool",
            "ok",
            json!({ "name": "al-ctl", "version": env!("CARGO_PKG_VERSION") }),
        ),
        check_git(&repo),
    ];
    if let Some(e) = discover_error {
        checks[2].detail["repo_discovery"] = json!(e);
    }
    checks.push(check_samples(&repo.join("samples")));
    checks.push(check_dictionaries(&repo));
    checks.push(check_base_dir(args));
    checks.push(check_al_binary(args, &repo));

    let ok = checks.iter().all(|c| c.status != "fail");
    if json_out {
        let report = json!({
            "tool": "al-ctl",
            "version": env!("CARGO_PKG_VERSION"),
            "command": "doctor",
            "ok": ok,
            "repo": repo.display().to_string(),
            "checks": checks.iter().map(Check::to_json).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("al-ctl doctor (repo: {})", repo.display());
        for check in &checks {
            let detail = serde_json::to_string(&check.detail).unwrap();
            println!("check {}: {} {}", check.name, check.status, detail);
            if let Some(guidance) = &check.guidance {
                println!("  -> {guidance}");
            }
        }
        println!("ok: {ok}");
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

// ------------------------------------------------------------------
// launch
// ------------------------------------------------------------------

fn cmd_launch(args: &[String], al_args: &[String]) -> ExitCode {
    if al_args.is_empty() {
        return fail(
            "launch requires `al` arguments after `--` (e.g. \
             `al-ctl launch -- run --base-dir <dir> --input <role>:<path>`)",
        );
    }
    let (repo, _) = repo_root(args);
    let json_out = flag_present(args, "--json");
    let dry_run = flag_present(args, "--dry-run");

    let bin = match resolve_al(args, &repo) {
        Ok(bin) => bin,
        Err(e) => return fail(e),
    };
    let (program, display) = bin.program();

    if dry_run {
        let report = json!({
            "command": "launch",
            "dry_run": true,
            "spawned": false,
            "al": display,
            "source": bin.source(),
            "args": al_args,
        });
        if json_out {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!("dry-run: would spawn {display} {}", al_args.join(" "));
        }
        return ExitCode::SUCCESS;
    }

    let spawned = Command::new(&program)
        .args(al_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let child = match spawned {
        Ok(child) => child,
        Err(e) => {
            return fail(format!(
                "cannot spawn `{display}`: {e}; build `al` with \
                 `mise run build al-cli` or pass --al-path <path>"
            ));
        }
    };
    let pid = child.id();
    let out = match child.wait_with_output() {
        Ok(out) => out,
        Err(e) => return fail(format!("`{display}` did not complete: {e}")),
    };
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let exit_code = out.status.code().unwrap_or(-1);
    let report = json!({
        "command": "launch",
        "dry_run": false,
        "spawned": true,
        "al": display,
        "source": bin.source(),
        "args": al_args,
        "pid": pid,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
    });
    if json_out {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        print!("{stdout}");
        eprint!("{stderr}");
        println!("launch: pid={pid} exit={exit_code} al={display}");
    }
    if out.status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

// ------------------------------------------------------------------
// drive
// ------------------------------------------------------------------

/// scenario lane の driver。`fake`/`gpui` は `cargo test` の scenario
/// lane、`real-os` は test-only bin `al-gui-real-scenario` の lane。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Driver {
    Fake,
    Gpui,
    RealOs,
}

impl Driver {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "fake" => Some(Self::Fake),
            "gpui" => Some(Self::Gpui),
            "real-os" => Some(Self::RealOs),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Fake => "fake",
            Self::Gpui => "gpui",
            Self::RealOs => "real-os",
        }
    }
}

/// 静的 scenario catalog の 1 行。
///
/// 正本は `crates/al-gui/src/bin/al-gui-real-scenario.rs` の `CASES`
/// (実行順)と `crates/al-gui/tests/scenario_real.rs` の
/// `case_catalog_ids_are_stable` が pin する (scenario, case) 組。
/// `tests/al_ctl.rs` がこの表と pin の一致、および各 filter 文字列が
/// lane 側 test 源に実在することを検査する — catalog 意味論を変える
/// 更新では正本側を先に直し、本表を追従させる。`required` は
/// `LaneCase.required` の写し(表示用)。
struct CatalogCase {
    scenario: &'static str,
    case: &'static str,
    /// `tests/scenario.rs`(fake lane)内の test 名 filter。
    /// `None` は fake lane で走らない(real-os 専用 case)。
    fake_filter: Option<&'static str>,
    /// `tests/scenario_gpui.rs`(gpui lane)内の test 名 filter。
    gpui_filter: Option<&'static str>,
    required: &'static [&'static str],
}

impl CatalogCase {
    /// この case が実行可能な driver。real-os lane は catalog 全体を
    /// 1 session で実行するため全 case に含まれる。
    fn drivers(&self) -> Vec<Driver> {
        let mut drivers = Vec::with_capacity(3);
        if self.fake_filter.is_some() {
            drivers.push(Driver::Fake);
        }
        if self.gpui_filter.is_some() {
            drivers.push(Driver::Gpui);
        }
        drivers.push(Driver::RealOs);
        drivers
    }
}

/// 実行順の静的 catalog。正本の写し(出典は [`CatalogCase`])。
const CATALOG: &[CatalogCase] = &[
    CatalogCase {
        scenario: "al.desktop.jobs",
        case: "job-select-resume",
        fake_filter: Some("fake_desktop_jobs"),
        gpui_filter: Some("gpui_desktop_jobs"),
        required: &["SemanticAction"],
    },
    CatalogCase {
        scenario: "al.source.seek",
        case: "seek-commit",
        fake_filter: Some("fake_source_seek"),
        gpui_filter: Some("gpui_and_fake_run_the_same_scenarios"),
        required: &["SemanticAction"],
    },
    CatalogCase {
        scenario: "al.dataset.workspace",
        case: "workspace-tabs",
        fake_filter: Some("fake_dataset_workspace"),
        gpui_filter: Some("gpui_dataset_workspace"),
        required: &["SemanticAction"],
    },
    CatalogCase {
        scenario: "al.real-os.input",
        case: "os-input-preflight",
        fake_filter: None,
        gpui_filter: None,
        required: &["RealOsPointerOrKeyboard"],
    },
    CatalogCase {
        scenario: "al.real-os.pointer",
        case: "seek-pointer-click",
        fake_filter: None,
        gpui_filter: None,
        required: &["RealOsPointerOrKeyboard"],
    },
    CatalogCase {
        scenario: "al.real-os.keyboard",
        case: "seek-key-end",
        fake_filter: None,
        gpui_filter: None,
        required: &["RealOsPointerOrKeyboard"],
    },
    CatalogCase {
        scenario: "al.real-os.visual",
        case: "capture-frame-desktop",
        fake_filter: None,
        gpui_filter: None,
        required: &["VisualCapture"],
    },
    CatalogCase {
        scenario: "al.real-os.visual",
        case: "capture-frame-source",
        fake_filter: None,
        gpui_filter: None,
        required: &["VisualCapture"],
    },
    CatalogCase {
        scenario: "al.real-os.window",
        case: "close-veto-approve",
        fake_filter: None,
        gpui_filter: None,
        required: &[],
    },
];

/// `cargo` へ渡す解決済み lane コマンド。
struct LaneCommand {
    /// argv[0] は `cargo`(PATH 解決)。
    argv: Vec<String>,
    /// receipt に載せる lane 表示名。
    lane: &'static str,
}

/// driver + scenario 指定から spawn する cargo argv を組み立てる。
/// lane 形状は mise task と同一(`mise.toml` の `test`/`test-ui-gpui`/
/// `test-ui-gpui-real(-headless)`)— 違いは al-ctl が cargo を子
/// プロセスとして直接呼ぶ点だけ。
fn resolve_lane(
    driver: Driver,
    scenario: Option<&str>,
    blocked_probe: bool,
    out: Option<&str>,
) -> Result<LaneCommand, String> {
    match driver {
        Driver::Fake | Driver::Gpui => {
            let mut argv = vec![
                "cargo".to_owned(),
                "test".to_owned(),
                "--locked".to_owned(),
                "-p".to_owned(),
                "al-gui".to_owned(),
            ];
            let lane = if driver == Driver::Gpui {
                argv.extend(["--features".to_owned(), "gpui-test".to_owned()]);
                argv.extend(["--test".to_owned(), "scenario_gpui".to_owned()]);
                "cargo test -p al-gui --features gpui-test --test scenario_gpui"
            } else {
                argv.extend(["--test".to_owned(), "scenario".to_owned()]);
                "cargo test -p al-gui --test scenario"
            };
            if let Some(id) = scenario {
                let entry = CATALOG.iter().find(|c| c.scenario == id).ok_or_else(|| {
                    format!("unknown scenario {id:?}; see `al-ctl drive --list` for the catalog")
                })?;
                let filter = match driver {
                    Driver::Fake => entry.fake_filter,
                    _ => entry.gpui_filter,
                };
                let Some(filter) = filter else {
                    return Err(format!(
                        "scenario {id:?} has no {driver} lane — it runs only under \
                         `--driver real-os` (which drives the whole catalog)",
                        driver = driver.name()
                    ));
                };
                argv.extend(["--".to_owned(), filter.to_owned()]);
            }
            Ok(LaneCommand { argv, lane })
        }
        Driver::RealOs => {
            if let Some(id) = scenario {
                return Err(format!(
                    "the real-os lane always drives the whole catalog in one \
                     session — drop `scenario {id:?}` and run `al-ctl drive \
                     --driver real-os`"
                ));
            }
            let mut argv = vec![
                "cargo".to_owned(),
                "run".to_owned(),
                "--locked".to_owned(),
                "-p".to_owned(),
                "al-gui".to_owned(),
                "--features".to_owned(),
                "gpui-real".to_owned(),
                "--bin".to_owned(),
                "al-gui-real-scenario".to_owned(),
                "--".to_owned(),
            ];
            if blocked_probe {
                argv.push("--blocked-probe".to_owned());
            }
            if let Some(path) = out {
                argv.extend(["--out".to_owned(), path.to_owned()]);
            }
            Ok(LaneCommand {
                argv,
                lane: "cargo run -p al-gui --features gpui-real --bin al-gui-real-scenario",
            })
        }
    }
}

/// drive/evidence 用の flag 検査 — 値を取る flag と bool flag 以外の
/// token を usage error にする。launch と違ってこれらの subcommand は
/// `--` の後ろを子へ素通ししない。
fn check_flags(args: &[String], value_flags: &[&str], bool_flags: &[&str]) -> Result<(), String> {
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        if value_flags.contains(&arg) {
            if i + 1 >= args.len() {
                return Err(format!("{arg} requires a value"));
            }
            i += 2;
        } else if bool_flags.contains(&arg) {
            i += 1;
        } else {
            return Err(format!("unknown argument {arg:?}; see `al-ctl --help`"));
        }
    }
    Ok(())
}

fn drive_list(json_out: bool) -> ExitCode {
    let cases: Vec<Value> = CATALOG
        .iter()
        .map(|c| {
            json!({
                "scenario": c.scenario,
                "case": c.case,
                "drivers": c.drivers().iter().map(|d| d.name()).collect::<Vec<_>>(),
                "required": c.required,
                "fake_filter": c.fake_filter,
                "gpui_filter": c.gpui_filter,
            })
        })
        .collect();
    if json_out {
        let report = json!({
            "command": "drive",
            "subcommand": "list",
            "cases": cases,
            "catalog_source": "crates/al-gui/src/bin/al-gui-real-scenario.rs CASES + \
                 crates/al-gui/tests/scenario_real.rs case_catalog_ids_are_stable pins",
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!(
            "scenario catalog: {} cases (source: crates/al-gui/src/bin/al-gui-real-scenario.rs CASES)",
            CATALOG.len()
        );
        for c in CATALOG {
            let drivers = c
                .drivers()
                .iter()
                .map(|d| d.name())
                .collect::<Vec<_>>()
                .join(",");
            let required = if c.required.is_empty() {
                "-".to_owned()
            } else {
                c.required.join(",")
            };
            println!(
                "  {}/{}  drivers={}  required={}",
                c.scenario, c.case, drivers, required
            );
        }
    }
    ExitCode::SUCCESS
}

fn cmd_drive(args: &[String], trailing: &[String]) -> ExitCode {
    if flag_present(args, "--help") || flag_present(args, "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if flag_present(args, "--list") {
        return drive_list(flag_present(args, "--json"));
    }
    if !trailing.is_empty() {
        return fail("drive builds the lane command itself — nothing is forwarded after `--`");
    }
    // `drive scenario <id>` の positional 形を先に剥がす。
    let mut positional_scenario: Option<&str> = None;
    let mut flags: &[String] = args;
    if args.first().map(String::as_str) == Some("scenario") {
        match args.get(1) {
            Some(id) => {
                positional_scenario = Some(id.as_str());
                flags = &args[2..];
            }
            None => {
                return fail("`drive scenario` requires a scenario id; see `al-ctl drive --list`");
            }
        }
    }
    if let Err(e) = check_flags(
        flags,
        &["--repo", "--driver", "--scenario", "--out"],
        &["--json", "--dry-run", "--blocked-probe"],
    ) {
        return fail(e);
    }

    let raw_driver = match parse_flag(flags, "--driver") {
        Some(d) => d,
        None => return fail("drive requires `--driver <fake|gpui|real-os>` (or `--list`)"),
    };
    let Some(driver) = Driver::parse(raw_driver) else {
        return fail(format!(
            "unknown driver {raw_driver:?}; expected fake|gpui|real-os"
        ));
    };
    let scenario = match (positional_scenario, parse_flag(flags, "--scenario")) {
        (Some(pos), Some(flag)) if pos != flag => {
            return fail("scenario given twice (`scenario <id>` and `--scenario`)");
        }
        (Some(pos), _) => Some(pos.to_owned()),
        (None, flag) => flag.map(str::to_owned),
    };
    let blocked_probe = flag_present(flags, "--blocked-probe");
    let out_path = parse_flag(flags, "--out");
    if driver != Driver::RealOs && (blocked_probe || out_path.is_some()) {
        return fail("`--blocked-probe`/`--out` apply only to `--driver real-os`");
    }
    let lane = match resolve_lane(driver, scenario.as_deref(), blocked_probe, out_path) {
        Ok(lane) => lane,
        Err(e) => return fail(e),
    };
    let (repo, _) = repo_root(flags);
    let json_out = flag_present(flags, "--json");

    if flag_present(flags, "--dry-run") {
        let report = json!({
            "command": "drive",
            "dry_run": true,
            "spawned": false,
            "driver": driver.name(),
            "scenario": scenario,
            "lane": lane.lane,
            "argv": lane.argv,
            "cwd": repo.display().to_string(),
        });
        if json_out {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!(
                "dry-run: would spawn {} (cwd: {})",
                lane.argv.join(" "),
                repo.display()
            );
        }
        return ExitCode::SUCCESS;
    }

    let spawned = Command::new(&lane.argv[0])
        .args(&lane.argv[1..])
        .current_dir(&repo)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let child = match spawned {
        Ok(child) => child,
        Err(e) => {
            return fail(format!(
                "cannot spawn `cargo`: {e}; run on a host with the rust \
                 toolchain, or drive the mise lanes directly"
            ));
        }
    };
    let pid = child.id();
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => return fail(format!("`{}` did not complete: {e}", lane.argv.join(" "))),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);
    // real-os の `--out` レポートは receipt に行数で写す(観測の一部)。
    let out_records = out_path.and_then(|p| {
        let path = Path::new(p);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            repo.join(path)
        };
        std::fs::read_to_string(path)
            .ok()
            .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
    });
    let report = json!({
        "command": "drive",
        "dry_run": false,
        "spawned": true,
        "driver": driver.name(),
        "scenario": scenario,
        "lane": lane.lane,
        "argv": lane.argv,
        "cwd": repo.display().to_string(),
        "pid": pid,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "out": out_path,
        "out_records": out_records,
    });
    if json_out {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        print!("{stdout}");
        eprint!("{stderr}");
        println!(
            "drive: pid={pid} exit={exit_code} driver={} scenario={}",
            driver.name(),
            scenario.as_deref().unwrap_or("(whole lane)")
        );
    }
    if output.status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

// ------------------------------------------------------------------
// evidence
// ------------------------------------------------------------------

/// `evidence/al<N>/verification.md` の scaffold 雛形。台帳規約は
/// `evidence/al219/README.md`・`evidence/al255/verification.md` の先例 —
/// 生成するのは最小の骨格で、実測の記録は台帳の owner が書く。
const VERIFICATION_SKELETON: &str = "\
# Issue #{ISSUE} — verification evidence

実行環境: <os>/<arch> — <fill in>

## ファイル

| ファイル | 内容 |
| --- | --- |
| `actions.jsonl` | action → 観測結果の 1 行台帳 |

## verdict(POLICIES §3 — 要求状態 × 実行結果)

| 項目 | 要求状態 | 結果 | 根拠 |
| --- | --- | --- | --- |
";

/// `--issue <N>` の検査。数字列のみ受理し `al<N>` の dir 名を返す。
fn ledger_name(args: &[String]) -> Result<String, String> {
    let raw = parse_flag(args, "--issue")
        .ok_or("`--issue <N>` is required (e.g. `--issue 255` -> evidence/al255/)")?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "--issue must be digits (e.g. `--issue 255` -> evidence/al255/), got {raw:?}"
        ));
    }
    Ok(format!("al{raw}"))
}

fn cmd_evidence(args: &[String], trailing: &[String]) -> ExitCode {
    if flag_present(args, "--help") || flag_present(args, "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if !trailing.is_empty() {
        return fail("evidence takes no `--` arguments");
    }
    let Some(sub) = args.first().map(String::as_str) else {
        return fail("evidence requires a subcommand: scaffold | append");
    };
    let rest = &args[1..];
    const BOOLS: &[&str] = &["--json", "--dry-run"];
    let (scaffold, value_flags) = match sub {
        "scaffold" => (true, ["--repo", "--issue"].as_slice()),
        "append" => (
            false,
            [
                "--repo",
                "--issue",
                "--action",
                "--command",
                "--exit-code",
                "--observed",
            ]
            .as_slice(),
        ),
        other => {
            return fail(format!(
                "unknown evidence subcommand {other:?}; expected scaffold | append"
            ));
        }
    };
    if let Err(e) = check_flags(rest, value_flags, BOOLS) {
        return fail(e);
    }
    let ledger = match ledger_name(rest) {
        Ok(name) => name,
        Err(e) => return fail(e),
    };
    let (repo, _) = repo_root(rest);
    let dir = repo.join("evidence").join(&ledger);
    if scaffold {
        evidence_scaffold(rest, &dir, &ledger)
    } else {
        evidence_append(rest, &dir)
    }
}

fn evidence_scaffold(args: &[String], dir: &Path, ledger: &str) -> ExitCode {
    let issue = ledger.strip_prefix("al").unwrap_or(ledger);
    let actions = dir.join("actions.jsonl");
    let verification = dir.join("verification.md");
    let dry_run = flag_present(args, "--dry-run");
    let json_out = flag_present(args, "--json");

    let mut files: Vec<Value> = Vec::new();
    for (path, content) in [
        (&actions, String::new()),
        (
            &verification,
            VERIFICATION_SKELETON.replace("{ISSUE}", issue),
        ),
    ] {
        let existed = path.exists();
        let state = if existed {
            "exists"
        } else if dry_run {
            "would-create"
        } else {
            "created"
        };
        if !existed && !dry_run {
            if let Err(e) = std::fs::create_dir_all(dir) {
                return fail(format!("cannot create {}: {e}", dir.display()));
            }
            if let Err(e) = std::fs::write(path, content) {
                return fail(format!("cannot write {}: {e}", path.display()));
            }
        }
        files.push(json!({ "path": path.display().to_string(), "status": state }));
    }
    let report = json!({
        "command": "evidence",
        "subcommand": "scaffold",
        "dry_run": dry_run,
        "ledger": ledger,
        "dir": dir.display().to_string(),
        "files": files,
    });
    if json_out {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        for file in &files {
            println!("{}: {}", file["status"], file["path"]);
        }
        println!("scaffold: {dir}", dir = dir.display());
    }
    ExitCode::SUCCESS
}

fn evidence_append(args: &[String], dir: &Path) -> ExitCode {
    let actions_path = dir.join("actions.jsonl");
    let json_out = flag_present(args, "--json");
    let dry_run = flag_present(args, "--dry-run");
    for name in ["--action", "--command", "--exit-code", "--observed"] {
        if parse_flag(args, name).is_none() {
            return fail(format!("evidence append requires {name} <value>"));
        }
    }
    let exit_code = match parse_flag(args, "--exit-code").unwrap().parse::<i64>() {
        Ok(code) => code,
        Err(_) => {
            return fail(format!(
                "--exit-code must be an integer, got {:?}",
                parse_flag(args, "--exit-code").unwrap()
            ));
        }
    };
    let record = json!({
        "action": parse_flag(args, "--action").unwrap(),
        "command": parse_flag(args, "--command").unwrap(),
        "exit_code": exit_code,
        "observed": parse_flag(args, "--observed").unwrap(),
    });
    if !actions_path.is_file() {
        return fail_with_code(
            format!(
                "ledger {} not found; run `al-ctl evidence scaffold` for this issue first",
                actions_path.display()
            ),
            1,
        );
    }
    if !dry_run {
        let line = serde_json::to_string(&record).unwrap();
        let appended = std::fs::OpenOptions::new()
            .append(true)
            .open(&actions_path)
            .and_then(|mut f| writeln!(f, "{line}"));
        if let Err(e) = appended {
            return fail_with_code(
                format!("cannot append to {}: {e}", actions_path.display()),
                1,
            );
        }
    }
    let report = json!({
        "command": "evidence",
        "subcommand": "append",
        "dry_run": dry_run,
        "appended": !dry_run,
        "ledger": actions_path.display().to_string(),
        "record": record,
    });
    if json_out {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!(
            "{}: {} -> {}",
            if dry_run { "would append" } else { "appended" },
            serde_json::to_string(&record).unwrap(),
            actions_path.display()
        );
    }
    ExitCode::SUCCESS
}

/// 操作失敗(usage ではない)の報告 — exit 1。
fn fail_with_code(message: impl std::fmt::Display, code: u8) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(code)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // `--` 以降は launch が `al` へそのまま渡す引数 — al-ctl 自身の
    // flag/option は `--` より前だけで解釈する(`al --help` 等を通す)。
    let sep = args.iter().position(|a| a == "--").unwrap_or(args.len());
    let head = &args[..sep];
    if head.is_empty() || flag_present(head, "--help") || flag_present(head, "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let al_args: &[String] = if sep < args.len() {
        &args[sep + 1..]
    } else {
        &[]
    };
    match head.first().map(String::as_str) {
        Some("doctor") => cmd_doctor(&head[1..]),
        Some("launch") => cmd_launch(&head[1..], al_args),
        Some("drive") => cmd_drive(&head[1..], al_args),
        Some("evidence") => cmd_evidence(&head[1..], al_args),
        Some(other) => fail(format!("unknown subcommand {other:?}\n{USAGE}")),
        None => fail(format!("expected a subcommand\n{USAGE}")),
    }
}
