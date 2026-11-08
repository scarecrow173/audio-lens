//! `al-gui` — ruviform-ui-webview host 上の AudioLens GUI。
//! `webview` feature 有効時のみビルドされる(`required-features`)。
//!
//! 使い方:
//!   `cargo run -p al-gui --features webview -- [base_dir]`

use std::process::ExitCode;
use std::time::Duration;

use al_gui::assets::app_assets;
use al_gui::{AlGuiService, DesktopSession};
use ruviform_ui::{DataRegistry, UiRuntime, WindowSpec};
use ruviform_ui_webview::{
    SessionConfig, WebViewWindowConfig, detect_backend, run_multi, webview_available,
};

const APP_URL: &str = "rvf://app/index.html";

/// job drain tick 間隔(旧 `HostConfig::tick` 既定値と同じ)。
const TICK: Duration = Duration::from_millis(16);

fn main() -> ExitCode {
    let base_dir = std::env::args().nth(1);
    let report = detect_backend();
    eprintln!(
        "backend: {} state={:?} version={:?}",
        report.name, report.state, report.version
    );
    for note in &report.notes {
        eprintln!("  note: {note}");
    }
    if !webview_available() {
        eprintln!("webview backend unavailable — install the runtime and retry");
        return ExitCode::from(2);
    }

    let registry = DataRegistry::new();
    let mut rt = UiRuntime::new(registry.clone());
    let service = match &base_dir {
        Some(dir) => AlGuiService::with_base_dir(dir),
        None => AlGuiService::new(),
    };
    rt.register_service(service).expect("al-gui service");

    // driver-neutral session をそのまま webview driver に接続する。
    // close veto / job 終端待ち Defer / last-window exit の policy は
    // al-desktop (GPUI) と同一実装を共有する。
    let mut session = DesktopSession::new(rt);

    let config = SessionConfig {
        data_dir: None,
        tick: Some(TICK),
        windows: vec![WebViewWindowConfig {
            spec: WindowSpec {
                title: "AudioLens".to_string(),
                ..WindowSpec::default()
            },
            url: APP_URL.to_string(),
            devtools: cfg!(debug_assertions),
        }],
        default_url: APP_URL.to_string(),
        devtools: cfg!(debug_assertions),
    };

    match run_multi(&config, app_assets(), registry, |event, driver| {
        session.handle(event, driver);
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("host exited with error: {e}");
            ExitCode::from(2)
        }
    }
}
