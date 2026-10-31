//! `al-service`: AudioLens の UI 非依存サービス層。
//!
//! `al-cli` と `al-gui` が共有する解析入口。Job 操作
//! (run/resume/inspect/list/cancel)と永続 Analysis Dataset への
//! range query port を提供する。GUI・WebView・ウィンドウ系への
//! 依存を持たない(チェック: `mise run check-cli-headless`)。
//!
//! 設計: `docs/designs/gui-app-architecture.md`。

pub mod dataset;
pub mod error;
pub mod jobs;
pub mod pipeline;
pub mod source;

pub use dataset::{DatasetSession, FeatureQuery};
pub use error::ServiceError;
pub use jobs::{JobInspection, JobService, JobSummary, RunControl, RunSpec};
pub use pipeline::{
    default_stage_kinds, make_stage, parse_input_spec, parse_stage, resolve_config,
};
pub use source::{DatasetSpectrum, DatasetWaveform};
