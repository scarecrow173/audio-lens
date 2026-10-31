//! Analysis pipeline: cheap / reference / expensive feature stages
//! writing `FeatureRecord`s and spectral intermediates into the
//! Analysis Dataset.
//!
//! All DSP calls `ruviform_numeric` kernels directly; decoded input
//! comes from `al_locate::decode`.

pub mod avsync;
mod band_gain;
mod calibration;
pub mod cheap;
pub mod events;
pub mod expensive;
mod pool;
pub mod records;
pub mod reference;
pub mod session;
pub mod stage;
pub mod stft;

pub mod video;
