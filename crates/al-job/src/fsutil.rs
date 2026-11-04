//! Staged file publish for job-owned artifacts.
//!
//! Writes go through the `ruviform-io-file` type-state contract
//! (`StagedFile` → `SealedFile::publish`): bytes land in a temp
//! sibling on the same filesystem and only the requested publish
//! options make them visible at the destination — a process kill can
//! never leave a truncated file readers would accept as complete.
//! Job execution is serialized by the runner's advisory lock, so
//! per-destination staging needs no extra coordination.
//!
//! Callers pick [`PublishOptions`] per artifact class — there is no
//! implicit mapping: `PublishMode::Replace` for the create-or-replace
//! job stores, `Durability::FileAndDirectorySync` for the commit
//! records resume trusts (manifests, store markers, metadata JSON),
//! and `Durability::BestEffort` for regenerable bulk payloads the
//! resume-time artifact verification re-creates on loss
//! (docs/designs/artifact-durability-tiers-resume-verification.md).

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

pub use ruviform_io_file::{
    Durability, DurabilityEvidence, OperationId, PublishEffect, PublishFailureKind, PublishMode,
    PublishOptions, PublishPhase, PublishReceipt, SealedFile, StagedFile, TempState,
};
use ruviform_io_file::{FileError, FileErrorKind, PublishFailure};

/// Join `rel` to `job_dir`, rejecting paths that would escape it.
///
/// Persisted manifests are untrusted input on resume: an absolute path
/// or `..` component must never read or write outside the job.
pub fn checked_job_path(job_dir: &Path, rel: &Path) -> io::Result<PathBuf> {
    if rel.is_absolute()
        || rel
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("path escapes job dir: {}", rel.display()),
        ));
    }
    Ok(job_dir.join(rel))
}

/// A staged-publish failure with the ruviform outcome classification
/// preserved: the failing step, publish effect, achieved durability
/// and post-policy temp state stay structured — never flattened into
/// a bare `io::Error` the job's resume/retry diagnostics could not
/// read back.
#[derive(Debug)]
pub struct PublishError {
    inner: PublishErrorRepr,
}

#[derive(Debug)]
enum PublishErrorRepr {
    /// The publish never ran: parent-dir creation, temp open, or the
    /// write phase failed. The destination was untouched and
    /// `StagedFile`'s drop already removed the temp.
    Staging { source: io::Error },
    /// `SealedFile::publish` reported a classified failure after the
    /// AudioLens temp-recovery policy ran (see [`publish_sealed`]).
    Publish(PublishRecord),
}

/// The classification extracted from a [`PublishFailure`]: every
/// field the job's resume/retry policy and stage-error diagnostics
/// need, without holding the platform ops the failure owns.
#[derive(Debug)]
struct PublishRecord {
    kind: PublishFailureKind,
    phase: PublishPhase,
    effect: PublishEffect,
    durability: DurabilityEvidence,
    operation: OperationId,
    destination: PathBuf,
    temp_path: Option<PathBuf>,
    temp_state: TempState,
    detail: Option<String>,
}

impl PublishError {
    fn staging(source: io::Error) -> Self {
        Self {
            inner: PublishErrorRepr::Staging { source },
        }
    }

    /// What the failed operation did to the destination —
    /// [`PublishEffect::NotPublished`] also covers staging/write
    /// failures that never reached the publish pipeline.
    pub fn effect(&self) -> PublishEffect {
        match &self.inner {
            PublishErrorRepr::Staging { .. } => PublishEffect::NotPublished,
            PublishErrorRepr::Publish(record) => record.effect,
        }
    }

    /// The publish-pipeline phase that failed, or `None` for
    /// staging/write failures before publish ran.
    pub fn phase(&self) -> Option<PublishPhase> {
        match &self.inner {
            PublishErrorRepr::Staging { .. } => None,
            PublishErrorRepr::Publish(record) => Some(record.phase),
        }
    }

    /// The publish failure category, or `None` for staging/write
    /// failures before publish ran.
    pub fn failure_kind(&self) -> Option<PublishFailureKind> {
        match &self.inner {
            PublishErrorRepr::Staging { .. } => None,
            PublishErrorRepr::Publish(record) => Some(record.kind),
        }
    }

    /// The durability evidence achieved before the failure.
    pub fn durability(&self) -> DurabilityEvidence {
        match &self.inner {
            PublishErrorRepr::Staging { .. } => DurabilityEvidence::None,
            PublishErrorRepr::Publish(record) => record.durability,
        }
    }

    /// The temp state after the recovery policy ran — a retained
    /// `Indeterminate` leftover is reported, never hidden.
    pub fn temp_state(&self) -> TempState {
        match &self.inner {
            PublishErrorRepr::Staging { .. } => TempState::Absent,
            PublishErrorRepr::Publish(record) => record.temp_state,
        }
    }

    /// The staged temp path when the adapter tracked one (diagnostic,
    /// not an identity proof).
    pub fn temp_path(&self) -> Option<&Path> {
        match &self.inner {
            PublishErrorRepr::Staging { .. } => None,
            PublishErrorRepr::Publish(record) => record.temp_path.as_deref(),
        }
    }

    fn io_kind(&self) -> io::ErrorKind {
        match &self.inner {
            PublishErrorRepr::Staging { source } => source.kind(),
            PublishErrorRepr::Publish(record) => match record.kind {
                PublishFailureKind::DestinationExists => io::ErrorKind::AlreadyExists,
                PublishFailureKind::Unsupported => io::ErrorKind::Unsupported,
                _ => io::ErrorKind::Other,
            },
        }
    }
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            PublishErrorRepr::Staging { source } => {
                write!(f, "staged write failed before publish: {source}")
            }
            PublishErrorRepr::Publish(record) => {
                write!(
                    f,
                    "publish to {} failed ({}) — {} in {} phase; effect: {}; achieved {}; temp: {}",
                    record.destination.display(),
                    record.operation,
                    record.kind,
                    record.phase,
                    record.effect,
                    record.durability,
                    record.temp_state,
                )?;
                if let Some(path) = &record.temp_path {
                    write!(f, " at {}", path.display())?;
                }
                if let Some(detail) = &record.detail {
                    write!(f, ": {detail}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.inner {
            PublishErrorRepr::Staging { source } => Some(source),
            PublishErrorRepr::Publish(_) => None,
        }
    }
}

impl From<PublishError> for io::Error {
    fn from(error: PublishError) -> Self {
        if let PublishErrorRepr::Staging { source } = error.inner {
            return source;
        }
        let kind = error.io_kind();
        io::Error::new(kind, error)
    }
}

/// Stage a write to `path`: creates missing parent directories, then
/// opens the exclusive temp sibling the staged file writes into.
///
/// The returned [`StagedFile`] is a `Write + Seek` sink — feed it to a
/// streaming producer (e.g. `WavWriter`), `seal()` it, and finish with
/// [`publish_sealed`]. Dropping it un-sealed removes the temp, so a
/// failed write never leaves a stray sibling.
pub fn stage_file(path: &Path) -> Result<StagedFile, PublishError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(PublishError::staging)?;
    }
    StagedFile::create(path).map_err(|e| PublishError::staging(file_error_to_io(e)))
}

/// Write `bytes` to `path` and publish them under `options`.
///
/// A write-phase failure drops the staged file (its temp is removed);
/// a publish failure surfaces as [`PublishError`] with the ruviform
/// classification intact.
pub fn publish_bytes(
    path: &Path,
    bytes: &[u8],
    options: PublishOptions,
) -> Result<PublishReceipt, PublishError> {
    let mut staged = stage_file(path)?;
    staged.write_all(bytes).map_err(PublishError::staging)?;
    publish_sealed(staged.seal(), options)
}

/// Publish a sealed staged file under `options`.
///
/// `Ok(PublishReceipt)` means every requested guarantee was achieved.
/// The AudioLens temp-recovery policy runs inside the failure path:
/// a temp retained under a *classified* effect (`NotPublished`,
/// `Published`) is known-garbage and removed through the recovery
/// handle, while an `Indeterminate` leftover is kept and reported in
/// the [`PublishError`] — it may alias the destination's only copy of
/// unknown-state data, so resume diagnostics must find it instead of
/// silently deleting it.
pub fn publish_sealed(
    sealed: SealedFile,
    options: PublishOptions,
) -> Result<PublishReceipt, PublishError> {
    match sealed.publish(options) {
        Ok(mut receipt) => {
            // A leftover temp after a successful publish is known
            // garbage — drop it so repeated runs never accumulate
            // orphan siblings.
            if let Some(handle) = receipt.take_recovery_handle() {
                let _ = handle.cleanup();
            }
            Ok(receipt)
        }
        Err(failure) => Err(PublishError::from_failure(failure)),
    }
}

impl PublishError {
    /// Apply the temp-recovery policy to a classified publish failure
    /// and extract the record the job's diagnostics need.
    fn from_failure(mut failure: PublishFailure) -> Self {
        let mut temp_state = failure.info().temp_state();
        if failure.effect() != PublishEffect::Indeterminate
            && let Some(handle) = failure.take_recovery_handle()
        {
            temp_state = match handle.cleanup() {
                Ok(receipt) => receipt.temp_state(),
                Err(cleanup) => cleanup.info().temp_state(),
            };
        }
        if failure.effect() == PublishEffect::Indeterminate {
            tracing::warn!(
                operation = %failure.info().operation_id(),
                destination = %failure.info().destination().display(),
                temp_path = ?failure.info().temp_path(),
                "publish outcome indeterminate — staged temp retained for resume-time inspection"
            );
        }
        let detail = error_chain(&failure);
        Self {
            inner: PublishErrorRepr::Publish(PublishRecord {
                kind: failure.kind(),
                phase: failure.phase(),
                effect: failure.effect(),
                durability: failure.durability(),
                operation: failure.info().operation_id(),
                destination: failure.info().destination().to_path_buf(),
                temp_path: failure.info().temp_path().map(Path::to_path_buf),
                temp_state,
                detail,
            }),
        }
    }
}

/// The `PublishFailure` source chain as text — its `Display` already
/// carries the classification, so only the underlying causes are kept.
fn error_chain(error: &PublishFailure) -> Option<String> {
    let mut parts = Vec::new();
    let mut source = std::error::Error::source(error);
    while let Some(e) = source {
        parts.push(e.to_string());
        source = e.source();
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(": "))
    }
}

fn file_error_to_io(error: FileError) -> io::Error {
    let kind = match error.kind() {
        FileErrorKind::Unsupported => io::ErrorKind::Unsupported,
        FileErrorKind::InvalidInput => io::ErrorKind::InvalidInput,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_relative_paths_pass() {
        let job = Path::new("job");
        assert_eq!(
            checked_job_path(job, Path::new("artifacts/decoded.f64le")).unwrap(),
            Path::new("job/artifacts/decoded.f64le")
        );
        assert_eq!(
            checked_job_path(job, Path::new("./artifacts/x")).unwrap(),
            Path::new("job/artifacts/x")
        );
    }

    #[test]
    fn traversal_is_rejected() {
        let job = Path::new("job");
        assert!(checked_job_path(job, Path::new("../escape")).is_err());
        assert!(checked_job_path(job, Path::new("a/../../escape")).is_err());
        assert!(checked_job_path(job, Path::new("..")).is_err());
    }

    #[test]
    fn absolute_paths_are_rejected() {
        let job = Path::new("job");
        #[cfg(windows)]
        let abs = Path::new(r"C:\tmp\evil.bin");
        #[cfg(not(windows))]
        let abs = Path::new("/tmp/evil.bin");
        assert!(checked_job_path(job, abs).is_err());
        #[cfg(windows)]
        assert!(checked_job_path(job, Path::new(r"\\server\share")).is_err());
    }

    fn workdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fsutil-{tag}-{}", al_core::JobId::new()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Whether a `.rvf-tmp-*` staged sibling survives under `dir`.
    fn staged_temp_left(dir: &Path) -> bool {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.file_name().to_string_lossy().contains(".rvf-tmp-"))
    }

    const DURABLE: PublishOptions =
        PublishOptions::new(PublishMode::Replace, Durability::FileAndDirectorySync);
    const RELAXED: PublishOptions =
        PublishOptions::new(PublishMode::Replace, Durability::BestEffort);

    // The create→publish rewrite paths (manifest.json, array.json)
    // rely on Replace mode publishing over an existing destination
    // atomically.
    #[test]
    fn publish_bytes_replaces_existing_destination() {
        let dir = workdir("replace");
        let path = dir.join("manifest.json");
        publish_bytes(&path, b"first", DURABLE).unwrap();
        publish_bytes(&path, b"second", DURABLE).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!staged_temp_left(&dir));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn publish_bytes_best_effort_replaces_existing_destination() {
        let dir = workdir("relaxed");
        let path = dir.join("chunk.bin");
        publish_bytes(&path, b"first", RELAXED).unwrap();
        publish_bytes(&path, b"second", RELAXED).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        assert!(!staged_temp_left(&dir));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn create_new_reports_destination_exists_and_recovers_temp() {
        let dir = workdir("exists");
        let path = dir.join("manifest.json");
        publish_bytes(&path, b"original", DURABLE).unwrap();
        let err = publish_bytes(
            &path,
            b"new",
            PublishOptions::new(PublishMode::CreateNew, Durability::FileSync),
        )
        .unwrap_err();
        assert_eq!(
            err.failure_kind(),
            Some(PublishFailureKind::DestinationExists)
        );
        assert_eq!(err.effect(), PublishEffect::NotPublished);
        assert_eq!(err.phase(), Some(PublishPhase::Publish));
        // The classified leftover temp was cleaned by the recovery
        // policy; the destination keeps its previous content.
        assert!(!staged_temp_left(&dir));
        assert_eq!(fs::read(&path).unwrap(), b"original");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn staging_failure_reports_not_published() {
        let dir = workdir("staging");
        // A file where the parent dir must be: mkdir fails before any
        // temp exists — destination untouched, effect NotPublished.
        let blocker = dir.join("blocker");
        fs::write(&blocker, b"file").unwrap();
        let err = publish_bytes(&blocker.join("x.json"), b"data", DURABLE).unwrap_err();
        assert_eq!(err.effect(), PublishEffect::NotPublished);
        assert_eq!(err.phase(), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn streamed_write_publishes_through_seal() {
        let dir = workdir("stream");
        let path = dir.join("nested").join("out.wav");
        let mut staged = stage_file(&path).unwrap();
        staged.write_all(b"payload").unwrap();
        let receipt = publish_sealed(staged.seal(), RELAXED).unwrap();
        assert_eq!(receipt.temp_state(), TempState::Absent);
        assert_eq!(fs::read(&path).unwrap(), b"payload");
        assert!(!staged_temp_left(&dir.join("nested")));
        fs::remove_dir_all(&dir).ok();
    }
}
