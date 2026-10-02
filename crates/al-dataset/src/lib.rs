//! AudioLens analysis dataset: manifest + role-separated stores
//! (columnar table, chunked tensor, WAV audio) with provenance.

pub mod manifest;
pub mod provenance;
pub mod types;

pub use manifest::{DatasetArtifact, DatasetManifest, RoleStatus, StoreKind};
pub use provenance::{InputReference, Provenance};
pub use types::{ChannelSelector, Delta, FeatureRecord, Range, Resolution, Value};
