//! `al-desktop`: AudioLens native desktop composition root
//! (Issue #166 AL-G1)。
//!
//! この binary のみが GPUI 型を持つ。`DesktopSession` は
//! driver-neutral であり、`GpuiDriver` はここで組み立てる。
//! 使い方: `cargo run -p al-gui --features gpui --bin al-desktop`
//! (実 OS window が開く。CI では `check-ui-gpui`/`lint-ui-gpui`/
//! `test-ui-gpui` で compile/lint/headless test のみ検証)。

use std::process::ExitCode;
use std::time::Duration;

use al_gui::desktop::DesktopSession;
use al_gui::preferences::{GuiPreferences, default_path};
use al_gui::service::AlGuiService;
use al_gui::theme;
use ruviform_ui::{DataRegistry, UiRuntime, WindowDriver};

/// job drain tick 間隔(webview host と同程度)。
const TICK: Duration = Duration::from_millis(30);

fn main() -> ExitCode {
    let prefs = default_path()
        .map(|p| GuiPreferences::load_or_default(&p))
        .unwrap_or_default();

    // AL-G2: product theme を RF#385 resolver で解決。mode は
    // preferences 由来(OS 通知は未接続 → `resolved_mode(None)`)。
    let mode = prefs.resolved_mode(None);
    let theme = match theme::resolve(mode) {
        Ok(t) => Some(t),
        Err(e) => {
            eprintln!("[al-desktop] theme resolve failed ({mode}): {e:?} — fallback palette");
            None
        }
    };

    let mut runtime = UiRuntime::new(DataRegistry::new());
    let service = match &prefs.base_dir {
        Some(dir) => AlGuiService::with_base_dir(dir),
        None => AlGuiService::new(),
    };
    if let Err(e) = runtime.register_service(service) {
        eprintln!("[al-desktop] service registration failed: {e}");
        return ExitCode::from(2);
    }

    let mut session = DesktopSession::new(runtime);
    if let Some(t) = theme {
        session.set_theme(t);
    }

    let mut driver = ruviform_ui_gpui::GpuiDriver::new();
    let window = match driver.create_window(&prefs.window_spec("AudioLens")) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[al-desktop] create_window failed: {e}");
            return ExitCode::from(2);
        }
    };
    eprintln!("[al-desktop] window {window} (mode={mode})");

    match driver.run(Some(TICK), move |ev, drv| session.handle(ev, drv)) {
        Ok(()) => {
            eprintln!("[al-desktop] exited cleanly");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[al-desktop] driver exited with error: {e}");
            ExitCode::from(2)
        }
    }
}
