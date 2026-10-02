//! Source localization for AudioLens: multi-method locate evidence,
//! consensus acceptance, loop/long-source tracking, mixer fitting,
//! and residual search.

pub mod dsp;

pub use dsp::{
    bandpass, condition_number, deinterleave, fft, gcc_phat, least_squares, ncc, spectral_envelope,
};
