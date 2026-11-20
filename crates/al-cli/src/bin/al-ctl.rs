//! `al-ctl` — AudioLens verification control CLI (Issue #255).
//!
//! Thin control surface for the `verify-audio-lens` skill: a read-only
//! `doctor` that reports environment/fixture/binary health, and a
//! `launch` entry that spawns the product `al` binary and reports the
//! spawned process identity (PID) plus captured output.
//!
//! Headless boundary: std + serde_json only — this bin must not pull
//! GUI/GPU/WebView dependencies (`mise run check-cli-headless`).

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

OPTIONS:
  --repo <dir>     Repository root to inspect. Default: discovered via
                   `git rev-parse --show-toplevel` from the current
                   directory.
  --base-dir <dir> Job data dir to report on (the `--base-dir` used by
                   `al run`). Optional; skipped when not given.
  --al-path <path> Explicit path to the `al` binary. Default: sibling
                   of the al-ctl executable, then <repo>/target/debug,
                   then <repo>/target/release, then PATH.
  --json           Emit a single JSON object instead of text lines.
  --dry-run        (launch) resolve and print the command only.
  --help, -h       Show this text.

EXIT CODES:
  0  success (doctor: all checks ok/skipped; launch: child exited 0)
  1  doctor found a failed check, or the launched `al` exited non-zero
  2  usage error or unresolvable `al` binary
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
        Some(other) => fail(format!("unknown subcommand {other:?}\n{USAGE}")),
        None => fail(format!("expected a subcommand\n{USAGE}")),
    }
}
