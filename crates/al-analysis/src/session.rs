//! Shared dataset session for analysis stages: resolves the job's
//! `analysis/` dataset root with validated open semantics so every
//! stage appends to the same canonical dataset.

use std::io;

use al_dataset::provenance::input_references;
use al_dataset::writer::DatasetWriter;
use al_job::stage::StageContext;

/// Open (or create) the job's Analysis Dataset for appending.
///
/// The manifest's schema version, inputs, and config must match the
/// job exactly — mismatches and corrupt manifests fail explicitly.
pub fn open_analysis_dataset(ctx: &StageContext<'_>) -> io::Result<DatasetWriter> {
    let root = ctx.registry.job_dir().join("analysis");
    let inputs = input_references(ctx.manifest)?;
    DatasetWriter::open_or_create(&root, inputs, ctx.config.clone())
}
