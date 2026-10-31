//! `al-gui`: Ruviform GUI host 上の AudioLens GUI アプリケーション。
//!
//! thin presentation 層。`ruviform_ui::Service` 実装
//! ([`service::AlGuiService`])が IPC op を `al-service` の
//! Job/Dataset サービスへ橋渡しする。永続・解析ロジックは
//! ここに置かない(設計: `docs/designs/gui-app-architecture.md`)。
//!
//! デフォルト feature は headless(lib + op 層のみ)。`webview`
//! feature で `ruviform-ui-webview` の wry/tao host binary
//! `al-gui` が有効になる。

pub mod assets;
pub mod service;

pub use service::AlGuiService;
