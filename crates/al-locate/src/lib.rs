//! Source localization for AudioLens: multi-method locate evidence,
//! consensus acceptance, loop/long-source tracking, mixer fitting,
//! and residual search.

pub mod candidate;
pub mod consensus;
pub mod decode;
pub mod methods;
pub mod mixer;
pub mod normalize;
pub mod pool;
pub mod prepared;
pub mod residual;
pub mod result;
pub mod stage;
pub mod tracking;

pub use candidate::{LocateCandidate, MethodEvidence};
pub use consensus::{ConsensusConfig, evaluate};
pub use decode::{DecodedInput, decode_input, deinterleave, format_scale};
pub use methods::{BandNcc, FullNcc, GccPhat, LocateMethod, SpectralEnvelope, default_methods};
pub use mixer::{MixerEstimate, fit_mix};
pub use residual::{
    PreparedResidual, ResidualEnergy, ResidualSearch, assemble_residual_search, prepare_residual,
    subtract_fit,
};
pub use result::{LocateResult, SampleRange};
pub use stage::{LocateConfig, LocateStage};
pub use tracking::{
    Anchor, LoopOccurrence, Segment, SegmentKind, TimeMap, build_time_map,
    identify_loop_occurrences,
};
