//! `DatasetReader`: the manifest is the sole entry point. Artifact
//! enumeration, role lookups, and store access all go through it; a
//! missing artifact or a malformed manifest fails explicitly.

use std::io;
use std::path::{Path, PathBuf};

use al_media::wav::DecodedAudio;

use crate::audio::read_wav;
use crate::manifest::{DatasetArtifact, DatasetManifest, RoleStatus, StoreKind};
use crate::table::{Cell, TableReader};
use crate::tensor::TensorReader;

fn invalid(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// Reads a dataset under `root` via `manifest.json`.
pub struct DatasetReader {
    root: PathBuf,
    manifest: DatasetManifest,
}

impl DatasetReader {
    /// Open `root/manifest.json`. Missing or malformed manifests, and
    /// schema-version mismatches, fail explicitly (pre-release: no
    /// migration or compatibility layer — regenerate the dataset).
    pub fn open(root: &Path) -> io::Result<Self> {
        let manifest = DatasetManifest::load(root)?;
        if manifest.schema_version != crate::manifest::SCHEMA_VERSION {
            return Err(invalid(format!(
                "dataset schema {} unsupported (expected {})",
                manifest.schema_version,
                crate::manifest::SCHEMA_VERSION
            )));
        }
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
        })
    }

    /// The manifest.
    pub fn manifest(&self) -> &DatasetManifest {
        &self.manifest
    }

    /// All artifacts registered under `role`.
    pub fn artifacts(&self, role: &str) -> Vec<&DatasetArtifact> {
        self.manifest
            .artifacts
            .iter()
            .filter(|a| a.role == role)
            .collect()
    }

    /// Whether `role` produced anything.
    pub fn role_produced(&self, role: &str) -> bool {
        self.manifest
            .roles
            .get(role)
            .is_some_and(|e| e.status == RoleStatus::Produced)
    }

    fn artifact(&self, role: &str, name: &str, store: StoreKind) -> io::Result<&DatasetArtifact> {
        let rel = format!("{role}/{name}");
        self.manifest
            .artifacts
            .iter()
            .find(|a| a.role == role && a.path == rel)
            .filter(|a| a.store == store)
            .ok_or_else(|| invalid(format!("no {store:?} artifact at {rel}")))
    }

    /// Resolve a manifest-recorded artifact path against the dataset
    /// root. Manifest paths are untrusted: reject anything that could
    /// point outside the root.
    fn resolve(&self, a: &DatasetArtifact) -> io::Result<PathBuf> {
        for part in a.path.split('/') {
            if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
                return Err(invalid(format!("unsafe artifact path {:?}", a.path)));
            }
        }
        Ok(self.root.join(&a.path))
    }

    /// Open a table artifact for range/column reads.
    pub fn open_table(&self, role: &str, name: &str) -> io::Result<TableReader> {
        let a = self.artifact(role, name, StoreKind::Table)?;
        TableReader::open(&self.resolve(a)?)
    }

    /// Convenience: read `[start, end)` rows of selected columns.
    pub fn read_table(
        &self,
        role: &str,
        name: &str,
        row_start: u64,
        row_end: u64,
        columns: Option<&[usize]>,
    ) -> io::Result<Vec<Vec<Cell>>> {
        self.open_table(role, name)?
            .read_range(row_start, row_end, columns)
    }

    /// Open a tensor artifact for region reads.
    pub fn open_tensor(&self, role: &str, name: &str) -> io::Result<TensorReader> {
        let a = self.artifact(role, name, StoreKind::Tensor)?;
        TensorReader::open(&self.resolve(a)?)
    }

    /// Convenience: read a tensor region (per-dim `[start, end)`).
    pub fn read_tensor(
        &self,
        role: &str,
        name: &str,
        region: &[[u64; 2]],
    ) -> io::Result<(Vec<f64>, Vec<u64>)> {
        self.open_tensor(role, name)?.read_range(region)
    }

    /// Decode an audio artifact.
    pub fn read_audio(&self, role: &str, name: &str) -> io::Result<DecodedAudio> {
        read_wav(&self.resolve_audio(role, name)?)
    }

    /// Decode frames `[start_frame, end_frame)` of an audio artifact
    /// without loading the whole file. Out-of-range reads fail.
    pub fn read_audio_range(
        &self,
        role: &str,
        name: &str,
        start_frame: u64,
        end_frame: u64,
    ) -> io::Result<DecodedAudio> {
        let mut file = std::fs::File::open(self.resolve_audio(role, name)?)?;
        al_media::wav::read_wav_range(&mut file, start_frame, end_frame)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Frame count of an audio artifact, from the header alone.
    pub fn audio_frames(&self, role: &str, name: &str) -> io::Result<u64> {
        let mut file = std::fs::File::open(self.resolve_audio(role, name)?)?;
        al_media::wav::wav_frame_count(&mut file)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    fn resolve_audio(&self, role: &str, name: &str) -> io::Result<PathBuf> {
        let name = if name.ends_with(".wav") {
            name.to_string()
        } else {
            format!("{name}.wav")
        };
        let a = self.artifact(role, &name, StoreKind::Audio)?;
        self.resolve(a)
    }

    /// Read a JSON artifact.
    pub fn read_json(&self, role: &str, name: &str) -> io::Result<serde_json::Value> {
        let name = if name.ends_with(".json") {
            name.to_string()
        } else {
            format!("{name}.json")
        };
        let a = self.artifact(role, &name, StoreKind::Json)?;
        let bytes = std::fs::read(self.resolve(a)?)?;
        serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))
    }
}
