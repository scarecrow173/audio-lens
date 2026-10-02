//! AudioLens analysis dataset: manifest + role-separated stores
//! (columnar table, chunked tensor, WAV audio) with provenance.

pub mod audio;
pub mod manifest;
pub mod provenance;
pub mod reader;
pub mod stage;
pub mod table;
pub mod tensor;
pub mod types;
pub mod writer;

pub use audio::{read_wav, write_wav};
pub use manifest::{DatasetArtifact, DatasetManifest, RoleStatus, StoreKind};
pub use provenance::{InputReference, Provenance};
pub use reader::DatasetReader;
pub use stage::DatasetStage;
pub use table::{Cell, Column, DataType, TableReader, TableWriter};
pub use tensor::{ArrayMeta, TensorReader, TensorWriter};
pub use types::{ChannelSelector, Delta, FeatureRecord, Range, Resolution, Value};
pub use writer::{DatasetWriter, TensorChunk};
