//! `DatasetWriter`: normalizes stage output and routes each artifact
//! into its role-specific store, recording provenance and updating the
//! manifest as it goes.
//!
//! Roles follow the dataset layout (`inputs`, `locate`,
//! `intermediate`, `features/cheap`, `features/reference`,
//! `features/expensive`, `residual`, `events`, `visualization`); an
//! artifact named `x` under role `features/cheap` lands at
//! `features/cheap/x` beneath the dataset root. Declared inputs are
//! recorded by reference (path + SHA-256) and never copied.

use std::fs;
use std::io;
use std::path::PathBuf;

use al_media::wav::WavEncoding;

use crate::audio::write_wav;
use crate::manifest::{DatasetArtifact, DatasetManifest, StoreKind};
use crate::provenance::{InputReference, Provenance};
use crate::table::{Cell, Column, TableWriter};
use crate::tensor::TensorWriter;

fn bad_input(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.into())
}

/// One tensor chunk to persist: chunk coordinates plus row-major f64
/// data matching the chunk extent.
pub struct TensorChunk {
    /// Chunk coordinates (one per dimension).
    pub coords: Vec<u64>,
    /// Row-major data for the chunk extent.
    pub data: Vec<f64>,
}

/// Writes normalized stage output into a dataset root (`analysis/`).
pub struct DatasetWriter {
    root: PathBuf,
    manifest: DatasetManifest,
}

impl DatasetWriter {
    /// Create a dataset root and its manifest. Fails if `root`
    /// already exists — a dataset is regenerated wholesale, never
    /// merged into a previous run.
    pub fn create(
        root: &std::path::Path,
        inputs: Vec<InputReference>,
        config: serde_json::Value,
    ) -> io::Result<Self> {
        fs::create_dir(root).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("create dataset root {}: {e}", root.display()),
            )
        })?;
        let manifest = DatasetManifest::new(inputs, config);
        manifest.save(root)?;
        Ok(Self {
            root: root.to_path_buf(),
            manifest,
        })
    }

    /// The manifest as built so far.
    pub fn manifest(&self) -> &DatasetManifest {
        &self.manifest
    }

    /// Dataset root path.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    fn artifact_path(&self, role: &str, name: &str) -> io::Result<PathBuf> {
        for part in role.split('/').chain(name.split('/')) {
            if part.is_empty() || part == "." || part == ".." || part.contains('\\') {
                return Err(bad_input(format!("unsafe artifact path {role}/{name}")));
            }
        }
        Ok(self.root.join(role).join(name))
    }

    fn register(&mut self, role: &str, rel_path: String, store: StoreKind, prov: Provenance) {
        self.manifest.register(DatasetArtifact {
            role: role.to_string(),
            path: rel_path,
            store,
            provenance: prov,
        });
    }

    /// Persist a columnar table of `rows` under `role/name`.
    pub fn write_table(
        &mut self,
        role: &str,
        name: &str,
        schema: &[Column],
        rows: &[Vec<Cell>],
        provenance: Provenance,
    ) -> io::Result<()> {
        let path = self.artifact_path(role, name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut table =
            TableWriter::create(&path, schema.to_vec(), crate::table::DEFAULT_CHUNK_ROWS)?;
        for row in rows {
            table.push(row)?;
        }
        table.finish()?;
        self.register(role, format!("{role}/{name}"), StoreKind::Table, provenance);
        Ok(())
    }

    /// Persist a chunked tensor under `role/name`.
    pub fn write_tensor(
        &mut self,
        role: &str,
        name: &str,
        shape: &[u64],
        chunk_shape: &[u64],
        chunks: &[TensorChunk],
        provenance: Provenance,
    ) -> io::Result<()> {
        let path = self.artifact_path(role, name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut tensor = TensorWriter::create(&path, shape, chunk_shape)?;
        for chunk in chunks {
            tensor.write_chunk(&chunk.coords, &chunk.data)?;
        }
        tensor.finish()?;
        self.register(
            role,
            format!("{role}/{name}"),
            StoreKind::Tensor,
            provenance,
        );
        Ok(())
    }

    /// Persist audio under `role/name.wav`.
    #[allow(clippy::too_many_arguments)]
    pub fn write_audio(
        &mut self,
        role: &str,
        name: &str,
        encoding: WavEncoding,
        channels: u16,
        sample_rate: u32,
        samples: &[f64],
        provenance: Provenance,
    ) -> io::Result<()> {
        let name = if name.ends_with(".wav") {
            name.to_string()
        } else {
            format!("{name}.wav")
        };
        let path = self.artifact_path(role, &name)?;
        write_wav(&path, encoding, channels, sample_rate, samples)?;
        self.register(role, format!("{role}/{name}"), StoreKind::Audio, provenance);
        Ok(())
    }

    /// Persist a JSON document under `role/name.json`.
    pub fn write_json(
        &mut self,
        role: &str,
        name: &str,
        value: &serde_json::Value,
        provenance: Provenance,
    ) -> io::Result<()> {
        let name = if name.ends_with(".json") {
            name.to_string()
        } else {
            format!("{name}.json")
        };
        let path = self.artifact_path(role, &name)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        al_job::fsutil::write_durable(&path, &bytes)?;
        self.register(role, format!("{role}/{name}"), StoreKind::Json, provenance);
        Ok(())
    }

    /// Write the final manifest. After this the dataset is complete.
    pub fn finish(self) -> io::Result<DatasetManifest> {
        self.manifest.save(&self.root)?;
        Ok(self.manifest)
    }
}
