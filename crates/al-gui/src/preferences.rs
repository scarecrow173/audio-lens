//! `GuiPreferences`: AudioLens が所有する GUI 設定 (AL-G2)。
//!
//! theme mode・window 初期状態・既定 base_dir を JSON file へ
//! 保存/復元する。std env のみで OS 別 path を解決し、新外部依存は
//! 持たない。DTCG 解釈は [`crate::theme`]/ruviform-ui-theme に委譲し、
//! ここには mode 選択と window 射影のみを置く。

use std::path::PathBuf;

use ruviform_ui::{LogicalSize, WindowSpec};
use serde::{Deserialize, Serialize};

use crate::theme::FALLBACK_MODE;

/// preferences 保存 path を上書きする環境変数
/// (test・portable 運用用)。
pub const PREFERENCES_ENV: &str = "AL_GUI_PREFERENCES";

/// theme mode。`System` は OS 通知値または `FALLBACK_MODE` に解決する。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemeMode {
    /// OS の外観設定に従う。
    #[default]
    System,
    /// 常に light。
    Light,
    /// 常に dark。
    Dark,
}

/// window の初期ジオメトリとリサイズ可否。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowPrefs {
    /// 初期幅(logical px)。
    pub width: u32,
    /// 初期高さ(logical px)。
    pub height: u32,
    /// リサイズ可否。
    pub resizable: bool,
}

impl Default for WindowPrefs {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 800,
            resizable: true,
        }
    }
}

/// GUI 全体の preferences。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GuiPreferences {
    /// theme mode。
    pub theme_mode: ThemeMode,
    /// window 初期状態。
    pub window: WindowPrefs,
    /// `al-gui` binary の既定 base_dir 相当。
    pub base_dir: Option<PathBuf>,
}

impl GuiPreferences {
    /// `path` から読む。無い/不正な場合は `Err` を返し、
    /// `load_or_default` が fallback を担う。
    ///
    /// # Errors
    /// 読み取りまたは JSON 解釈に失敗した場合。
    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let text = std::fs::read_to_string(path)?;
        serde_json::from_str(&text)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// `path` へ保存する。親 dir は必要に応じて作成する。
    ///
    /// # Errors
    /// 書き込みに失敗した場合。
    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, text)
    }

    /// `path` から読み、失敗時は default を返す。
    #[must_use]
    pub fn load_or_default(path: &std::path::Path) -> Self {
        Self::load(path).unwrap_or_default()
    }

    /// `theme_mode` を resolver が受け付ける mode 名へ解決する。
    /// `System` 時は OS 通知値 `system` が `MODES` にあればそれを、
    /// 無ければ `FALLBACK_MODE` を返す。
    #[must_use]
    pub fn resolved_mode(&self, system: Option<&str>) -> &'static str {
        match self.theme_mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => match system {
                Some("light") => "light",
                Some("dark") => "dark",
                _ => FALLBACK_MODE,
            },
        }
    }

    /// window 初期値を `WindowSpec` へ写す(position は OS 任せ)。
    #[must_use]
    pub fn window_spec(&self, title: impl Into<String>) -> WindowSpec {
        WindowSpec {
            title: title.into(),
            logical_size: LogicalSize {
                width: self.window.width,
                height: self.window.height,
            },
            position: None,
            resizable: self.window.resizable,
        }
    }
}

/// OS 別の既定 preferences path。
/// 優先順位: `AL_GUI_PREFERENCES` env > OS 別 config dir > `None`。
#[must_use]
pub fn default_path() -> Option<PathBuf> {
    if let Some(raw) = std::env::var_os(PREFERENCES_ENV)
        && !raw.is_empty()
    {
        return Some(PathBuf::from(raw));
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|p| p.join("AudioLens").join("gui-preferences.json"))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|h| {
            PathBuf::from(h)
                .join("Library")
                .join("Application Support")
                .join("AudioLens")
                .join("gui-preferences.json")
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join("audiolens").join("gui-preferences.json"))
    }
    #[cfg(not(any(windows, target_os = "macos", unix)))]
    {
        None
    }
}
