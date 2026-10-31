//! `DatasetWriter`: normalizes stage output and routes each artifact
//! into its role-specific store, recording provenance and updating the
//! manifest as it goes.
//!
//! Roles follow the dataset layout (`inputs`, `locate`,
//! `intermediate`, `features/cheap`, `features/reference`,
//! `features/expensive`, `residual`, `events`); an
//! artifact named `x` under role `features/cheap` lands at
//! `features/cheap/x` beneath the dataset root. Declared inputs are
//! recorded by reference (path + SHA-256) and never copied.

use std::fs;
use std::io;
use std::path::PathBuf;

use al_media::wav::WavEncoding;

use crate::audio::write_wav;
use crate::manifest::{DatasetArtifact, DatasetManifest, MANIFEST_FILE, SCHEMA_VERSION, StoreKind};
use crate::provenance::{InputReference, Provenance};
use crate::table::{Cell, Column, TableWriter};
use crate::tensor::TensorWriter;

fn bad_input(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, msg.into())
}

fn invalid_data(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

/// One tensor chunk to persist: chunk coordinates plus row-major f64
/// data matching the chunk extent.
pub struct TensorChunk {
    /// Chunk coordinates (one per dimension).
    pub coords: Vec<u64>,
    /// Row-major data for the chunk extent.
    pub data: Vec<f64>,
}

/// Incremental tensor sink returned by
/// [`DatasetWriter::create_tensor`]. Chunks land on disk as they are
/// pushed; nothing is registered until [`TensorSink::finish`].
pub struct TensorSink<'w> {
    writer: &'w mut DatasetWriter,
    inner: TensorWriter,
    role: String,
    name: String,
}

impl TensorSink<'_> {
    /// Persist one whole chunk at `coords`. `data` length must equal
    /// the chunk extent (smaller at array edges).
    pub fn push(&mut self, coords: &[u64], data: &[f64]) -> io::Result<()> {
        self.inner.write_chunk(coords, data)
    }

    /// Commit the array and register it under the role.
    pub fn finish(self, provenance: Provenance) -> io::Result<()> {
        self.inner.finish()?;
        self.writer.register(
            &self.role,
            format!("{}/{}", self.role, self.name),
            StoreKind::Tensor,
            provenance,
        );
        Ok(())
    }
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

    /// Open an existing dataset (after validating manifest identity)
    /// or create a fresh one when no manifest exists.
    ///
    /// An existing manifest must match `inputs`, `config`, and the
    /// current [`SCHEMA_VERSION`] exactly — a mismatch or an
    /// unparseable manifest fails explicitly (pre-release rule: no
    /// compat layer, no migration, no silent recreation). A missing
    /// manifest under a non-empty root leaves orphan stores on disk;
    /// they are lazily replaced by the write-replacement rules, so a
    /// clean create is still correct.
    pub fn open_or_create(
        root: &std::path::Path,
        inputs: Vec<InputReference>,
        config: serde_json::Value,
    ) -> io::Result<Self> {
        if root.join(MANIFEST_FILE).exists() {
            let manifest = DatasetManifest::load(root)?;
            if manifest.schema_version != SCHEMA_VERSION {
                return Err(invalid_data(format!(
                    "dataset schema version {} != {} — regenerate the dataset",
                    manifest.schema_version, SCHEMA_VERSION
                )));
            }
            if manifest.inputs != inputs {
                return Err(invalid_data(
                    "dataset manifest inputs differ from the job manifest",
                ));
            }
            if manifest.config != config {
                return Err(invalid_data(
                    "dataset manifest config differs from the job config",
                ));
            }
            Ok(Self {
                root: root.to_path_buf(),
                manifest,
            })
        } else {
            fs::create_dir_all(root)?;
            let manifest = DatasetManifest::new(inputs, config);
            manifest.save(root)?;
            Ok(Self {
                root: root.to_path_buf(),
                manifest,
            })
        }
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

    /// Remove an existing store at `path` (registered or orphan) so a
    /// rewrite fully replaces the physical store — retries never merge
    /// into partial leftovers.
    fn reset_store(path: &PathBuf) -> io::Result<()> {
        if path.is_dir() {
            fs::remove_dir_all(path)?;
        } else if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
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
        Self::reset_store(&path)?;
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
        let mut sink = self.create_tensor(role, name, shape, chunk_shape)?;
        for chunk in chunks {
            sink.push(&chunk.coords, &chunk.data)?;
        }
        sink.finish(provenance)
    }

    /// Begin an incremental tensor write under `role/name`: chunks
    /// are persisted as they are pushed, so a producer never needs
    /// the whole array in memory. [`TensorSink::finish`] commits the
    /// array and registers the artifact.
    pub fn create_tensor(
        &mut self,
        role: &str,
        name: &str,
        shape: &[u64],
        chunk_shape: &[u64],
    ) -> io::Result<TensorSink<'_>> {
        let path = self.artifact_path(role, name)?;
        Self::reset_store(&path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let inner = TensorWriter::create(&path, shape, chunk_shape)?;
        Ok(TensorSink {
            writer: self,
            inner,
            role: role.to_string(),
            name: name.to_string(),
        })
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
        Self::reset_store(&path)?;
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
        Self::reset_store(&path)?;
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
