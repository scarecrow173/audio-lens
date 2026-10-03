//! `al-viz`: derived visualization products and the exportable package.
//!
//! Everything here reads the saved Analysis Dataset and re-emits
//! display-shaped data — no media decode, no feature extraction, no
//! new diagnostic vocabulary (derived-visualization-dataset ADR).

pub mod golden;
pub mod lod;
pub mod package;
pub mod stage;
pub mod summary;
pub mod tiles;
pub mod timeline;
pub mod views;

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
}

/// A derived JSON product under `visualization/`.
pub struct JsonProduct {
    /// Dataset-relative path.
    pub path: String,
    /// Document body.
    pub body: serde_json::Value,
}
