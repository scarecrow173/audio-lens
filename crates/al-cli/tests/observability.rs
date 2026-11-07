//! Observability wiring tests: `--log`/`--log-format` and
//! `AL_LOG`/`AL_LOG_FORMAT` activation, sink routing, fail-fast
//! behavior on malformed directives, and the metadata-only field
//! invariant enforced through an in-process capture subscriber.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use al_core::{InputRole, StageKind};
use al_job::job::{JobDefinition, input};
use al_job::manifest::ArtifactRecord;
use al_job::runner::StageRunner;
use al_job::stage::{Stage, StageContext, StageError};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("al-cli-obs-{tag}-{}", al_core::JobId::new()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Same env scrub as `cli.rs`: `AL_LOG`/`AL_LOG_FORMAT` never leak
/// from the parent environment into the spawned `al` process.
fn al_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_al"));
    command.env_remove("AL_LOG");
    command.env_remove("AL_LOG_FORMAT");
    command
}

fn wav_bytes() -> Vec<u8> {
    let frames: Vec<i16> = (0..960).map(|i| i % 200 - 100).collect();
    let data_len = frames.len() as u32 * 2;
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&48000u32.to_le_bytes());
    out.extend_from_slice(&96000u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in frames {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

fn run_ingest(jobs: &Path, wav: &Path, extra_args: &[&str]) -> Output {
    let mut args = vec![
        "run".to_owned(),
        "--base-dir".to_owned(),
        jobs.to_str().unwrap().to_owned(),
        "--input".to_owned(),
        format!("capture_bug:{}", wav.display()),
        "--stages".to_owned(),
        "ingest".to_owned(),
    ];
    args.extend(extra_args.iter().map(|s| s.to_string()));
    al_command().args(&args).output().unwrap()
}

fn log_files(jobs: &Path) -> Vec<PathBuf> {
    let logs = jobs.join("logs");
    match std::fs::read_dir(&logs) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("al.") && n.ends_with(".log"))
            })
            .collect(),
        Err(_) => Vec::new(),
    }
}

#[test]
fn log_flag_enables_stderr_and_file_sinks() {
    let dir = temp_dir("flag");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(&jobs, &wav, &["--log", "info"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("stage completed") && stderr.contains("ingest"),
        "stderr sink must carry the stage completion event: {stderr}"
    );

    // The file sink is daily-rotated JSON-lines under <base_dir>/logs.
    let files = log_files(&jobs);
    assert_eq!(files.len(), 1, "expected one al.YYYY-MM-DD.log file");
    let body = std::fs::read_to_string(&files[0]).unwrap();
    assert!(
        body.contains("\"stage\":\"ingest\"") && body.contains("duration_ms"),
        "file sink must carry JSON-lines stage events: {body}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn al_log_env_activates_logging_without_flags() {
    let dir = temp_dir("env");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = al_command()
        .env("AL_LOG", "info")
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "ingest",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        log_files(&jobs).len(),
        1,
        "AL_LOG=info must build the file sink"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn logging_off_creates_no_log_files() {
    let dir = temp_dir("off");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(&jobs, &wav, &[]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.lines().any(|l| l.starts_with("job_id=")));
    assert!(stdout.lines().any(|l| l.starts_with("job_dir=")));
    assert!(
        !jobs.join("logs").exists(),
        "default OFF must not create <base_dir>/logs"
    );

    // `--log-format` alone does not activate logging.
    let out = run_ingest(&jobs, &wav, &["--log-format", "stderr=json"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !jobs.join("logs").exists(),
        "format directive without activation must stay OFF"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn inspect_logs_to_stderr_only_and_keeps_stdout_json() {
    let dir = temp_dir("inspect");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(&jobs, &wav, &[]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_id = stdout
        .lines()
        .find_map(|l| l.strip_prefix("job_id="))
        .expect("job_id line");

    let out = al_command()
        .args([
            "inspect",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--job-id",
            job_id,
            "--log",
            "info",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "inspect failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // stdout stays a clean JSON manifest even with logging enabled.
    serde_json::from_slice::<serde_json::Value>(&out.stdout)
        .expect("inspect stdout must be valid JSON");
    assert!(
        !jobs.join("logs").exists(),
        "inspect is read-only: no file sink under <base_dir>/logs"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Retention: `max_log_files(7)` prunes at appender build, keeping
/// `max - 1` pre-existing files plus the newly created daily file —
/// at most 7 `al.*.log` files remain under `<base_dir>/logs`.
#[test]
fn file_sink_retention_keeps_max_seven_files() {
    let dir = temp_dir("retention");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");
    let logs = jobs.join("logs");
    std::fs::create_dir_all(&logs).unwrap();
    for i in 1..=9 {
        std::fs::write(logs.join(format!("al.2020-01-{i:02}.log")), b"stale\n").unwrap();
    }

    let out = run_ingest(&jobs, &wav, &["--log", "info"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files = log_files(&jobs);
    assert_eq!(
        files.len(),
        7,
        "retention must cap al.*.log at 7 files: {files:?}"
    );
    let fresh = files.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| !n.starts_with("al.2020-"))
    });
    assert!(
        fresh,
        "today's rotated file must survive pruning: {files:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// The file sink must never break the job: when `<base_dir>/logs`
/// cannot be created (a regular file occupies the path), the build
/// degrades to stderr-only with an explicit `warn:` line and the
/// run still succeeds.
#[test]
fn file_sink_init_failure_degrades_to_stderr() {
    let dir = temp_dir("degrade");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");
    std::fs::create_dir_all(&jobs).unwrap();
    // `<base_dir>/logs` exists as a regular file — `create_dir_all`
    // and the appender build both fail on it.
    std::fs::write(jobs.join("logs"), b"not a directory").unwrap();

    let out = run_ingest(&jobs, &wav, &["--log", "info"]);
    assert!(
        out.status.success(),
        "run must succeed with a degraded file sink: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warn: cannot initialize file log sink"),
        "expected an explicit degrade warning: {stderr}"
    );
    assert!(
        stderr.contains("stage completed"),
        "stderr sink must keep working after file sink degradation: {stderr}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Per-sink format switching end-to-end: `stderr=json` turns stderr
/// tracing lines into JSON-lines while the file sink keeps its
/// JSON-lines default.
#[test]
fn stderr_json_format_switch() {
    let dir = temp_dir("stderr-json");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(
        &jobs,
        &wav,
        &["--log", "info", "--log-format", "stderr=json"],
    );
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let mut json_lines = 0;
    for line in stderr.lines().filter(|l| !l.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line)
            .unwrap_or_else(|_| panic!("stderr line must be JSON-lines: {line:?}\nfull: {stderr}"));
        json_lines += 1;
    }
    assert!(json_lines > 0, "stderr must carry JSON-lines events");
    assert_eq!(log_files(&jobs).len(), 1, "file sink must still write");

    std::fs::remove_dir_all(&dir).ok();
}

/// Per-sink format switching: `file=text` switches the file sink to
/// human text while stderr keeps its text default.
#[test]
fn file_text_format_switch() {
    let dir = temp_dir("file-text");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(&jobs, &wav, &["--log", "info", "--log-format", "file=text"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files = log_files(&jobs);
    assert_eq!(files.len(), 1);
    let body = std::fs::read_to_string(&files[0]).unwrap();
    let first = body.lines().next().expect("file sink must emit lines");
    assert!(
        !first.trim_start().starts_with('{'),
        "file=text must not produce JSON-lines: {first:?}"
    );
    assert!(
        body.contains("stage completed"),
        "text file sink must carry stage events: {body}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Flags replace their env vars wholesale per concern: a broken
/// `AL_LOG`/`AL_LOG_FORMAT` must not be evaluated when `--log`/
/// `--log-format` are given, and `AL_LOG=""` is unspecified (OFF).
#[test]
fn flag_overrides_env_and_empty_env_is_off() {
    let dir = temp_dir("priority");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    // Broken env values are never parsed because the flags win.
    let out = al_command()
        .env("AL_LOG", "=broken")
        .env("AL_LOG_FORMAT", "stderr=json")
        .args([
            "run",
            "--base-dir",
            jobs.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "ingest",
            "--log",
            "info",
            "--log-format",
            "file=text",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "flag must override broken env wholesale: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files = log_files(&jobs);
    assert_eq!(files.len(), 1, "flag activation must build the file sink");
    let body = std::fs::read_to_string(&files[0]).unwrap();
    let first = body.lines().next().expect("file sink must emit lines");
    assert!(
        !first.trim_start().starts_with('{'),
        "--log-format file=text must replace AL_LOG_FORMAT: {first:?}"
    );

    // `AL_LOG=""` counts as unspecified — logging stays OFF.
    let jobs_off = dir.join("jobs-off");
    let out = al_command()
        .env("AL_LOG", "")
        .args([
            "run",
            "--base-dir",
            jobs_off.to_str().unwrap(),
            "--input",
            &format!("capture_bug:{}", wav.display()),
            "--stages",
            "ingest",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(
        !jobs_off.join("logs").exists(),
        "AL_LOG=\"\" must keep logging OFF"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Leaf-span observability on stderr: the ruviform stderr sink is
/// configured with `SpanEvents::CLOSE`, restoring the
/// pre-migration `FmtSpan::CLOSE` behaviour (ruviform#335), so
/// span close records carrying `time.busy`/`time.idle` reach
/// stderr as well as the file sink.
#[test]
fn stderr_sink_emits_span_close_events() {
    let dir = temp_dir("stderr-span-close");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(&jobs, &wav, &["--log", "info"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("time.busy"),
        "stderr sink must carry span close records: {stderr}"
    );

    // Close records inherit the span's level/target and pass the
    // same per-sink EnvFilter: the `stage`/`cli::run` spans are
    // INFO, so `--log warn` must suppress their close records.
    let jobs_warn = dir.join("jobs-warn");
    let out = run_ingest(&jobs_warn, &wav, &["--log", "warn"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("time.busy"),
        "filtered-out spans must not emit close records: {stderr}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Leaf-span observability: the file JSON-lines sink emits
/// `FmtSpan::CLOSE` lifecycle events (with `time.busy`/`time.idle`
/// fields) so spans with no inner events still leave a trace.
#[test]
fn file_sink_emits_span_close_events() {
    let dir = temp_dir("span-close");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    let out = run_ingest(&jobs, &wav, &["--log", "info"]);
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let files = log_files(&jobs);
    assert_eq!(files.len(), 1);
    let body = std::fs::read_to_string(&files[0]).unwrap();
    assert!(
        body.contains("\"time.busy\""),
        "file sink must carry span close events: {body}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn malformed_log_options_fail_fast() {
    let dir = temp_dir("malformed");
    let wav = dir.join("cap.wav");
    std::fs::write(&wav, wav_bytes()).unwrap();
    let jobs = dir.join("jobs");

    // Each case: extra args appended to a valid `run` invocation, and
    // the expected `error:` line prefix. All must fail with exit 2 and
    // never panic.
    let cases: Vec<(Vec<String>, &str)> = vec![
        (vec!["--log".into()], "--log requires a value"),
        (vec!["--log-format".into()], "--log-format requires a value"),
        (vec!["--log".into(), "".into()], "invalid log directive"),
        (
            vec![
                "--log".into(),
                "info".into(),
                "--log-format".into(),
                "".into(),
            ],
            "malformed log format directive",
        ),
        (
            vec![
                "--log".into(),
                "info".into(),
                "--log-format".into(),
                "stdout=text".into(),
            ],
            "unknown log sink",
        ),
        (
            vec![
                "--log".into(),
                "info".into(),
                "--log-format".into(),
                "file=bogus".into(),
            ],
            "unknown log format",
        ),
        (
            vec!["--log".into(), "info=bogus".into()],
            "invalid log directive",
        ),
    ];
    for (extra, expected) in cases {
        let out = run_ingest(
            &jobs,
            &wav,
            &extra.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "case {extra:?}: {stderr}");
        assert!(
            stderr.contains(&format!("error: {expected}")),
            "case {extra:?} missing error line: {stderr}"
        );
    }

    // Flags are only interpreted after the subcommand: `al --log x run`
    // is still an unknown subcommand.
    let out = al_command().args(["--log", "x", "run"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unknown subcommand"),
        "expected unknown subcommand: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Visitor collecting `(field name, rendered size)` for every recorded
/// field, to assert emitted events and span attributes stay
/// metadata-only (lengths, counts, hashes, IDs — never raw payload).
#[derive(Default)]
struct FieldSizes(Vec<(String, usize)>);

impl tracing::field::Visit for FieldSizes {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0
            .push((field.name().to_owned(), format!("{value:?}").len()));
    }
}

/// Layer that captures the rendered size of every event and span field
/// emitted under the subscriber.
struct FieldSizeLayer {
    fields: Arc<Mutex<FieldSizes>>,
}

impl<S> Layer<S> for FieldSizeLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = FieldSizes::default();
        event.record(&mut visitor);
        self.fields.lock().unwrap().0.extend(visitor.0);
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        _id: &tracing::span::Id,
        _ctx: Context<'_, S>,
    ) {
        let mut visitor = FieldSizes::default();
        attrs.record(&mut visitor);
        self.fields.lock().unwrap().0.extend(visitor.0);
    }
}

struct StubStage {
    kind: StageKind,
    fail: bool,
}

impl Stage for StubStage {
    fn kind(&self) -> StageKind {
        self.kind
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        if self.fail {
            return Err(StageError::new(self.kind, "stub failure"));
        }
        Ok(vec![ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "stub_marker".into(),
            path: PathBuf::from("intermediate/stub.json"),
            produced_by: Some(self.kind),
            inputs: vec![],
            params: serde_json::json!({}),
        }])
    }
}

fn sample_def(dir: &Path) -> JobDefinition {
    std::fs::create_dir_all(dir).unwrap();
    let capture = dir.join("capture.mkv");
    std::fs::write(&capture, b"fixture capture").unwrap();
    JobDefinition::create(
        vec![input(InputRole::CaptureBug, capture)],
        serde_json::json!({}),
        vec![StageKind::Ingest],
    )
    .unwrap()
}

/// Metadata-only regression: every field emitted on the instrumented
/// runner path (stage span + completion/failure events) must stay
/// under 1 KiB — no raw PCM, waveforms, or large buffers may leak into
/// recorded fields.
#[test]
fn emitted_fields_stay_metadata_sized() {
    const MAX_FIELD_BYTES: usize = 1024;

    let fields = Arc::new(Mutex::new(FieldSizes::default()));
    let subscriber = tracing_subscriber::registry().with(FieldSizeLayer {
        fields: fields.clone(),
    });

    let dir = temp_dir("capture");
    tracing::subscriber::with_default(subscriber, || {
        let runner = StageRunner::new(&dir);
        // Success path: emits the `stage` span + `stage completed` event.
        let def = sample_def(&dir.join("ok"));
        let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(StubStage {
            kind: StageKind::Ingest,
            fail: false,
        })];
        runner.run(&def, &mut stages).unwrap();
        // Failure path: emits the `stage failed` error event.
        let def = sample_def(&dir.join("fail"));
        let mut stages: Vec<Box<dyn Stage>> = vec![Box::new(StubStage {
            kind: StageKind::Ingest,
            fail: true,
        })];
        assert!(runner.run(&def, &mut stages).is_err());
    });

    let fields = fields.lock().unwrap();
    assert!(
        fields.0.iter().any(|(name, _)| name == "duration_ms"),
        "expected stage completion fields to be captured: {:?}",
        fields.0
    );
    for (name, size) in &fields.0 {
        assert!(
            *size <= MAX_FIELD_BYTES,
            "field {name} renders to {size} bytes (> {MAX_FIELD_BYTES} B)"
        );
    }

    std::fs::remove_dir_all(&dir).ok();
}
