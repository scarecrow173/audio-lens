use std::path::{Path, PathBuf};
use std::process::ExitCode;

use al_core::{InputId, JobId};
use al_service::jobs::{JobService, RunControl, RunSpec};
use al_service::{parse_input_spec, parse_stage};

const USAGE: &str = "\
al — AudioLens job runner (foundation CLI)

USAGE:
  al run --base-dir <dir> --input <role>:<path>[,<role>:<path>...] \\
        [--stages <stage>[,<stage>...]] [--config <file>] [--input-ids <IN-...>[,<IN-...>...]]
  al resume --base-dir <dir> --job-id <JOB-...>
  al export --base-dir <dir> --job-id <JOB-...>
  al inspect --base-dir <dir> --job-id <JOB-...>

STAGES:
  ingest, locate, cheap_analysis, reference_analysis, expensive_analysis,
  dataset, export

CONFIG:
  A JSON file passed via --config. Keys:
    analysis.avsync.markers
                 [{input_id, clock: \"video_stream\", time_s, frame}].
                 time_s includes video dwStart; frame is a decoded frame index.
                 Use --input-ids to assign matching IDs in --input order.
    dictionary   Path to a capture-artifact dictionary JSON. The file's
                 content is snapshotted into the job at run time, so resume
                 does not depend on the file and the job stays reproducible.
                 Defaults to the dictionary bundled with the binary.

LOGGING:
  --log <directive>        Enable logging with a tracing EnvFilter
                           directive (e.g. \"info\", \"al_locate=debug,info\").
                           Overrides the AL_LOG environment variable.
  --log-format <sink>=<format>[,<sink>=<format>...]
                           Per-sink output format. Sinks: stderr, file.
                           Formats: text, json. Defaults: stderr=text,
                           file=json. Overrides the AL_LOG_FORMAT
                           environment variable.
  run/resume log to stderr and <base-dir>/logs/al.YYYY-MM-DD.log;
  inspect logs to stderr only. Logging is off unless --log or AL_LOG
  is given.

NOTE:
  All stages are implemented: ingest (WAV/PCM, AVI/PCM+uncompressed
  video — see docs for container scope), locate (source
  localization), cheap analysis (always-on observations), reference
  analysis (aligned comparison), expensive analysis (candidate-
  driven deep dive), dataset (analysis manifest), and export
  (relocatable package under export/<name>/ with golden snapshot).

  `al export` は job の plan に export が無い場合でも単独で export
  stage を実行し、可搬パッケージを生成する(dataset stage 完了が
  前提)。表示・検査は persisted Analysis Dataset を読む GUI
  (al-gui) が担う。

  run/resume/export/inspect は `al-service` の JobService を経由する
  — GUI (al-gui) と同一の Job サービス層。
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

/// Interpret `--log`/`--log-format` and AL_LOG/AL_LOG_FORMAT and
/// initialize logging. `file_sink` controls whether a file sink under
/// `<base_dir>/logs` is built (`run`/`resume`: true, `inspect`: false —
/// stderr only). Returns `Ok(None)` when logging is not activated; the
/// caller holds the returned guard for the rest of the `cmd_*` scope so
/// the non-blocking file writer flushes on drop.
fn init_logging(
    args: &[String],
    base_dir: &Path,
    file_sink: bool,
) -> Result<Option<al_observability::LogGuard>, String> {
    if flag_present(args, "--log") && parse_flag(args, "--log").is_none() {
        return Err("--log requires a value".to_owned());
    }
    if flag_present(args, "--log-format") && parse_flag(args, "--log-format").is_none() {
        return Err("--log-format requires a value".to_owned());
    }
    let activation = al_observability::resolve_activation(
        std::env::var("AL_LOG").ok().as_deref(),
        parse_flag(args, "--log"),
        std::env::var("AL_LOG_FORMAT").ok().as_deref(),
        parse_flag(args, "--log-format"),
    )
    .map_err(|e| e.to_string())?;
    match activation {
        al_observability::Activation::Disabled => Ok(None),
        al_observability::Activation::Enabled(mut config) => {
            if file_sink {
                config.file_dir = Some(base_dir.join("logs"));
            }
            al_observability::init(*config)
                .map(Some)
                .map_err(|e| e.to_string())
        }
    }
}

fn cmd_run(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    let _log_guard = match init_logging(args, &base_dir, true) {
        Ok(guard) => guard,
        Err(e) => return fail(e),
    };
    // The dispatch span is created after `init_logging`: a span built
    // before the subscriber is installed stays inert and `record` is a
    // no-op on it.
    let span = tracing::info_span!(
        "cli::run",
        job_id = tracing::field::Empty,
        len = tracing::field::Empty
    );
    let _entered = span.enter();
    let Some(raw_inputs) = parse_flag(args, "--input") else {
        return fail("--input is required");
    };
    let mut inputs = match raw_inputs
        .split(',')
        .map(parse_input_spec)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(inputs) => inputs,
        Err(e) => return fail(e),
    };
    if flag_present(args, "--input-ids") && parse_flag(args, "--input-ids").is_none() {
        return fail("--input-ids requires a value");
    }
    if let Some(raw_ids) = parse_flag(args, "--input-ids") {
        let ids = match raw_ids
            .split(',')
            .map(InputId::parse)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(ids) => ids,
            Err(e) => return fail(format!("invalid --input-ids: {e}")),
        };
        if let Err(e) = JobService::assign_input_ids(&mut inputs, ids) {
            return fail(e);
        }
    }
    span.record("len", inputs.len() as u64);
    let stage_kinds: Option<Vec<al_core::StageKind>> = match parse_flag(args, "--stages") {
        Some(raw) => match raw
            .split(',')
            .map(parse_stage)
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(kinds) => Some(kinds),
            Err(e) => return fail(e),
        },
        // `None` はサービス側の既定 stage 集合(= 従来の既定)に委ねる。
        None => None,
    };
    // An explicitly given --config must not silently fall back to
    // defaults: unreadable or unparsable files are an error.
    let config = match parse_flag(args, "--config") {
        Some(path) => {
            let text = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(e) => return fail(format!("cannot read config {path}: {e}")),
            };
            match serde_json::from_str(&text) {
                Ok(config) => config,
                Err(e) => return fail(format!("invalid config JSON {path}: {e}")),
            }
        }
        None => serde_json::json!({}),
    };

    let service = JobService::new(&base_dir);
    let spec = RunSpec {
        inputs,
        stages: stage_kinds,
        config,
    };
    match service.run(spec, &RunControl::default()) {
        Ok((job_id, manifest)) => {
            println!("job_id={job_id}");
            println!("job_dir={}", service.job_dir(&job_id).display());
            let _ = manifest;
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn cmd_resume(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    let _log_guard = match init_logging(args, &base_dir, true) {
        Ok(guard) => guard,
        Err(e) => return fail(e),
    };
    let span = tracing::info_span!("cli::resume", job_id = tracing::field::Empty);
    let _entered = span.enter();
    let Some(job_id) = parse_flag(args, "--job-id") else {
        return fail("--job-id is required");
    };
    let job_id = match JobId::parse(job_id) {
        Ok(id) => id,
        Err(e) => return fail(e),
    };
    span.record("job_id", job_id.as_str());
    let service = JobService::new(&base_dir);
    match service.resume(&job_id, &RunControl::default()) {
        Ok(manifest) => {
            println!("job_id={}", manifest.job_id);
            for record in &manifest.stages {
                println!("{:?}: {:?}", record.stage, record.status);
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn cmd_export(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    let _log_guard = match init_logging(args, &base_dir, true) {
        Ok(guard) => guard,
        Err(e) => return fail(e),
    };
    let span = tracing::info_span!("cli::export", job_id = tracing::field::Empty);
    let _entered = span.enter();
    let Some(job_id) = parse_flag(args, "--job-id") else {
        return fail("--job-id is required");
    };
    let job_id = match JobId::parse(job_id) {
        Ok(id) => id,
        Err(e) => return fail(e),
    };
    span.record("job_id", job_id.as_str());
    let service = JobService::new(&base_dir);
    match service.export(&job_id, &RunControl::default()) {
        Ok(manifest) => {
            println!("job_id={}", manifest.job_id);
            for record in &manifest.stages {
                if record.stage == al_core::StageKind::Export {
                    println!("{:?}: {:?}", record.stage, record.status);
                }
            }
            for artifact in &manifest.artifacts {
                if artifact.role == "export_package" {
                    println!(
                        "export={}",
                        service.job_dir(&job_id).join(&artifact.path).display()
                    );
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn cmd_inspect(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    // `inspect` is read-only: stderr sink only, no file sink is built.
    let _log_guard = match init_logging(args, &base_dir, false) {
        Ok(guard) => guard,
        Err(e) => return fail(e),
    };
    let span = tracing::info_span!("cli::inspect", job_id = tracing::field::Empty);
    let _entered = span.enter();
    let Some(job_id) = parse_flag(args, "--job-id") else {
        return fail("--job-id is required");
    };
    let job_id = match JobId::parse(job_id) {
        Ok(id) => id,
        Err(e) => return fail(e),
    };
    span.record("job_id", job_id.as_str());
    let service = JobService::new(&base_dir);
    match service.inspect(&job_id).and_then(|i| {
        serde_json::to_string_pretty(&i.manifest)
            .map_err(std::io::Error::other)
            .map_err(al_service::ServiceError::from)
    }) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || flag_present(&args, "--help") || flag_present(&args, "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    match args[0].as_str() {
        "run" => cmd_run(&args[1..]),
        "resume" => cmd_resume(&args[1..]),
        "export" => cmd_export(&args[1..]),
        "inspect" => cmd_inspect(&args[1..]),
        other => fail(format!("unknown subcommand {other:?}\n{USAGE}")),
    }
}
