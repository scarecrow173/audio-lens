//! Relocatable export package (`export/<name>/`) and the `ExportStage`
//! that builds it. The package is a portability responsibility — it
//! copies the materialized analysis dataset, shrinks host paths, and
//! records hashes — not a display one.

pub mod golden;
pub mod package;

use std::io;
use std::path::PathBuf;

use al_core::StageKind;
use al_job::manifest::ArtifactRecord;
use al_job::stage::{Stage, StageContext, StageError};

use package::ExportConfig;

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// The export stage.
#[derive(Debug, Default)]
pub struct ExportStage;

impl ExportStage {
    /// Create the stage.
    pub fn new() -> Self {
        Self
    }
}

impl Stage for ExportStage {
    fn kind(&self) -> StageKind {
        StageKind::Export
    }

    /// The package reads only the materialized analysis dataset — the
    /// pipeline-order default would also demand the analysis stages,
    /// which an ingest+dataset job legitimately skips (#103).
    fn depends_on(&self) -> Vec<StageKind> {
        vec![StageKind::Dataset]
    }

    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError> {
        let err = |e: io::Error| StageError::new(StageKind::Export, e.to_string());
        let cfg = ExportConfig::from_job(ctx.config);
        let job_dir = ctx.registry.job_dir();
        let (dest, files) = package::build(job_dir, ctx.manifest, &cfg).map_err(err)?;

        // Golden snapshot over the package (excluding itself).
        let only: Vec<String> = ctx.config["export"]["golden_paths"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let snap = golden::snapshot(&dest, &only).map_err(err)?;
        let golden_rel = "golden/snapshot.json";
        std::fs::create_dir_all(dest.join("golden")).map_err(err)?;
        std::fs::write(
            dest.join(golden_rel),
            serde_json::to_string_pretty(&snap).map_err(|e| invalid(e.to_string()))?,
        )
        .map_err(err)?;

        let rel = |p: &str| PathBuf::from(format!("export/{}/{p}", cfg.name));
        let mut records = Vec::new();
        for (path, role) in [
            ("export_manifest.json", "export_manifest"),
            (golden_rel, "export_golden"),
        ] {
            records.push(ArtifactRecord {
                artifact_id: ctx.registry.issue(),
                role: role.into(),
                path: rel(path),
                produced_by: Some(StageKind::Export),
                inputs: Vec::new(),
                params: serde_json::json!({}),
            });
        }
        records.push(ArtifactRecord {
            artifact_id: ctx.registry.issue(),
            role: "export_package".into(),
            path: PathBuf::from(format!("export/{}", cfg.name)),
            produced_by: Some(StageKind::Export),
            inputs: Vec::new(),
            params: serde_json::json!({
                "package": cfg.name,
                "files": files.len(),
                "copy_media": cfg.copy_media,
            }),
        });
        Ok(records)
    }
}
