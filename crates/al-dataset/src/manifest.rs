//! The dataset manifest: the single entry point that aggregates schema
//! version, inputs, configuration, per-role artifacts, and provenance.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use al_job::fsutil::{Durability, PublishMode, PublishOptions, publish_bytes};

use crate::provenance::{InputReference, Provenance};

/// Filename of the dataset manifest inside `analysis/`.
pub const MANIFEST_FILE: &str = "manifest.json";

/// Current dataset schema version. Independent from any algorithm
/// version: a schema bump means the on-disk layout changed and the
/// dataset must be regenerated (pre-release: no compat layer).
pub const SCHEMA_VERSION: u32 = 1;

/// Whether a role actually produced artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoleStatus {
    /// The role produced artifacts.
    Produced,
    /// The role is defined but nothing was produced — either because
    /// the producing stage is not implemented yet or produced nothing.
    /// Always distinguishable from `Produced` with zero meaning.
    NotProduced,
}

/// Per-role summary in the manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoleEntry {
    /// Production status — "produced nothing" vs "not implemented".
    pub status: RoleStatus,
    /// Number of artifacts under this role.
    pub artifact_count: u64,
}

/// Kind of store an artifact lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreKind {
    /// Chunked columnar table directory.
    Table,
    /// Chunked tensor array directory.
    Tensor,
    /// WAV audio file.
    Audio,
    /// JSON document.
    Json,
}

/// One materialized artifact: a store root (table dir, tensor array
/// dir, audio file, or JSON doc) plus its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetArtifact {
    /// Role this artifact belongs to (`inputs`, `locate`, `features`,
    /// `residual`, `events`, `intermediate`).
    pub role: String,
    /// Dataset-relative path of the store root.
    pub path: String,
    /// Which store backs the artifact.
    pub store: StoreKind,
    /// Provenance chain for the artifact.
    pub provenance: Provenance,
}

/// The dataset manifest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetManifest {
    /// On-disk schema version (independent from algorithm versions).
    pub schema_version: u32,
    /// Audio codec used under audio roles (`wav` today).
    pub audio_format: String,
    /// Declared job inputs recorded by reference.
    pub inputs: Vec<InputReference>,
    /// Job configuration snapshot.
    pub config: serde_json::Value,
    /// Per-role status and artifact counts.
    pub roles: BTreeMap<String, RoleEntry>,
    /// Materialized artifacts in role order.
    pub artifacts: Vec<DatasetArtifact>,
}

impl DatasetManifest {
    /// A manifest with no artifacts yet; all roles `not_produced`.
    pub fn new(inputs: Vec<InputReference>, config: serde_json::Value) -> Self {
        let mut roles = BTreeMap::new();
        for role in [
            "inputs",
            "locate",
            "intermediate",
            "features/cheap",
            "features/reference",
            "features/expensive",
            "residual",
            "events",
        ] {
            roles.insert(
                role.to_string(),
                RoleEntry {
                    status: RoleStatus::NotProduced,
                    artifact_count: 0,
                },
            );
        }
        Self {
            schema_version: SCHEMA_VERSION,
            audio_format: "wav".into(),
            inputs,
            config,
            roles,
            artifacts: Vec::new(),
        }
    }

    /// Register an artifact and update its role status. An existing
    /// entry with the same `(role, path)` is replaced — rewriting a
    /// store is idempotent and must not inflate the artifact count.
    pub fn register(&mut self, artifact: DatasetArtifact) {
        if let Some(existing) = self
            .artifacts
            .iter_mut()
            .find(|a| a.role == artifact.role && a.path == artifact.path)
        {
            *existing = artifact;
            return;
        }
        let entry = self
            .roles
            .entry(artifact.role.clone())
            .or_insert(RoleEntry {
                status: RoleStatus::Produced,
                artifact_count: 0,
            });
        entry.status = RoleStatus::Produced;
        entry.artifact_count += 1;
        self.artifacts.push(artifact);
    }

    /// Load a manifest from `dataset_root/manifest.json`.
    pub fn load(dataset_root: &Path) -> std::io::Result<Self> {
        let path = dataset_root.join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&path)?;
        serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save the manifest durably under `dataset_root`.
    pub fn save(&self, dataset_root: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        publish_bytes(
            &dataset_root.join(MANIFEST_FILE),
            json.as_bytes(),
            PublishOptions::new(PublishMode::Replace, Durability::FileAndDirectorySync),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Provenance;

    fn prov() -> Provenance {
        Provenance {
            input_id: "IN-x".into(),
            algorithm_id: "test.v1".into(),
            algorithm_version: "1".into(),
            params: serde_json::json!({}),
            alignment_id: None,
            parents: vec![],
        }
    }

    #[test]
    fn register_updates_role_status() {
        let mut m = DatasetManifest::new(vec![], serde_json::json!({}));
        assert_eq!(m.roles["features/cheap"].status, RoleStatus::NotProduced);
        m.register(DatasetArtifact {
            role: "features/cheap".into(),
            path: "features/cheap/rms".into(),
            store: StoreKind::Table,
            provenance: prov(),
        });
        assert_eq!(m.roles["features/cheap"].status, RoleStatus::Produced);
        assert_eq!(m.roles["features/cheap"].artifact_count, 1);
    }

    #[test]
    fn manifest_round_trips() {
        let mut m = DatasetManifest::new(
            vec![InputReference {
                input_id: "IN-1".into(),
                role: "capture_bug".into(),
                source_path: "/x/cap.wav".into(),
                source_sha256: "ab".repeat(32),
            }],
            serde_json::json!({"a": 1}),
        );
        m.register(DatasetArtifact {
            role: "inputs".into(),
            path: "inputs/cap.wav".into(),
            store: StoreKind::Audio,
            provenance: prov(),
        });
        let dir = std::env::temp_dir().join(format!("ds-manifest-{}", al_core::JobId::new()));
        std::fs::create_dir_all(&dir).unwrap();
        m.save(&dir).unwrap();
        let loaded = DatasetManifest::load(&dir).unwrap();
        assert_eq!(loaded, m);
        std::fs::remove_dir_all(&dir).ok();
    }
}
