//! `al-viz`: derived visualization products and the exportable package.
//!
//! Everything here reads the saved Analysis Dataset and re-emits
//! display-shaped data — no media decode, no feature extraction, no
//! new diagnostic vocabulary (derived-visualization-dataset ADR).

pub mod golden;
pub mod lod;
pub mod package;
pub mod report;
pub mod stage;
pub mod summary;
pub mod tiles;
pub mod timeline;
pub mod views;

use al_dataset::writer::TensorChunk;

/// A derived tensor product under `visualization/`. Paths are
/// dataset-relative (`visualization/<...>`); the stage splits role
/// and name for the writer.
pub struct TensorProduct {
    /// Dataset-relative path.
    pub path: String,
    /// Tensor dimensions.
    pub shape: Vec<u64>,
    /// Row-major data.
    pub data: Vec<f64>,
    /// Dataset-relative paths of the source artifacts this product
    /// derives from (recorded as provenance parents).
    pub parents: Vec<String>,
}

/// A derived JSON product under `visualization/`.
pub struct JsonProduct {
    /// Dataset-relative path.
    pub path: String,
    /// Document body.
    pub body: serde_json::Value,
    /// Dataset-relative source artifact paths recorded as provenance
    /// parents.
    pub parents: Vec<String>,
}

/// A tensor product emitted chunk-by-chunk: conversion streams the
/// source via ranged reads, so peak memory stays one tile rather
/// than the whole array.
pub struct StreamedTensor {
    /// Dataset-relative path.
    pub path: String,
    /// Tensor dimensions.
    pub shape: Vec<u64>,
    /// Chunk grid granularity.
    pub chunk_shape: Vec<u64>,
    /// Dataset-relative source artifact paths recorded as provenance
    /// parents.
    pub parents: Vec<String>,
    /// Chunks in row-major grid order, produced on demand.
    pub chunks: Box<dyn Iterator<Item = std::io::Result<TensorChunk>> + Send>,
}
