//! AudioLens job orchestration (job model, manifest, registry, runner).

pub mod job;
pub mod manifest;
pub mod registry;

pub use job::{JobDefinition, JobInput, input};
pub use manifest::{ArtifactRecord, Manifest, StageRecord};
pub use registry::ArtifactRegistry;
