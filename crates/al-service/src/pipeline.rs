//! Pipeline 構成の正本。`al-cli` と `al-gui` が同一の stage 集合・
//! config 解決・stage 構築を共有するため、ここに集約する。
//! (旧 `al-cli::main.rs` の `resolve_config`/`make_stage`/`parse_stage`
//! /`parse_input` を移設)

use std::path::PathBuf;

use al_core::{InputRole, StageKind};
use al_dataset::DatasetStage;
use al_job::job::{JobInput, input};
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};
use al_locate::LocateStage;
use al_media::IngestStage;

/// `--stages` 未指定時の既定 stage 集合(pipeline order)。
/// CLI の既定値と同一: 全実装済み stage。
pub fn default_stage_kinds() -> Vec<StageKind> {
    vec![
        StageKind::Ingest,
        StageKind::Locate,
        StageKind::CheapAnalysis,
        StageKind::ReferenceAnalysis,
        StageKind::ExpensiveAnalysis,
        StageKind::Dataset,
        StageKind::Visualization,
        StageKind::Export,
    ]
}

/// stage 名(serde snake_case)を `StageKind` に変換する。
pub fn parse_stage(value: &str) -> Result<StageKind, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|_| format!("unknown stage {value:?}"))
}

/// `<role>:<path>` 形式の input 指定を `JobInput` に変換する。
/// `JobDefinition::create` が内容 hash を pin するため、ここでは
/// role と path の解釈のみ行う。
pub fn parse_input_spec(value: &str) -> Result<JobInput, String> {
    let (role, path) = value
        .split_once(':')
        .ok_or_else(|| format!("input {value:?} must be <role>:<path>"))?;
    let role: InputRole = serde_json::from_value(serde_json::Value::String(role.to_owned()))
        .map_err(|_| format!("unknown input role {role:?}"))?;
    Ok(input(role, path))
}

/// Snapshot the external dictionary into the job config.
///
/// `config.dictionary` names a file; at `run` time its content is copied
/// into `config.dictionary_text` (with `config.dictionary_sha256`) so the
/// manifest — and therefore `resume` — is pinned to the exact bytes that
/// ran, regardless of what happens to the file afterwards.
pub fn resolve_config(config: &mut serde_json::Value) -> Result<(), String> {
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
pub fn make_stage(kind: StageKind, config: &serde_json::Value) -> Result<Box<dyn Stage>, String> {
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
    if kind == StageKind::Locate {
        return Ok(Box::new(LocateStage::new()));
    }
    if kind == StageKind::CheapAnalysis {
        return Ok(Box::new(al_analysis::stage::CheapAnalysisStage::new()));
    }
    if kind == StageKind::ReferenceAnalysis {
        return Ok(Box::new(al_analysis::stage::ReferenceAnalysisStage::new()));
    }
    if kind == StageKind::ExpensiveAnalysis {
        return Ok(Box::new(al_analysis::stage::ExpensiveAnalysisStage::new()));
    }
    if kind == StageKind::Dataset {
        return Ok(Box::new(DatasetStage::new()));
    }
    if kind == StageKind::Visualization {
        return Ok(Box::new(al_viz::stage::VisualizationStage::new()));
    }
    if kind == StageKind::Export {
        return Ok(Box::new(al_viz::stage::ExportStage::new()));
    }
    Ok(Box::new(StubStage { kind }))
}

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
