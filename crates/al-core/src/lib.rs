//! AudioLens shared core types.

pub mod ids;
pub mod types;

pub use ids::{ArtifactId, IdError, InputId, JobId};
pub use types::{Confidence, InputRole, StageKind, StageStatus, Validity};
