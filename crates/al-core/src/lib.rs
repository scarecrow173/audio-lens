//! AudioLens shared core types.

pub mod b64;
pub mod hash;
pub mod ids;
pub mod types;

pub use hash::sha256_hex;
pub use ids::{AlignmentId, ArtifactId, IdError, InputId, JobId};
pub use types::{Confidence, InputRole, StageKind, StageStatus, Validity};

/// The resolved Ruviform revision this build is pinned to.
///
/// Derived at build time by `build.rs` from the `#<sha>` fragment of
/// the `ruviform` git sources in the workspace `Cargo.lock`, so
/// recorded provenance can never drift from the resolved dependency
/// (AL#115). All ruviform entries share one revision; the build fails
/// if the lockfile disagrees.
pub const RUVIFORM_REV: &str = env!("RUVIFORM_REV");
