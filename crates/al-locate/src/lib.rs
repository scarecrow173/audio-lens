//! Source localization for AudioLens: multi-method locate evidence,
//! consensus acceptance, loop/long-source tracking, mixer fitting,
//! and residual search.

pub mod candidate;
pub mod consensus;
pub mod decode;
pub mod dsp;
pub mod methods;
pub mod mixer;
pub mod normalize;
pub mod prepared;
pub mod residual;
pub mod result;
pub mod stage;
pub mod tracking;

pub use al_math::linalg::{condition_number, least_squares};
pub use candidate::{LocateCandidate, MethodEvidence};
pub use consensus::{ConsensusConfig, evaluate};
pub use decode::{DecodedInput, decode_input, format_scale};
pub use dsp::{bandpass, deinterleave, fft, gcc_phat, ncc, spectral_envelope};
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
