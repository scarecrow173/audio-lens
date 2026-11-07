//! AL-G2: product theme/preferences の検証。

use al_gui::preferences::{GuiPreferences, ThemeMode, WindowPrefs};
use al_gui::theme::{self, FALLBACK_MODE, MODES};
use ruviform_ui_theme::ResolvedValue;

#[test]
fn resolves_light_and_dark() {
    for mode in MODES {
        let theme = theme::resolve(mode).unwrap_or_else(|e| panic!("resolve {mode}: {e}"));
        assert_eq!(theme.mode(), *mode);
        assert!(theme.color("color.bg").is_some(), "{mode} lacks color.bg");
        assert!(theme.color("color.fg").is_some(), "{mode} lacks color.fg");
        assert!(theme.color("color.accent").is_some(), "{mode} lacks accent");
    }
}

#[test]
fn modes_differ_in_surface_colors() {
    let light = theme::resolve("light").unwrap();
    let dark = theme::resolve("dark").unwrap();
    assert_ne!(
        light.color("color.bg"),
        dark.color("color.bg"),
        "light/dark bg must differ"
    );
    assert_ne!(
        light.color("color.fg"),
        dark.color("color.fg"),
        "light/dark fg must differ"
    );
}

#[test]
fn product_layer_aliases_resolve_to_palette() {
    let theme = theme::resolve("dark").unwrap();
    // product `color.accent` は `{color.brand}` alias 経由で
    // primitive palette 値へ解決される。
    let accent = theme.color("color.accent").expect("accent");
    let brand = theme.color("color.brand").expect("primitive brand");
    assert_eq!(accent, brand);
    // waveform 製品 token も palette へ解決される。
    assert_eq!(theme.color("color.waveform"), Some(brand));
}

#[test]
fn invalid_mode_is_rejected() {
    assert!(theme::resolve("auto").is_err());
}

#[test]
fn preferences_serde_round_trip() {
    let prefs = GuiPreferences {
        theme_mode: ThemeMode::Dark,
        window: WindowPrefs {
            width: 1600,
            height: 900,
            resizable: false,
        },
        base_dir: Some("C:/work/jobs".into()),
    };
    let text = serde_json::to_string(&prefs).unwrap();
    let back: GuiPreferences = serde_json::from_str(&text).unwrap();
    assert_eq!(prefs, back);
}

#[test]
fn preferences_load_or_default_on_corruption() {
    let dir =
        std::env::temp_dir().join(format!("al-gui-prefs-test-{}-corrupt", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("gui-preferences.json");
    std::fs::write(&path, "{ not json").unwrap();
    let prefs = GuiPreferences::load_or_default(&path);
    assert_eq!(prefs, GuiPreferences::default());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn preferences_save_then_load() {
    let dir = std::env::temp_dir().join(format!("al-gui-prefs-test-{}", std::process::id()));
    let path = dir.join("nested").join("gui-preferences.json");
    let prefs = GuiPreferences {
        theme_mode: ThemeMode::Light,
        ..GuiPreferences::default()
    };
    prefs.save(&path).unwrap();
    let back = GuiPreferences::load(&path).unwrap();
    assert_eq!(prefs, back);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn resolved_mode_matrix() {
    let system = GuiPreferences {
        theme_mode: ThemeMode::System,
        ..GuiPreferences::default()
    };
    assert_eq!(system.resolved_mode(Some("light")), "light");
    assert_eq!(system.resolved_mode(Some("dark")), "dark");
    assert_eq!(system.resolved_mode(Some("bogus")), FALLBACK_MODE);
    assert_eq!(system.resolved_mode(None), FALLBACK_MODE);
    let light = GuiPreferences {
        theme_mode: ThemeMode::Light,
        ..GuiPreferences::default()
    };
    assert_eq!(light.resolved_mode(None), "light");
    assert_eq!(light.resolved_mode(Some("dark")), "light");
}

#[test]
fn window_spec_maps_prefs() {
    let prefs = GuiPreferences {
        window: WindowPrefs {
            width: 1024,
            height: 640,
            resizable: false,
        },
        ..GuiPreferences::default()
    };
    let spec = prefs.window_spec("AudioLens");
    assert_eq!(spec.title, "AudioLens");
    assert_eq!(spec.logical_size.width, 1024);
    assert_eq!(spec.logical_size.height, 640);
    assert!(!spec.resizable);
}

#[test]
fn resolved_theme_values_are_typed() {
    let theme = theme::resolve("dark").unwrap();
    // product 層の値は ResolvedValue::Color として取れる。
    match theme.get("color.accent") {
        Some(ResolvedValue::Color(_)) => {}
        other => panic!("color.accent should be a color, got {other:?}"),
    }
}
