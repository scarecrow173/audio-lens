//! AudioLens job orchestration (job model, manifest, registry, runner).

pub mod fsutil;
pub mod job;
pub mod manifest;
pub mod registry;
pub mod runner;
pub mod stage;

pub use job::{JobDefinition, JobInput, input};
pub use manifest::{ArtifactRecord, Manifest, StageRecord};
pub use registry::ArtifactRegistry;
pub use runner::StageRunner;
pub use stage::{Stage, StageContext, StageError};
