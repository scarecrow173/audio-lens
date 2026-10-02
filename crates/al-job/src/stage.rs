use std::fmt;

use al_core::StageKind;

use crate::manifest::{ArtifactRecord, Manifest};
use crate::registry::ArtifactRegistry;

/// Read-only context handed to a stage during execution.
///
/// Stages read prior artifacts through the manifest and never mutate it
/// directly; produced artifacts are returned to the runner for registration.
pub struct StageContext<'a> {
    /// Job manifest (read-only view of prior artifacts and records).
    pub manifest: &'a Manifest,
    /// Artifact id issuer and path resolver for the job directory.
    pub registry: &'a ArtifactRegistry,
    /// Job configuration snapshot.
    pub config: &'a serde_json::Value,
}

/// A pipeline stage.
///
/// Implementations must communicate only through artifact ids recorded in
/// the manifest; hidden shared state between stages is not allowed.
pub trait Stage {
    /// The stage kind this implementation covers.
    fn kind(&self) -> StageKind;

    /// Stage kinds that must be completed before this stage runs.
    ///
    /// Defaults to all stages that precede this kind in the canonical
    /// pipeline order.
    fn depends_on(&self) -> Vec<StageKind> {
        self.kind().dependencies()
    }

    /// Execute the stage and return produced artifact records.
    fn run(&mut self, ctx: &StageContext<'_>) -> Result<Vec<ArtifactRecord>, StageError>;
}

/// Error raised by a stage execution.
#[derive(Debug)]
pub struct StageError {
    /// Stage that failed.
    pub stage: StageKind,
    /// Human-readable failure description.
    pub message: String,
}

impl StageError {
    /// Build an error for `stage` from any displayable message.
    pub fn new(stage: StageKind, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "stage {:?} failed: {}", self.stage, self.message)
    }
}

impl std::error::Error for StageError {}
