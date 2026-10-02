use std::path::PathBuf;
use std::process::ExitCode;

use al_core::{InputId, InputRole, JobId, StageKind};
use al_job::job::{JobDefinition, JobInput};
use al_job::manifest::{ArtifactRecord, Manifest};
use al_job::runner::StageRunner;
use al_job::stage::{Stage, StageContext, StageError};
use al_media::IngestStage;

const USAGE: &str = "\
al — AudioLens job runner (foundation CLI)

USAGE:
  al run --base-dir <dir> --input <role>:<path>[,<role>:<path>...] \\
        [--stages <stage>[,<stage>...]] [--config <file>]
  al resume --base-dir <dir> --job-id <JOB-...>
  al inspect --base-dir <dir> --job-id <JOB-...>

STAGES:
  ingest, locate, cheap_analysis, reference_analysis, expensive_analysis,
  dataset, visualization, export

CONFIG:
  A JSON file passed via --config. Keys:
    dictionary   Path to a capture-artifact dictionary JSON. The file's
                 content is snapshotted into the job at run time, so resume
                 does not depend on the file and the job stays reproducible.
                 Defaults to the dictionary bundled with the binary.

NOTE:
  The ingest stage is implemented (WAV/PCM). All other stages are stubs
  that emit marker artifacts; real stages are added by later plans.
";

/// Foundation stub stage: emits one marker artifact per run so the runner,
/// manifest, and registry can be exercised end to end.
struct StubStage {
    kind: StageKind,
}

impl Stage for StubStage {
    fn kind(&self) -> StageKind {
        self.kind
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let id = ctx.registry.issue();
        let rel = format!(
            "intermediate/{}_{}.json",
            serde_json::to_value(self.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| format!("{:?}", self.kind)),
            id.as_str()
        );
        Ok(vec![ArtifactRecord {
            artifact_id: id,
            role: "stub_marker".into(),
            path: PathBuf::from(rel),
            produced_by: Some(self.kind),
            inputs: vec![],
            params: serde_json::json!({"stub": true}),
        }])
    }
}

/// Snapshot the external dictionary into the job config.
///
/// `config.dictionary` names a file; at `run` time its content is copied
/// into `config.dictionary_text` (with `config.dictionary_sha256`) so the
/// manifest — and therefore `resume` — is pinned to the exact bytes that
/// ran, regardless of what happens to the file afterwards.
fn resolve_config(config: &mut serde_json::Value) -> Result<(), String> {
    // Always pin the dictionary into the job config: an explicit
    // `dictionary` path is read now, otherwise the bundled dictionary is
    // used. Either way `resume` replays the snapshot inside the manifest
    // rather than re-resolving a file or the then-current binary.
    let text = match config.get("dictionary").and_then(|v| v.as_str()) {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read dictionary {path}: {e}"))?,
        None => al_media::Dictionary::EMBEDDED_JSON.to_owned(),
    };
    let obj = config
        .as_object_mut()
        .ok_or_else(|| "config must be a JSON object".to_owned())?;
    obj.insert(
        "dictionary_sha256".into(),
        serde_json::json!(al_media::sha256_hex(text.as_bytes())),
    );
    obj.insert("dictionary_text".into(), serde_json::json!(text));
    Ok(())
}

/// Build the stage implementation for `kind`. Stages without a real
/// implementation fall back to the foundation stub.
fn make_stage(kind: StageKind, config: &serde_json::Value) -> Result<Box<dyn Stage>, String> {
    if kind == StageKind::Ingest {
        // `dictionary_text` (the run-time snapshot) pins the dictionary
        // content; `dictionary` remains usable for configs that never went
        // through `resolve_config`. Default is the bundled dictionary, so
        // results never depend on the working directory.
        let stage = if let Some(text) = config.get("dictionary_text").and_then(|v| v.as_str()) {
            let dictionary = al_media::Dictionary::from_json(text)
                .map_err(|e| format!("invalid snapshotted dictionary: {e}"))?;
            IngestStage::with_dictionary(dictionary)
        } else if let Some(path) = config.get("dictionary").and_then(|v| v.as_str()) {
            IngestStage::with_dictionary_file(path).map_err(|e| e.to_string())?
        } else {
            IngestStage::with_dictionary(al_media::Dictionary::embedded())
        };
        return Ok(Box::new(stage));
    }
    Ok(Box::new(StubStage { kind }))
}

fn fail(message: impl std::fmt::Display) -> ExitCode {
    eprintln!("error: {message}");
    ExitCode::from(2)
}

fn parse_stage(value: &str) -> Result<StageKind, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| format!("unknown stage {value:?}"))
}

fn parse_input(value: &str) -> Result<JobInput, String> {
    let (role, path) = value
        .split_once(':')
        .ok_or_else(|| format!("input {value:?} must be <role>:<path>"))?;
    let role: InputRole = serde_json::from_value(serde_json::Value::String(role.to_owned()))
        .map_err(|_| format!("unknown input role {role:?}"))?;
    // Pin the input's content hash at job creation so a later resume
    // cannot silently ingest a replaced file under the same job id.
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read input {path}: {e}"))?;
    Ok(JobInput {
        input_id: InputId::new(),
        role,
        path: PathBuf::from(path),
        source_sha256: Some(al_media::sha256_hex(&bytes)),
    })
}

fn parse_flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
}

fn flag_present(args: &[String], name: &str) -> bool {
    args.iter().any(|arg| arg == name)
}

fn cmd_run(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    let Some(raw_inputs) = parse_flag(args, "--input") else {
        return fail("--input is required");
    };
    let inputs = match raw_inputs
        .split(',')
        .map(parse_input)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(inputs) => inputs,
        Err(e) => return fail(e),
    };
    let stage_names: Vec<&str> = parse_flag(args, "--stages")
        .map(|s| s.split(',').collect())
        .unwrap_or_else(|| {
            vec![
                "ingest",
                "locate",
                "cheap_analysis",
                "reference_analysis",
                "expensive_analysis",
                "dataset",
                "visualization",
                "export",
            ]
        });
    let stage_kinds = match stage_names
        .into_iter()
        .map(parse_stage)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(kinds) => kinds,
        Err(e) => return fail(e),
    };
    // An explicitly given --config must not silently fall back to
    // defaults: unreadable or unparsable files are an error.
    let mut config = match parse_flag(args, "--config") {
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
    if let Err(e) = resolve_config(&mut config) {
        return fail(e);
    }

    let def = JobDefinition::create(inputs, config.clone(), stage_kinds.clone());
    let mut stages: Vec<Box<dyn Stage>> = Vec::new();
    for kind in &stage_kinds {
        match make_stage(*kind, &config) {
            Ok(stage) => stages.push(stage),
            Err(e) => return fail(e),
        }
    }
    let runner = StageRunner::new(base_dir);
    match runner.run(&def, &mut stages) {
        Ok(manifest) => {
            println!("job_id={}", manifest.job_id);
            println!("job_dir={}", runner.job_dir(&manifest.job_id).display());
            ExitCode::SUCCESS
        }
        Err(e) => fail(e),
    }
}

fn load_definition(base_dir: &PathBuf, job_id: &str) -> Result<(JobDefinition, PathBuf), String> {
    let job_id = JobId::parse(job_id).map_err(|e| e.to_string())?;
    let runner = StageRunner::new(base_dir);
    let job_dir = runner.job_dir(&job_id);
    let manifest = Manifest::load(&job_dir).map_err(|e| format!("cannot load manifest: {e}"))?;
    let def = JobDefinition {
        job_id: job_id.clone(),
        inputs: manifest.inputs.clone(),
        config: manifest.config.clone(),
        stages: manifest.stages.iter().map(|s| s.stage).collect(),
    };
    Ok((def, job_dir))
}

fn cmd_resume(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    let Some(job_id) = parse_flag(args, "--job-id") else {
        return fail("--job-id is required");
    };
    let (def, _) = match load_definition(&base_dir, job_id) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let mut stages: Vec<Box<dyn Stage>> = Vec::new();
    for kind in &def.stages {
        match make_stage(*kind, &def.config) {
            Ok(stage) => stages.push(stage),
            Err(e) => return fail(e),
        }
    }
    let runner = StageRunner::new(base_dir);
    match runner.resume(&def, &mut stages) {
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

fn cmd_inspect(args: &[String]) -> ExitCode {
    let Some(base_dir) = parse_flag(args, "--base-dir").map(PathBuf::from) else {
        return fail("--base-dir is required");
    };
    let Some(job_id) = parse_flag(args, "--job-id") else {
        return fail("--job-id is required");
    };
    let (_, job_dir) = match load_definition(&base_dir, job_id) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    match Manifest::load(&job_dir)
        .and_then(|m| serde_json::to_string_pretty(&m).map_err(std::io::Error::other))
    {
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
        "inspect" => cmd_inspect(&args[1..]),
        other => fail(format!("unknown subcommand {other:?}\n{USAGE}")),
    }
}
