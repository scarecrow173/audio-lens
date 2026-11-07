//! `al-gui`: Ruviform GUI host 上の AudioLens GUI アプリケーション。
//!
//! thin presentation 層。`ruviform_ui::Service` 実装
//! ([`service::AlGuiService`])が IPC op を `al-service` の
//! Job/Dataset サービスへ橋渡しする。永続・解析ロジックは
//! ここに置かない(設計: `docs/designs/gui-app-architecture.md`)。
//!
//! デフォルト feature は headless(lib + op 層のみ)。`webview`
//! feature で `ruviform-ui-webview` の wry/tao host binary
//! `al-gui` が、`gpui` feature で `ruviform-ui-gpui` の
//! GPUI composition root binary `al-desktop` が有効になる
//! (Issue #166 AL-G1)。`desktop`/`theme`/`preferences`/`view` は
//! どちらの backend にも属さない driver-neutral 層。

pub mod assets;
pub mod desktop;
pub mod preferences;
pub mod service;
pub mod theme;
pub mod view;

pub use desktop::{DesktopPolicy, DesktopSession};
pub use service::AlGuiService;
