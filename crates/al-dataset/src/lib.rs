//! AudioLens analysis dataset: manifest + role-separated stores
//! (columnar table, chunked tensor, WAV audio) with provenance.

pub mod manifest;
pub mod provenance;
pub mod table;
pub mod tensor;
pub mod types;

pub use manifest::{DatasetArtifact, DatasetManifest, RoleStatus, StoreKind};
pub use provenance::{InputReference, Provenance};
pub use table::{Cell, Column, DataType, TableReader, TableWriter};
pub use tensor::{ArrayMeta, TensorReader, TensorWriter};
pub use types::{ChannelSelector, Delta, FeatureRecord, Range, Resolution, Value};
