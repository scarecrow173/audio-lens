//! Source localization for AudioLens: multi-method locate evidence,
//! consensus acceptance, loop/long-source tracking, mixer fitting,
//! and residual search.

pub mod candidate;
pub mod consensus;
pub mod dsp;
pub mod methods;
pub mod result;

pub use candidate::{LocateCandidate, MethodEvidence};
pub use consensus::{ConsensusConfig, evaluate};
pub use dsp::{
    bandpass, condition_number, deinterleave, fft, gcc_phat, least_squares, ncc, spectral_envelope,
};
pub use methods::{BandNcc, FullNcc, GccPhat, LocateMethod, SpectralEnvelope, default_methods};
pub use result::{LocateResult, SampleRange};
