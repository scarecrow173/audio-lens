//! `al-gui` — ruviform-ui-webview host 上の AudioLens GUI。
//! `webview` feature 有効時のみビルドされる(`required-features`)。
//!
//! 使い方:
//!   `cargo run -p al-gui --features webview -- [base_dir]`

use std::process::ExitCode;

use al_gui::{AlGuiService, assets::app_assets};
use ruviform_ui::{DataRegistry, UiRuntime, dispatch_script};
use ruviform_ui_webview::{HostConfig, detect_backend, run, webview_available};

fn main() -> ExitCode {
    let base_dir = std::env::args().nth(1);
    let report = detect_backend();
    eprintln!(
        "backend: {} state={:?} version={:?}",
        report.name, report.state, report.version
    );
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

    let config = HostConfig {
        title: "AudioLens".to_string(),
        ..HostConfig::default()
    };
    match run(&config, app_assets(), registry, |event, host| {
        for out in rt.handle(event) {
            host.eval(&dispatch_script(&out));
        }
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("host exited with error: {e}");
            ExitCode::from(2)
        }
    }
}
