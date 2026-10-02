//! Media ingest layer: container detection, decoding, timing observation,
//! and the versioned capture-artifact dictionary.

pub mod format;
pub mod metadata;

pub use format::{MediaFormat, detect_format};
pub use metadata::MediaError;
