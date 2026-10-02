//! Source localization for AudioLens: multi-method locate evidence,
//! consensus acceptance, loop/long-source tracking, mixer fitting,
//! and residual search.

pub mod candidate;
pub mod consensus;
pub mod dsp;
pub mod methods;
pub mod mixer;
pub mod residual;
pub mod result;
pub mod stage;
pub mod tracking;

pub use candidate::{LocateCandidate, MethodEvidence};
pub use consensus::{ConsensusConfig, evaluate};
pub use dsp::{
    bandpass, condition_number, deinterleave, fft, gcc_phat, least_squares, ncc, spectral_envelope,
};
pub use methods::{BandNcc, FullNcc, GccPhat, LocateMethod, SpectralEnvelope, default_methods};
pub use mixer::{MixerEstimate, fit_mix};
pub use residual::{ResidualEnergy, ResidualSearch, residual_search, subtract_fit};
pub use result::{LocateResult, SampleRange};
pub use stage::{LocateConfig, LocateStage};
pub use tracking::{
    Anchor, LoopOccurrence, Segment, SegmentKind, TimeMap, build_time_map,
    identify_loop_occurrences,
};
