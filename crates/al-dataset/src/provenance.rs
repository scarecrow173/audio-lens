//! Provenance records: every stored artifact can be traced back to the
//! inputs, algorithm, parameters, alignment, and parents that made it.

use serde::{Deserialize, Serialize};

/// Provenance of one dataset artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// Id of the input the artifact derives from.
    pub input_id: String,
    /// Algorithm id that produced the artifact.
    pub algorithm_id: String,
    /// Algorithm version (independent from the dataset schema version).
    pub algorithm_version: String,
    /// Algorithm parameters.
    pub params: serde_json::Value,
    /// Alignment id underpinning the artifact, when applicable.
    pub alignment_id: Option<String>,
    /// Parent / intermediate artifact ids in the derivation chain.
    pub parents: Vec<String>,
}

/// A declared input recorded by reference — never copied into the
/// dataset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputReference {
    /// Input id from the job manifest.
    pub input_id: String,
    /// Role the input was declared with.
    pub role: String,
    /// Absolute path to the source file at job-creation time.
    pub source_path: String,
    /// SHA-256 of the source bytes pinned at job creation.
    pub source_sha256: String,
}
