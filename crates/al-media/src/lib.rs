//! Media ingest layer: container detection, decoding, timing observation,
//! and the versioned capture-artifact dictionary.

pub mod avi;
pub mod dictionary;
pub mod format;
pub mod metadata;
pub mod stage;
pub mod timing;
pub mod wav;

pub use al_core::sha256_hex;
pub use avi::{AviMedia, StreamTiming, VideoInfo, decode_avi};
pub use dictionary::{Annotation, DictEntry, Dictionary, EntryMatch};
pub use format::{MediaFormat, detect_format};
pub use metadata::{AudioMetadata, IntegrityObservation, MediaError, SampleFormat};
pub use stage::IngestStage;
pub use timing::{
    ClockKind, Discontinuity, IntervalStats, SequenceTimestamps, TimestampSource,
    TimingObservation, observe_source, observe_timing,
};
pub use wav::{DecodedAudio, decode_wav};
