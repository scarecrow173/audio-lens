//! `al-desktop`: AudioLens native desktop composition root
//! (Issue #166 AL-G1/AL-G3)。
//!
//! この binary のみが GPUI 型を持つ。`DesktopSession` は
//! driver-neutral であり、`GpuiDriver` はここで組み立てる。
//! 使い方: `cargo run -p al-gui --features gpui --bin al-desktop -- [<dataset-root> <input-id>]`
//! 引数を省略すると Source を載せない空の画面を mount する(実 OS window が開く。
//! CI では `check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` で compile/lint/headless
//! test のみ検証)。

use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use al_gui::desktop::DesktopSession;
use al_gui::preferences::{GuiPreferences, default_path};
use al_gui::service::AlGuiService;
use al_gui::theme;
use al_gui::view::screen::PANEL_KEY;
use al_gui::view::workspace::workspace_screen;
use al_service::dataset::DatasetSession;
use ruviform_ui::dock::ViewInstanceId;
use ruviform_ui::view::{
    A11ySpec, AttachEpochAllocator, ComponentKey, ComponentSpec, ComponentVisualState, PanelSpec,
    ViewHost, ViewSpec,
};
use ruviform_ui::{DataRegistry, UiRuntime};
use ruviform_ui_gpui::ViewMount;

/// job drain tick 間隔(webview host と同程度)。
const TICK: Duration = Duration::from_millis(30);

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let source = match args.as_slice() {
        [] => None,
        [root, input] => Some((root.as_str(), input.as_str())),
        _ => {
            eprintln!("usage: al-desktop [<dataset-root> <input-id>]");
            return ExitCode::from(2);
        }
    };

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

    let screen = match source {
        None => empty_screen(),
        Some((root, input)) => match open_workspace(Path::new(root), input) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[al-desktop] source open failed: {e}");
                return ExitCode::from(2);
            }
        },
    };
    let host = match mount_host(screen) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("[al-desktop] screen mount failed: {e}");
            return ExitCode::from(2);
        }
    };

    let mut session = DesktopSession::new(runtime);
    if let Some(t) = theme {
        session.set_theme(t);
    }

    let mount = ViewMount {
        host,
        theme: session.theme().cloned(),
    };
    let mut driver = ruviform_ui_gpui::GpuiDriver::new();
    let window = match driver.create_view_window(&prefs.window_spec("AudioLens"), mount) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[al-desktop] create_view_window failed: {e}");
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

/// `root` の dataset を Source・Spectrogram・Analysis の tab 画面へ写す。
fn open_workspace(root: &Path, input: &str) -> Result<ViewSpec, String> {
    let session = DatasetSession::open(root).map_err(|e| e.to_string())?;
    workspace_screen(&session, input).map_err(|e| e.to_string())
}

/// Source を載せない空の画面(dataset 未指定時)。
fn empty_screen() -> ViewSpec {
    ViewSpec {
        root: ComponentSpec::Panel(PanelSpec {
            key: ComponentKey::from(PANEL_KEY),
            title: None,
            children: Vec::new(),
            visual: ComponentVisualState::default(),
            a11y: A11ySpec::default(),
        }),
    }
}

fn mount_host(screen: ViewSpec) -> Result<ViewHost, String> {
    let epoch = AttachEpochAllocator::new().mint();
    let mut host = ViewHost::new(ViewInstanceId::new(1).expect("nonzero"), epoch);
    host.mount(screen).map_err(|e| format!("{e:?}"))?;
    host.drain_events();
    Ok(host)
}
