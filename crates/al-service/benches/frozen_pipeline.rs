//! Issue #257 frozen benchmark harness (slice 1) — `harness = false` standalone.
//!
//! 2 経路を同一バイナリで計測し、pinned baseline と比較する fail-closed
//! regression gate。
//!
//! - Arm K `kernel/<method>`: `al_locate::methods::default_methods()` の
//!   本番 locate method を、凍結 naive NCC 参照実装(coarse stride grid +
//!   近傍 refine — `FullNcc::locate_coarse_to_fine` と同型)と in-process
//!   A/B で計測する。PRIMARY metric は `median(A)/median(B)` の比 —
//!   絶対時間は機器依存なので informational。naive 側は本ファイル凍結
//!   のため、本番側の回帰は ratio 低下として現れる。
//! - Arm P `pipeline/e2e`: `JobService::run` の本番 end-to-end 解析
//!   pipeline(ingest..dataset)を決定的 WAV fixture 上で計測する。
//!   PRIMARY は同一セッションの凍結参照 kernel median で正規化した
//!   e2e/locate stage wall — 併せて JobId/時刻を除く analysis payload
//!   fingerprint を exact-match で照合し、正確性・機器非依存を
//!   fail-closed で担保する。
//!
//! gate は cell 単位: 計測 ratio / 正規化比が pin の閾値を外れる、
//! error>0(lag 不一致・validity reject・stage 失敗・fingerprint 差分)、
//! baseline の read/昇格不備のいずれでも fail。unsupported cell は
//! run 判定を動かさない。
//!
//! 使い方:
//! - 引数なし: 実走(構造化 JSONL を stdout、PASS/FAIL を stderr、gate 適用)
//! - `--test`: 1 反復 smoke(同じ記録を出すが閾値 gate は report_only、
//!   correctness 由来の fail は通常どおり exit 1)
//! - `--selftest`: 回帰注入(embed 欠落 fixture + kernel B 側を rep 内
//!   直列反復・pipeline e2e wall を ref kernel の 8 倍まで床上げする
//!   劣化注入)で、閾値逸脱・correctness 双方の gate fail を実証する
//! - `--write-baseline <path>`: 実測 candidate JSON を出力(gate 不適用、
//!   `"approval": null` — pin への昇格は人間が行う)
//! - `--baseline <path>`: 既定 pin 以外を読む
//! - positional 引数は cell filter(substring 一致、`kernel/` `pipeline/`
//!   `<method>` 等)。複数可。0 cell 一致は gate `not_run`・exit 0
//!   (cargo bench 慣習)。
//!
//! pinned baseline は `benches/frozen/baseline.al257.json`。昇格は手作業:
//! candidate を pin path に複写し `approval` を記入する。
//! 設計・契約: `docs/test-specs/issue-257-frozen-bench.md`。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use al_core::{InputId, StageKind, StageStatus, Validity};
use al_locate::candidate::MethodEvidence;
use al_locate::methods::default_methods;
use al_locate::pool::CorrelationPool;
use al_service::{JobService, RunControl, RunSpec, parse_input_spec};
use serde_json::{Map, Value, json};

const SCHEMA: &str = "al.frozen-pipeline-bench/v1";
const BASELINE_SCHEMA: &str = "al.frozen-pipeline-baseline/v1";
const ISSUE: &str = "https://github.com/scarecrow173/audio-lens/issues/257";
const DEFAULT_BASELINE: &str = "benches/frozen/baseline.al257.json";

// ---- fixture shape(凍結 — 変更する場合は repin 事由) ----

const RATE: u32 = 48_000;

// Arm K: 既知埋込 offset の noise 信号に対する template locate。
const K_SIGNAL_LEN: usize = 288_000; // 6.0s @48k
const K_TEMPLATE_LEN: usize = 48_000; // 1.0s
const K_EMBED_OFFSET: usize = 120_000; // 2.5s
const K_TEMPLATE_SEED: u64 = 0xA25711;
const K_BED_SEED: u64 = 0xA25712;
const TEMPLATE_AMP: f64 = 0.7;
const BED_AMP: f64 = 0.2;

// naive 参照: coarse grid stride + ±refine。1-3s/rep 目安で調整。
const NAIVE_STRIDE: usize = 6;

// Arm P: capture_bug(埋込) + source_asset の WAV pair。
const P_CAPTURE_LEN: usize = 2_400_000; // 50.0s
const P_TEMPLATE_LEN: usize = 48_000; // 1.0s
const P_EMBED_OFFSET: usize = 960_000; // 20.0s
const P_TEMPLATE_SEED: u64 = 0xA25713;
const P_BED_SEED: u64 = 0xA25714;

const CAPTURE_ID: &str = "IN-0000000000000000000001";
const SOURCE_ID: &str = "IN-0000000000000000000002";

const REPS: usize = 7;
const LAG_TOLERANCE: u64 = 64;
// NCC 系 method の top score と naive 参照 peak の許容乖離。
const PEAK_TOLERANCE: f64 = 1e-3;

// fingerprint 正規化契約(baseline/contracts.json と同じ語彙):
// record-only として落とす key、ID prefix、tolerance grid。
const RECORD_ONLY_KEYS: &[&str] = &[
    "duration_ms",
    "error",
    "started_at",
    "finished_at",
    "recorded_at",
    "created_at",
    "timestamp",
    "command_line",
    "working_dir",
];
const ID_PREFIXES: &[&str] = &["JOB-", "ART-", "ALN-", "IN-"];
const ID_BODY_LEN: usize = 22;
const QUANT_REL: f64 = 1e-9;
const QUANT_ABS: f64 = 1e-12;

fn fail(msg: &str) -> ! {
    eprintln!("frozen_pipeline: error: {msg}");
    std::process::exit(1)
}

fn usage_error(msg: &str) -> ! {
    eprintln!("frozen_pipeline: usage error: {msg}");
    eprintln!(
        "usage: cargo bench -p al-service --bench frozen_pipeline -- [--test|--selftest|--write-baseline <path>|--baseline <path>|--help|<filter>...]"
    );
    std::process::exit(2)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Run,
    Test,
    SelfTest,
    WriteBaseline,
}

struct Args {
    mode: Mode,
    baseline_path: PathBuf,
    write_baseline_path: Option<PathBuf>,
    filters: Vec<String>,
}

fn parse_args(args: &[String]) -> Args {
    if args.iter().any(|a| a == "--help") {
        eprintln!(
            "usage: cargo bench -p al-service --bench frozen_pipeline -- [--test|--selftest|--write-baseline <path>|--baseline <path>|--help|<filter>...]"
        );
        eprintln!(
            "  (no args)              real run: kernel+pipeline arms, JSONL to stdout, gate vs pinned baseline"
        );
        eprintln!(
            "  --test                 1-rep smoke; threshold gates report_only, correctness defects still exit 1"
        );
        eprintln!("  --selftest             regression injection: gate must fail");
        eprintln!("  --write-baseline <p>   emit an un-promoted baseline candidate (no gate)");
        eprintln!("  --baseline <p>         read a different pinned baseline");
        eprintln!(
            "  <filter>               substring match on cell key (kernel/, pipeline/, full_ncc, ...)"
        );
        std::process::exit(0);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut mode = Mode::Run;
    let mut baseline_path = manifest_dir.join(DEFAULT_BASELINE);
    let mut write_baseline_path = None;
    let mut filters = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bench" => {} // cargo が harness=false bench に渡す固定引数
            "--test" => {
                if mode != Mode::Run {
                    usage_error("--test conflicts with another mode flag");
                }
                mode = Mode::Test;
            }
            "--selftest" => {
                if mode != Mode::Run {
                    usage_error("--selftest conflicts with another mode flag");
                }
                mode = Mode::SelfTest;
            }
            "--write-baseline" => {
                if mode != Mode::Run {
                    usage_error("--write-baseline conflicts with another mode flag");
                }
                i += 1;
                if i >= args.len() {
                    usage_error("--write-baseline needs a path");
                }
                mode = Mode::WriteBaseline;
                write_baseline_path = Some(PathBuf::from(&args[i]));
            }
            "--baseline" => {
                i += 1;
                if i >= args.len() {
                    usage_error("--baseline needs a path");
                }
                baseline_path = PathBuf::from(&args[i]);
            }
            s if s.starts_with("--") => usage_error(&format!("unknown flag {s}")),
            s => filters.push(s.to_string()),
        }
        i += 1;
    }
    Args {
        mode,
        baseline_path,
        write_baseline_path,
        filters,
    }
}

// ---- measured env(repin 判断材料 — 実測は実機でしか得られない) ----

fn rustc_version() -> String {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_rev_parse(arg: &str) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", arg])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
}

fn repo_sha() -> Option<String> {
    git_rev_parse("HEAD")
}

fn workspace_root() -> Option<PathBuf> {
    git_rev_parse("--show-toplevel").map(PathBuf::from)
}

fn ruviform_sha() -> Option<String> {
    let lock = fs::read_to_string(workspace_root()?.join("Cargo.lock")).ok()?;
    let mut in_pkg = false;
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            in_pkg = false;
        }
        if in_pkg && line.starts_with("source = ") && line.contains("/ruviform") {
            return line
                .rsplit('#')
                .next()
                .map(|s| s.trim_end_matches('"').to_string());
        }
        if line.starts_with("name = ") && line.contains("ruviform-") {
            in_pkg = true;
        }
    }
    None
}

fn cpu_model() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(text) = fs::read_to_string("/proc/cpuinfo") {
            for line in text.lines() {
                if let Some((k, v)) = line.split_once(':') {
                    if k.trim() == "model name" {
                        return v.trim().to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(o) = std::process::Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
        {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() {
                return s;
            }
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown".to_string())
}

fn measured_env() -> Value {
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "cpu": cpu_model(),
        "cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(0),
        "rustc": rustc_version(),
        "repo_sha": repo_sha(),
        "ruviform_sha": ruviform_sha(),
        "note": "repin は機器/toolchain/al* コード・入力 asset・ruviform SHA の変更時に検討する",
    })
}

// ---- naive NCC 参照実装(凍結 — FullNcc::locate_coarse_to_fine と同型の
// coarse grid + fine refine。最適化しない: こちらが session 内の安定分母) ----

fn ncc_at(template: &[f64], signal: &[f64], lag: usize, template_sq: f64) -> (f64, u64) {
    let window = &signal[lag..lag + template.len()];
    let mut dot = 0.0f64;
    let mut window_sq = 0.0f64;
    for (i, &t) in template.iter().enumerate() {
        dot += t * window[i];
        window_sq += window[i] * window[i];
    }
    let denom = (template_sq * window_sq).sqrt().max(1e-12);
    (dot / denom, (template.len() * 2) as u64)
}

/// (best_lag, peak_score, macs)。coarse grid stride → argmax → ±stride refine。
fn frozen_naive_ncc(template: &[f64], signal: &[f64], stride: usize) -> (usize, f64, u64) {
    let m = template.len();
    let max_lag = signal.len() - m;
    let template_sq: f64 = template.iter().map(|v| v * v).sum();
    let mut macs = 0u64;
    let mut best_lag = 0usize;
    let mut best_score = f64::NEG_INFINITY;
    let mut lag = 0usize;
    while lag <= max_lag {
        let (score, ops) = ncc_at(template, signal, lag, template_sq);
        macs += ops;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
        lag += stride;
    }
    let lo = best_lag.saturating_sub(stride);
    let hi = (best_lag + stride).min(max_lag);
    for lag in lo..=hi {
        let (score, ops) = ncc_at(template, signal, lag, template_sq);
        macs += ops;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }
    (best_lag, best_score, macs)
}

// ---- fixture ----

fn to_s16(samples: &[f64]) -> Vec<i16> {
    samples
        .iter()
        .map(|&v| (v.clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect()
}

/// Arm K fixture。(template, signal, expected_lag)。`embed=false` は
/// selftest の回帰注入(埋込欠落 → locate が成立しない)。
fn kernel_fixture(embed: bool) -> (Vec<f64>, Vec<f64>, usize) {
    let template = al_testkit::signal::noise(K_TEMPLATE_LEN, K_TEMPLATE_SEED, TEMPLATE_AMP);
    let mut signal = al_testkit::signal::noise(K_SIGNAL_LEN, K_BED_SEED, BED_AMP);
    if embed {
        al_testkit::signal::embed(&mut signal, &template, K_EMBED_OFFSET, 1.0);
    }
    (template, signal, K_EMBED_OFFSET)
}

/// Arm P fixture を scratch dir に WAV として生成し、fixture dir path を返す。
fn pipeline_fixture(scratch: &Path, embed: bool) -> std::io::Result<PathBuf> {
    let dir = scratch.join("inputs");
    fs::create_dir_all(&dir)?;
    let template = al_testkit::signal::noise(P_TEMPLATE_LEN, P_TEMPLATE_SEED, TEMPLATE_AMP);
    let mut capture = al_testkit::signal::noise(P_CAPTURE_LEN, P_BED_SEED, BED_AMP);
    if embed {
        al_testkit::signal::embed(&mut capture, &template, P_EMBED_OFFSET, 1.0);
    }
    fs::write(
        dir.join("capture.wav"),
        al_testkit::media::wav_s16_bytes(1, RATE, &to_s16(&capture)),
    )?;
    fs::write(
        dir.join("source.wav"),
        al_testkit::media::wav_s16_bytes(1, RATE, &to_s16(&template)),
    )?;
    Ok(dir)
}

// ---- ID / record-only / 量子化正規化(baseline/contracts.json 先例) ----

fn is_id_body_char(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// `PREFIX-<22 base62>` 形の ID を canonical(`<PREFIX>-#<n>`)へ置換する。
/// 割当順は引き渡された文字列を左から走査した出現順。
fn replace_ids(
    text: &str,
    canonical: &mut BTreeMap<String, String>,
    counts: &mut BTreeMap<String, usize>,
) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        let mut matched: Option<(&str, usize)> = None;
        for prefix in ID_PREFIXES {
            // ID_RE と同じ (?<![0-9A-Za-z]) の左境界: より長い token 内の
            // "IN-…"(例 "JOIN-…")は ID ではない。
            if bytes[i..].starts_with(prefix.as_bytes())
                && (i == 0 || !is_id_body_char(bytes[i - 1]))
            {
                let start = i + prefix.len();
                let end = start + ID_BODY_LEN;
                if end <= bytes.len()
                    && bytes[start..end].iter().all(|b| is_id_body_char(*b))
                    && (end == bytes.len() || !is_id_body_char(bytes[end]))
                {
                    matched = Some((prefix, end));
                    break;
                }
            }
        }
        match matched {
            Some((prefix, end)) => {
                let raw = &text[i..end];
                let canon = canonical.entry(raw.to_string()).or_insert_with(|| {
                    let n = counts.entry(prefix.to_string()).or_insert(0);
                    *n += 1;
                    format!("{prefix}#{n}")
                });
                out.push_str(canon);
                i = end;
            }
            None => {
                let ch = text[i..].chars().next().unwrap();
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }
    out
}

fn quantize(v: f64) -> Value {
    if !v.is_finite() {
        return Value::String("<nonfinite>".to_string());
    }
    let grid = (QUANT_REL * v.abs()).max(QUANT_ABS);
    let q = (v / grid).round();
    if q.abs() < 9.007_199_254_740_992e15 {
        Value::from(q as i64)
    } else {
        Value::String(format!("q:{q}"))
    }
}

/// JSON Value を正規化する: record-only key 削除、ID canonical 化、
/// 絶対パス placeholder 化、f64 の tolerance grid 量子化。
/// `path_repl` は (実 path 文字列, placeholder) のペア。
fn normalize_value(
    v: &mut Value,
    canonical: &mut BTreeMap<String, String>,
    counts: &mut BTreeMap<String, usize>,
    path_repl: &[(String, String)],
) {
    match v {
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            let mut rebuilt = Map::new();
            for key in keys {
                if RECORD_ONLY_KEYS.contains(&key.as_str()) {
                    continue;
                }
                if let Some(mut child) = map.remove(&key) {
                    normalize_value(&mut child, canonical, counts, path_repl);
                    rebuilt.insert(key, child);
                }
            }
            *map = rebuilt;
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                normalize_value(item, canonical, counts, path_repl);
            }
        }
        Value::String(s) => {
            let mut t = replace_ids(s, canonical, counts);
            for (from, to) in path_repl {
                t = t.replace(from.as_str(), to.as_str());
            }
            *s = t;
        }
        Value::Number(n) if n.is_f64() => {
            *v = quantize(n.as_f64().unwrap_or(f64::NAN));
        }
        _ => {}
    }
}

// goldenJsonSkipGlobs 相当: 構造上 record-only 扱いの json を fingerprint
// 対象から除く(table 構造ファイル・export・visualization・動画 thumbnail)。
fn fingerprint_excluded(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    matches!(
        name,
        "video_thumbs.json" | "array.json" | "schema.json" | "index.json"
    ) || rel.starts_with("export/")
        || rel.starts_with("analysis/visualization/")
}

/// job dir 配下の analysis payload fingerprint。
/// 正規化 JSON の canonical 直列化 → 相対 path→sha256 の map。
fn fingerprint_job_dir(
    job_dir: &Path,
    fixture_dir: &Path,
    scratch_root: &Path,
) -> std::io::Result<BTreeMap<String, String>> {
    let mut rels: Vec<String> = Vec::new();
    let mut stack = vec![job_dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "json") {
                let rel = path
                    .strip_prefix(job_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                rels.push(rel);
            }
        }
    }
    rels.retain(|r| !fingerprint_excluded(r));
    rels.sort();

    let mut canonical: BTreeMap<String, String> = BTreeMap::new();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    // 長い path から先に置換する(job/fixture は scratch_root の配下)。
    let mut path_repl: Vec<(String, String)> = Vec::new();
    for (dir, name) in [
        (job_dir, "<JOB>"),
        (fixture_dir, "<FIXTURE>"),
        (scratch_root, "<SCRATCH>"),
    ] {
        path_repl.push((dir.to_string_lossy().to_string(), name.to_string()));
        path_repl.push((dir.to_string_lossy().replace('\\', "/"), name.to_string()));
    }

    let mut out = BTreeMap::new();
    for rel in &rels {
        let text = fs::read_to_string(job_dir.join(rel))?;
        let sha = match serde_json::from_str::<Value>(&text) {
            Ok(mut value) => {
                normalize_value(&mut value, &mut canonical, &mut counts, &path_repl);
                let canon = serde_json::to_string(&value).unwrap_or_default();
                al_core::sha256_hex(canon.as_bytes())
            }
            Err(_) => al_core::sha256_hex(text.as_bytes()),
        };
        let rel_norm = replace_ids(rel, &mut canonical, &mut counts);
        out.insert(rel_norm, sha);
    }
    Ok(out)
}

fn fingerprint_digest(map: &BTreeMap<String, String>) -> String {
    let mut buf = String::new();
    for (k, v) in map {
        buf.push_str(k);
        buf.push(' ');
        buf.push_str(v);
        buf.push('\n');
    }
    al_core::sha256_hex(buf.as_bytes())
}

/// fingerprint map 同士の差分(欠落・追加・値不一致の件数)。
fn fingerprint_diff(a: &BTreeMap<String, String>, b: &BTreeMap<String, String>) -> u64 {
    let mut diff = 0u64;
    for (k, v) in a {
        match b.get(k) {
            Some(w) if w == v => {}
            _ => diff += 1,
        }
    }
    for k in b.keys() {
        if !a.contains_key(k) {
            diff += 1;
        }
    }
    diff
}

// ---- 統計 ----

fn median(sorted: &[u64]) -> u64 {
    let n = sorted.len();
    if n == 0 {
        0
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    }
}

fn stats(times: &[u64]) -> Value {
    let mut s = times.to_vec();
    s.sort();
    json!({
        "median_ns": median(&s),
        "min_ns": s.first().copied().unwrap_or(0),
        "max_ns": s.last().copied().unwrap_or(0),
        "reps": s.len(),
    })
}

// ---- cell 状態 ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Pass,
    Fail,
    ReportOnly,
    Unsupported,
    NotRun,
}

impl Verdict {
    fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::ReportOnly => "report_only",
            Verdict::Unsupported => "unsupported",
            Verdict::NotRun => "not_run",
        }
    }
}

struct CellResult {
    key: String,
    verdict: Verdict,
    reasons: Vec<String>,
    record: Value,
}

// ---- Arm K: kernel A/B ----

struct KernelCellResult {
    key: String,
    a_times: Vec<u64>,
    b_times: Vec<u64>,
    ratio: f64,
    errors: u64,
    error_detail: Value,
    work: u64,
}

/// selftest の回帰注入: B 側を 1 rep 内でこの回数だけ直列実行し、
/// timed region を確定的に遅くする(`median(A)/median(B)` を pin の
/// min_ratio 未満へ落とす — 機器差に依存しない閾値逸脱)。
const SELFTEST_B_REPEATS: usize = 4;

/// kernel cell を 1 つ計測する(A/B 交互、median+range、error+work 計数)。
/// `degrade_b` は selftest の回帰注入(B 側 timed region を
/// `SELFTEST_B_REPEATS` 倍にする — 実在の「本番側が閾値を外れるほど
/// 遅くなった」回帰形)。
fn measure_kernel_cell(
    method_idx: usize,
    methods: &[Box<dyn al_locate::methods::LocateMethod>],
    template: &[f64],
    signal: &[f64],
    expected_lag: usize,
    reps: usize,
    degrade_b: bool,
) -> KernelCellResult {
    let method = &methods[method_idx];
    let id = method.id();
    let mut pool = CorrelationPool::new();
    let b_repeats = if degrade_b { SELFTEST_B_REPEATS } else { 1 };
    let mut a_times = Vec::with_capacity(reps);
    let mut b_times = Vec::with_capacity(reps);
    let mut work = 0u64;
    let mut evidence: Option<MethodEvidence> = None;
    let mut naive_peak = 0.0f64;
    let mut naive_lag = 0usize;
    for rep in 0..reps {
        // 交互: A→B の順。セッション内 drift の影響を両側に散らす。
        let t0 = Instant::now();
        let (lag, peak, macs) = frozen_naive_ncc(template, signal, NAIVE_STRIDE);
        a_times.push(t0.elapsed().as_nanos() as u64);
        work += macs;
        naive_peak = peak;
        naive_lag = lag;

        let t1 = Instant::now();
        let mut ev = method.locate(template, signal, RATE, &mut pool);
        for _ in 1..b_repeats {
            ev = method.locate(template, signal, RATE, &mut pool);
        }
        b_times.push(t1.elapsed().as_nanos() as u64);
        if rep == 0 {
            evidence = Some(ev);
        }
    }
    // B 側 work: 相関曲線の探索域サイズ × reps × 直列反復数
    // (signal 長に応じた実作業量)。
    let max_lag = signal.len().saturating_sub(template.len());
    work += (max_lag as u64 + 1) * reps as u64 * b_repeats as u64;

    let mut lag_mismatch = 0u64;
    let mut validity_rejects = 0u64;
    let mut peak_deviations = 0u64;
    let top = evidence.as_ref().and_then(|e| e.candidates.first());
    if naive_lag != expected_lag {
        lag_mismatch += 1; // 参照側 self-check
    }
    match evidence.as_ref() {
        Some(ev) => {
            if ev.validity != Validity::Valid {
                validity_rejects += 1;
            }
        }
        None => validity_rejects += 1,
    }
    match top {
        Some(c) => {
            if c.position.abs_diff(expected_lag as u64) > LAG_TOLERANCE {
                lag_mismatch += 1;
            }
            // NCC 系スコアのみ naive peak と比較可能(whitened/envelope 系は
            // スケールが違う — full_ncc のみ同じ正規化相関値を返す)。
            if id == "full_ncc" && (c.score - naive_peak).abs() > PEAK_TOLERANCE {
                peak_deviations += 1;
            }
        }
        None => lag_mismatch += 1,
    }
    let errors = lag_mismatch + validity_rejects + peak_deviations;
    let mut a_sorted = a_times.clone();
    a_sorted.sort();
    let mut b_sorted = b_times.clone();
    b_sorted.sort();
    let a_med = median(&a_sorted) as f64;
    let b_med = (median(&b_sorted).max(1)) as f64;
    KernelCellResult {
        key: format!("kernel/{id}"),
        ratio: a_med / b_med,
        a_times,
        b_times,
        errors,
        error_detail: json!({
            "lag_mismatch": lag_mismatch,
            "validity_reject": validity_rejects,
            "peak_deviation": peak_deviations,
        }),
        work,
    }
}

// ---- Arm P: pipeline ----

const PIPELINE_STAGES: &[StageKind] = &[
    StageKind::Ingest,
    StageKind::Locate,
    StageKind::CheapAnalysis,
    StageKind::ReferenceAnalysis,
    StageKind::ExpensiveAnalysis,
    StageKind::Dataset,
];

struct PipelineRep {
    wall_ns: u64,
    stage_walls: BTreeMap<String, u64>,
    stage_failures: u64,
    run_error: Option<String>,
    artifacts: u64,
    output_bytes: u64,
    fingerprint: Option<BTreeMap<String, String>>,
    accepted_lag: Option<u64>,
}

fn dir_bytes(root: &Path) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Ok(meta) = path.metadata() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn stage_name(kind: StageKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{kind:?}"))
}

/// pipeline rep を 1 回実行する。fixture dir の WAV 2 本を入力に、
/// fresh job dir で full stage plan を走らせ、e2e wall + stage wall +
/// fingerprint + error/work を採る。
fn run_pipeline_rep(service: &JobService, fixture_dir: &Path, scratch_root: &Path) -> PipelineRep {
    let cap = fixture_dir.join("capture.wav");
    let src = fixture_dir.join("source.wav");
    let mut inputs = vec![
        parse_input_spec(&format!("capture_bug:{}", cap.display()))
            .unwrap_or_else(|_| fail("capture_bug input spec parse")),
        parse_input_spec(&format!("source_asset:{}", src.display()))
            .unwrap_or_else(|_| fail("source_asset input spec parse")),
    ];
    JobService::assign_input_ids(
        &mut inputs,
        vec![
            InputId::parse(CAPTURE_ID).unwrap_or_else(|_| fail("capture InputId parse")),
            InputId::parse(SOURCE_ID).unwrap_or_else(|_| fail("source InputId parse")),
        ],
    )
    .unwrap_or_else(|e| fail(&format!("assign_input_ids: {e}")));

    let spec = RunSpec {
        inputs,
        stages: Some(PIPELINE_STAGES.to_vec()),
        config: json!({}),
    };
    let t0 = Instant::now();
    let run = service.run(spec, &RunControl::default());
    let wall_ns = t0.elapsed().as_nanos() as u64;

    match run {
        Ok((job_id, manifest)) => {
            let job_dir = service.job_dir(&job_id);
            let mut stage_walls = BTreeMap::new();
            let mut stage_failures = 0u64;
            for record in &manifest.stages {
                if let Some(ms) = record.duration_ms {
                    stage_walls.insert(stage_name(record.stage), ms * 1_000_000);
                }
                if record.status == StageStatus::Failed {
                    stage_failures += 1;
                }
            }
            let artifacts = manifest.artifacts.len() as u64;
            let output_bytes = dir_bytes(&job_dir);
            let fingerprint = fingerprint_job_dir(&job_dir, fixture_dir, scratch_root).ok();
            let accepted_lag = locate_accepted_lag(&job_dir);
            let result = PipelineRep {
                wall_ns,
                stage_walls,
                stage_failures,
                run_error: None,
                artifacts,
                output_bytes,
                fingerprint,
                accepted_lag,
            };
            let _ = fs::remove_dir_all(&job_dir);
            result
        }
        Err(e) => PipelineRep {
            wall_ns,
            stage_walls: BTreeMap::new(),
            stage_failures: PIPELINE_STAGES.len() as u64,
            run_error: Some(format!("{e}")),
            artifacts: 0,
            output_bytes: 0,
            fingerprint: None,
            accepted_lag: None,
        },
    }
}

/// `locate/<source>/<capture>/result.json` の accepted capture offset。
/// source/capture の canonical input id が固定のため path は決定的。
fn locate_accepted_lag(job_dir: &Path) -> Option<u64> {
    let path = job_dir
        .join("locate")
        .join(SOURCE_ID)
        .join(CAPTURE_ID)
        .join("result.json");
    let value: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    let result = value.get("result")?;
    if result.get("accepted")?.as_bool()? {
        result.get("capture_range")?.get("start")?.as_u64()
    } else {
        None
    }
}

struct PipelineResult {
    wall_ns: Vec<u64>,
    locate_wall_ns: Vec<u64>,
    stage_walls: BTreeMap<String, Vec<u64>>,
    fingerprints: Vec<BTreeMap<String, String>>,
    errors: u64,
    error_detail: Value,
    work: u64,
    rep_details: Vec<Value>,
}

/// `degrade_floor_ns` は selftest の回帰注入 — rep 毎に e2e wall が
/// この値を下回らないよう実 sleep を timed region に追加し、
/// 正規化比を pin の max_norm_ratio 超へ確定的に押し上げる。
fn measure_pipeline(
    service: &JobService,
    fixture_dir: &Path,
    scratch_root: &Path,
    pinned_fingerprint: Option<&BTreeMap<String, String>>,
    reps: usize,
    degrade_floor_ns: Option<u64>,
) -> PipelineResult {
    let mut wall_ns = Vec::with_capacity(reps);
    let mut locate_wall_ns = Vec::with_capacity(reps);
    let mut stage_walls: BTreeMap<String, Vec<u64>> = BTreeMap::new();
    let mut fingerprints = Vec::with_capacity(reps);
    let mut rep_details = Vec::with_capacity(reps);
    let mut stage_failures = 0u64;
    let mut lag_mismatch = 0u64;
    let mut artifacts = 0u64;
    let mut output_bytes = 0u64;
    for _ in 0..reps {
        let rep = run_pipeline_rep(service, fixture_dir, scratch_root);
        let deficit = degrade_floor_ns
            .map(|floor| floor.saturating_sub(rep.wall_ns))
            .unwrap_or(0);
        if deficit > 0 {
            std::thread::sleep(std::time::Duration::from_nanos(deficit));
        }
        wall_ns.push(rep.wall_ns + deficit);
        stage_failures += rep.stage_failures;
        artifacts += rep.artifacts;
        output_bytes += rep.output_bytes;
        match rep.accepted_lag {
            Some(lag) if lag.abs_diff(P_EMBED_OFFSET as u64) <= LAG_TOLERANCE => {}
            _ => lag_mismatch += 1,
        }
        for (k, v) in &rep.stage_walls {
            stage_walls.entry(k.clone()).or_default().push(*v);
        }
        locate_wall_ns.push(rep.stage_walls.get("locate").copied().unwrap_or(0));
        let fingerprint_files = rep.fingerprint.as_ref().map(|f| f.len());
        let had_fingerprint = rep.fingerprint.is_some();
        if let Some(fp) = rep.fingerprint {
            fingerprints.push(fp);
        }
        if !had_fingerprint {
            stage_failures += 1; // fingerprint 生成不能 = payload 未到達
        }
        rep_details.push(json!({
            "wall_ns": rep.wall_ns,
            "artifacts": rep.artifacts,
            "output_bytes": rep.output_bytes,
            "fingerprint_files": fingerprint_files,
            "run_error": rep.run_error,
        }));
    }
    // rep 間 A/A 決定性: rep0 基準で各 rep の差分を数える。
    let mut rep_divergence = 0u64;
    if let Some(base) = fingerprints.first() {
        for fp in fingerprints.iter().skip(1) {
            rep_divergence += fingerprint_diff(base, fp);
        }
    }
    let pin_diffs = pinned_fingerprint
        .and_then(|pin| fingerprints.first().map(|fp| fingerprint_diff(pin, fp)))
        .unwrap_or(0);
    let errors = stage_failures + lag_mismatch + rep_divergence + pin_diffs;
    PipelineResult {
        wall_ns,
        locate_wall_ns,
        stage_walls,
        fingerprints,
        errors,
        error_detail: json!({
            "stage_failed": stage_failures,
            "lag_mismatch": lag_mismatch,
            "fingerprint_rep_divergence": rep_divergence,
            "fingerprint_pin_diffs": pin_diffs,
        }),
        work: artifacts + output_bytes,
        rep_details,
    }
}

// ---- baseline ----

fn load_pinned(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("baseline read failed for {}: {e}", path.display()))?;
    let baseline: Value = serde_json::from_str(&text)
        .map_err(|e| format!("baseline parse failed for {}: {e}", path.display()))?;
    if baseline.get("schema").and_then(Value::as_str) != Some(BASELINE_SCHEMA) {
        return Err(format!(
            "baseline schema mismatch in {} — expected {BASELINE_SCHEMA}",
            path.display()
        ));
    }
    match baseline.get("approval") {
        Some(v) if !v.is_null() => {}
        _ => {
            return Err(format!(
                "baseline {} is not promoted (`approval` is null) — promote the candidate explicitly",
                path.display()
            ));
        }
    }
    Ok(baseline)
}

/// pin 閾値ルール: kernel は測定 ratio の優位半減、pipeline は
/// 正規化比の 1.5 倍を上限として切り上げる。
fn kernel_threshold(measured_ratio: f64) -> f64 {
    ((1.0 + (measured_ratio - 1.0) * 0.5).max(0.1) * 1000.0).ceil() / 1000.0
}

fn pipeline_threshold(measured_norm: f64) -> f64 {
    ((measured_norm * 1.5).max(0.01) * 100.0).ceil() / 100.0
}

// ---- main ----

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args(&argv);
    let mode = args.mode;
    let reps = if mode == Mode::Test { 1 } else { REPS };
    let selftest = mode == Mode::SelfTest;
    let write_baseline = mode == Mode::WriteBaseline;
    let mode_name = match mode {
        Mode::Run => "run",
        Mode::Test => "test",
        Mode::SelfTest => "selftest",
        Mode::WriteBaseline => "write-baseline",
    };
    let methods = default_methods();
    // limiter 記述: timed region の支配要因と e2e 関連性を常時記録する
    // (benchmark checklist の必須項)。
    let mut limiter_notes: Vec<String> = vec![
        "kernel: naive NCC は single-thread scalar(CPU bound)。本番 method は FFT+pool 再利用で支配的に速い — ratio は 10x-70x 帯".to_string(),
        "pipeline: locate stage が支配(本番 baseline で e2e の 69-79%)。disk I/O・s16 WAV decode は小さい".to_string(),
        "relevance: pipeline arm は production `al run` と同じ JobService::run 経路 — e2e 応答時間そのもの。kernel arm はその支配 stage の hot kernel".to_string(),
    ];

    // gate 用 pin は計測前に解決する(fail fast)。write-baseline は読まない。
    let pinned: Option<Value> = if write_baseline {
        None
    } else {
        match load_pinned(&args.baseline_path) {
            Ok(b) => Some(b),
            Err(e) => {
                if mode == Mode::Test {
                    limiter_notes.push(format!(
                        "pinned baseline unreadable (smoke: gate report_only): {e}"
                    ));
                    None
                } else {
                    println!(
                        "{}",
                        json!({
                            "kind": "gate",
                            "verdict": "fail",
                            "reasons": ["baseline_unreadable"],
                            "detail": e,
                        })
                    );
                    eprintln!("FAIL: baseline_unreadable — {e}");
                    return ExitCode::from(1);
                }
            }
        }
    };
    let pinned_fingerprint: Option<BTreeMap<String, String>> = pinned.as_ref().and_then(|b| {
        b.get("fingerprint").and_then(Value::as_object).map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
    });

    // filter: 空なら全 cell。write-baseline は pin 全体を再測定するため
    // filter を適用しない。
    let want = |key: &str| {
        write_baseline || args.filters.is_empty() || args.filters.iter().any(|f| key.contains(f))
    };
    let kernel_keys: Vec<String> = methods
        .iter()
        .map(|m| format!("kernel/{}", m.id()))
        .collect();
    let run_kernels: Vec<usize> = (0..methods.len())
        .filter(|&i| want(&kernel_keys[i]))
        .collect();
    let run_pipeline = want("pipeline/e2e");

    let mut records: Vec<String> = Vec::new();
    let mut cells: Vec<CellResult> = Vec::new();

    records.push(
        json!({
            "kind": "run",
            "schema": SCHEMA,
            "issue": ISSUE,
            "mode": mode_name,
            "reps": reps,
            "metric": "kernel: in-process A/B median(A)/median(B); pipeline: wall/ref_kernel_median",
            "fixture": {
                "rate": RATE,
                "kernel": {
                    "signal_len": K_SIGNAL_LEN,
                    "template_len": K_TEMPLATE_LEN,
                    "embed_offset": K_EMBED_OFFSET,
                    "embedded": !selftest,
                    "naive_stride": NAIVE_STRIDE,
                },
                "pipeline": {
                    "capture_len": P_CAPTURE_LEN,
                    "template_len": P_TEMPLATE_LEN,
                    "embed_offset": P_EMBED_OFFSET,
                    "embedded": !selftest,
                    "inputs": [
                        {"role": "capture_bug", "input_id": CAPTURE_ID},
                        {"role": "source_asset", "input_id": SOURCE_ID},
                    ],
                    "stages": PIPELINE_STAGES.iter().map(|k| stage_name(*k)).collect::<Vec<_>>(),
                },
            },
            "measured_env": measured_env(),
        })
        .to_string(),
    );
    if selftest {
        records.push(
            json!({
                "kind": "injected",
                "detail": "missing_embed fixture + kernel B 側直列反復(B timed region ×SELFTEST_B_REPEATS)+ pipeline e2e floor(ref×8)— 閾値逸脱と correctness 両経路で gate fail が期待値",
            })
            .to_string(),
        );
    }

    // ---- Arm K ----
    // selftest は embed 欠落 fixture(回帰注入: source が capture に
    // 存在しない → locate 不成立 → correctness gate が落ちる)。
    let (template, signal, expected_lag) = kernel_fixture(!selftest);
    let mut ref_times: Vec<u64> = Vec::new();
    let mut kernel_results: Vec<KernelCellResult> = Vec::new();
    for &idx in &run_kernels {
        let res = measure_kernel_cell(
            idx,
            &methods,
            &template,
            &signal,
            expected_lag,
            reps,
            selftest,
        );
        ref_times.extend(res.a_times.iter().copied());
        kernel_results.push(res);
    }
    // pipeline arm が選ばれているが kernel cell が 0 のときも、
    // 正規化分母として naive ref だけは測る。
    if run_pipeline && kernel_results.is_empty() {
        for _ in 0..reps {
            let t0 = Instant::now();
            frozen_naive_ncc(&template, &signal, NAIVE_STRIDE);
            ref_times.push(t0.elapsed().as_nanos() as u64);
        }
    }
    ref_times.sort();
    let ref_kernel_median_ns = median(&ref_times);
    if ref_kernel_median_ns > 0 {
        records.push(
            json!({
                "kind": "ref_kernel",
                "desc": "frozen naive NCC — pipeline normalized ratio の分母",
                "stats": stats(&ref_times),
            })
            .to_string(),
        );
    }

    // ---- Arm P ----
    let scratch = std::env::temp_dir().join(format!("al257-frozen-bench-{}", std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap_or_else(|e| fail(&format!("scratch dir: {e}")));
    let fixture_dir = pipeline_fixture(&scratch, !selftest)
        .unwrap_or_else(|e| fail(&format!("pipeline fixture: {e}")));
    let service = JobService::new(scratch.join("jobs"));
    let pipeline_result = if run_pipeline {
        Some(measure_pipeline(
            &service,
            &fixture_dir,
            &scratch,
            pinned_fingerprint.as_ref(),
            reps,
            // selftest 注入: e2e wall を ref median の 8 倍まで床上げ
            // (pin max_norm ≈6.6 を確定的に超える劣化)。
            selftest.then(|| ref_kernel_median_ns.saturating_mul(8)),
        ))
    } else {
        None
    };
    let _ = fs::remove_dir_all(&scratch);

    // ---- Arm K cell records + gate ----
    for res in &kernel_results {
        let pin_cell = pinned
            .as_ref()
            .and_then(|b| b.get("cells"))
            .and_then(|c| c.get(&res.key));
        let min_ratio = pin_cell
            .and_then(|c| c.get("min_ratio"))
            .and_then(Value::as_f64);
        let mut reasons: Vec<String> = Vec::new();
        if res.errors > 0 {
            reasons.push("errors>0".to_string());
        }
        let mut verdict = if res.errors > 0 {
            Verdict::Fail
        } else {
            Verdict::Pass
        };
        if let Some(th) = min_ratio {
            if res.ratio < th {
                reasons.push(format!("ratio_below_threshold({:.3}<{th})", res.ratio));
                if mode == Mode::Test {
                    verdict = Verdict::ReportOnly;
                    limiter_notes.push(format!(
                        "{} ratio {:.3} below pin {:.3} — smoke は report_only",
                        res.key, res.ratio, th
                    ));
                } else {
                    verdict = Verdict::Fail;
                }
            }
        } else if !write_baseline && pinned.is_some() {
            verdict = Verdict::Unsupported;
            reasons.push("no_pinned_threshold".to_string());
        }
        cells.push(CellResult {
            key: res.key.clone(),
            verdict,
            reasons: reasons.clone(),
            record: json!({
                "kind": "cell",
                "cell": res.key,
                "arm": "kernel",
                "metric": "median(A)/median(B)",
                "a": { "desc": "frozen naive NCC (coarse stride + refine)", "stats": stats(&res.a_times) },
                "b": { "desc": format!("production {}", res.key), "stats": stats(&res.b_times) },
                "ratio": res.ratio,
                "min_ratio": min_ratio,
                "errors": res.errors,
                "error_detail": res.error_detail,
                "work": res.work,
                "verdict": verdict.as_str(),
                "reasons": reasons,
            }),
        });
    }

    // ---- Arm P cell record + gate ----
    if let Some(p) = &pipeline_result {
        let mut wall_sorted = p.wall_ns.clone();
        wall_sorted.sort();
        let e2e_median_ns = median(&wall_sorted) as f64;
        let ref_med = ref_kernel_median_ns.max(1) as f64;
        let norm_e2e = e2e_median_ns / ref_med;
        let mut loc_sorted = p.locate_wall_ns.clone();
        loc_sorted.sort();
        let locate_norm = (median(&loc_sorted) as f64) / ref_med;

        let pin_cell = pinned
            .as_ref()
            .and_then(|b| b.get("cells"))
            .and_then(|c| c.get("pipeline/e2e"));
        let max_norm_e2e = pin_cell
            .and_then(|c| c.get("max_norm_ratio"))
            .and_then(Value::as_f64);
        let max_norm_locate = pin_cell
            .and_then(|c| c.get("max_norm_ratio_locate"))
            .and_then(Value::as_f64);

        let mut reasons: Vec<String> = Vec::new();
        if p.errors > 0 {
            reasons.push("errors>0".to_string());
        }
        let mut verdict = if p.errors > 0 {
            Verdict::Fail
        } else {
            Verdict::Pass
        };
        for (label, norm, max) in [
            ("e2e_norm", norm_e2e, max_norm_e2e),
            ("locate_norm", locate_norm, max_norm_locate),
        ] {
            if let Some(th) = max
                && norm > th
            {
                reasons.push(format!("{label}_above_threshold({norm:.3}>{th})"));
                if mode == Mode::Test {
                    verdict = Verdict::ReportOnly;
                    limiter_notes.push(format!(
                        "pipeline {label} {norm:.3} above pin {th} — smoke は report_only"
                    ));
                } else {
                    verdict = Verdict::Fail;
                }
            }
        }
        if !write_baseline && pinned.is_some() && max_norm_e2e.is_none() && p.errors == 0 {
            verdict = Verdict::Unsupported;
            reasons.push("no_pinned_threshold".to_string());
        }

        let fingerprint_digest_now = p.fingerprints.first().map(fingerprint_digest);
        let stage_wall_json: Map<String, Value> = p
            .stage_walls
            .iter()
            .map(|(k, v)| (k.clone(), stats(v)))
            .collect();
        cells.push(CellResult {
            key: "pipeline/e2e".to_string(),
            verdict,
            reasons: reasons.clone(),
            record: json!({
                "kind": "cell",
                "cell": "pipeline/e2e",
                "arm": "pipeline",
                "metric": "wall_ns/ref_kernel_median_ns",
                "ref_kernel_median_ns": ref_kernel_median_ns,
                "wall": stats(&p.wall_ns),
                "norm_ratio_e2e": norm_e2e,
                "max_norm_ratio": max_norm_e2e,
                "stage_walls_ns": Value::Object(stage_wall_json),
                "locate_norm_ratio": locate_norm,
                "max_norm_ratio_locate": max_norm_locate,
                "errors": p.errors,
                "error_detail": p.error_detail,
                "work": p.work,
                "fingerprint": {
                    "files": p.fingerprints.first().map(|f| f.len()).unwrap_or(0),
                    "digest": fingerprint_digest_now,
                    "quant_grid": {"rel": QUANT_REL, "abs": QUANT_ABS},
                },
                "reps_detail": p.rep_details,
                "verdict": verdict.as_str(),
                "reasons": reasons,
            }),
        });
    }

    // ---- 全体 verdict / gate record ----
    let mut run_verdict = Verdict::Pass;
    for cell in &cells {
        if cell.verdict == Verdict::Fail {
            run_verdict = Verdict::Fail;
        }
        records.push(cell.record.to_string());
    }
    let mut gate_reasons: Vec<String> = cells
        .iter()
        .flat_map(|c| c.reasons.iter().map(move |r| format!("{}:{r}", c.key)))
        .collect();
    if cells.is_empty() {
        // rf807 先例・cargo bench 慣習: filter が 0 cell に一致した
        // 場合は gate not_run・exit 0(実行する対象が無いだけで
        // 回帰ではない)。
        run_verdict = Verdict::NotRun;
        gate_reasons.push("no_cells_matched".to_string());
    }
    records.push(
        json!({
            "kind": "gate",
            "baseline": args.baseline_path.display().to_string(),
            "cells": cells.iter().map(|c| json!({
                "cell": c.key,
                "verdict": c.verdict.as_str(),
                "reasons": c.reasons,
            })).collect::<Vec<_>>(),
            "verdict": run_verdict.as_str(),
            "reasons": gate_reasons,
            "limiter": limiter_notes,
        })
        .to_string(),
    );
    records.push(
        json!({
            "kind": "hint",
            "detail": "decision.tsv 行(id/hypothesis/change/before/after/delta/tests/kept/status) — ledger への追記は operator が行う(hillclimb schema)",
        })
        .to_string(),
    );

    // ---- write-baseline: gate の代わりに un-promoted candidate を出力 ----
    if write_baseline {
        let mut cells_out = Map::new();
        for res in &kernel_results {
            // spec binding: baseline には比と計数のみ格納し、絶対時間
            // (median_ns 系)は置かない — 実測絶対時間は evidence の
            // run 記録側が保持する。
            cells_out.insert(
                res.key.clone(),
                json!({
                    "min_ratio": kernel_threshold(res.ratio),
                    "measured_ratio": res.ratio,
                    "errors": res.errors,
                    "work": res.work,
                }),
            );
        }
        if let Some(p) = &pipeline_result {
            let mut wall_sorted = p.wall_ns.clone();
            wall_sorted.sort();
            let e2e_median_ns = median(&wall_sorted) as f64;
            let ref_med = ref_kernel_median_ns.max(1) as f64;
            let norm_e2e = e2e_median_ns / ref_med;
            let mut loc_sorted = p.locate_wall_ns.clone();
            loc_sorted.sort();
            let locate_norm = (median(&loc_sorted) as f64) / ref_med;
            let fp_map: Map<String, Value> = p
                .fingerprints
                .first()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|(k, v)| (k, Value::String(v)))
                .collect();
            cells_out.insert(
                "pipeline/e2e".to_string(),
                json!({
                    "max_norm_ratio": pipeline_threshold(norm_e2e),
                    "measured_norm_ratio": norm_e2e,
                    "max_norm_ratio_locate": pipeline_threshold(locate_norm),
                    "measured_norm_ratio_locate": locate_norm,
                    "errors": p.errors,
                    "work": p.work,
                    "fingerprint_files": fp_map.len(),
                }),
            );
            let candidate = json!({
                "schema": BASELINE_SCHEMA,
                "issue": ISSUE,
                "metric": "kernel: median(A)/median(B) ratio >= min_ratio; pipeline: wall/ref_kernel <= max_norm_ratio (+ fingerprint exact match)",
                "arms": {
                    "kernel/<method>": "A=凍結 naive NCC(stride grid+refine) vs B=al_locate 本番 method — in-process A/B、計測系で吸収しきれない回帰を比で検出",
                    "pipeline/e2e": "JobService::run(ingest..dataset) e2e+stage wall を同一セッションの naive ref median で正規化 + analysis payload fingerprint(JobId/時刻/record-only 除く)を exact-match",
                },
                "threshold_rule": "kernel: min_ratio = ceil(1+(measured-1)*0.5, 0.001); pipeline: max_norm = ceil(measured*1.5, 0.01)",
                "measured_env": measured_env(),
                "fingerprint": Value::Object(fp_map),
                "limiter": {
                    "kernel": "naive NCC は single-thread scalar — CPU bound。production method は FFT(plan reuse)+prepared 入力で支配的に速い",
                    "pipeline": "locate stage が支配(本番 baseline で e2e の 69-79%)。disk I/O・s16 WAV decode は小さい",
                    "relevance": "production `al run` と同じ JobService::run 経路 — e2e 応答時間そのもの。kernel arm はその支配 stage の hot kernel",
                },
                "cells": Value::Object(cells_out),
                "approval": null,
            });
            let path = args
                .write_baseline_path
                .as_ref()
                .expect("write-baseline path");
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            match serde_json::to_string_pretty(&candidate) {
                Ok(text) => {
                    if let Err(e) = fs::write(path, format!("{text}\n")) {
                        fail(&format!("write baseline candidate {}: {e}", path.display()));
                    }
                    eprintln!(
                        "baseline candidate written to {} — approval は null。昇格は pin path への手動複写+approval 記入",
                        path.display()
                    );
                }
                Err(e) => fail(&format!("serialize candidate: {e}")),
            }
            for r in &records {
                println!("{r}");
            }
            return ExitCode::from(0);
        }
        fail("--write-baseline needs the pipeline arm measured (do not filter it out)");
    }

    for r in &records {
        println!("{r}");
    }

    match run_verdict {
        Verdict::Pass => {
            eprintln!(
                "PASS: {} cells within thresholds (reps={reps})",
                cells.len()
            );
            ExitCode::from(0)
        }
        Verdict::Fail => {
            eprintln!("FAIL: {}", gate_reasons.join(", "));
            ExitCode::from(1)
        }
        _ => {
            eprintln!(
                "{}: {}",
                run_verdict.as_str().to_uppercase(),
                gate_reasons.join(", ")
            );
            ExitCode::from(0)
        }
    }
}
