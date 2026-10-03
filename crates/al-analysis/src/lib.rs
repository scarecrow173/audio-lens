//! Analysis pipeline: cheap / reference / expensive feature stages
//! writing `FeatureRecord`s and spectral intermediates into the
//! Analysis Dataset.
//!
//! All DSP is implemented in-tree — spectral primitives reuse
//! `al_locate::dsp`, decoded input comes from `al_locate::decode`.

pub mod cheap;
pub mod events;
pub mod expensive;
pub mod records;
pub mod reference;
pub mod session;
pub mod stage;
pub mod stft;
