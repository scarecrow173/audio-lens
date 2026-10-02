//! Media ingest layer: container detection, decoding, timing observation,
//! and the versioned capture-artifact dictionary.

pub mod dictionary;
pub mod format;
pub mod hash;
pub mod metadata;
pub mod timing;
pub mod wav;

pub use dictionary::{Annotation, DictEntry, Dictionary, EntryMatch};
pub use format::{MediaFormat, detect_format};
pub use hash::sha256_hex;
pub use metadata::{AudioMetadata, IntegrityObservation, MediaError, SampleFormat};
pub use timing::{ClockKind, Discontinuity, IntervalStats, TimingObservation, observe_timing};
pub use wav::{DecodedAudio, decode_wav};
