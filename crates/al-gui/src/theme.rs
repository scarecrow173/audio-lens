//! AudioLens product theme (AL-G2)。
//!
//! `theme/base.tokens.json`(primitive 層)と
//! `theme/product.tokens.json`(product 層)を crate 埋め込みし、
//! DTCG 解釈・単位・alias・mode・override の解決は全て
//! ruviform-ui-theme (RF#385) の `TokenDocument`/`ThemeResolver`
//! へ委譲する。独自の token 型・単位・パースは持たない。

use ruviform_ui_theme::{ResolveError, ResolvedTheme, ThemeResolver, TokenDocument, TokenError};

/// `resolve` が受け付ける mode 名(`GuiPreferences::resolved_mode`
/// と一致させる)。
pub const MODES: &[&str] = &["light", "dark"];

/// 無効/未指定の mode に対する既定値。
pub const FALLBACK_MODE: &str = "dark";

const BASE_TOKENS: &str = include_str!("../theme/base.tokens.json");
const PRODUCT_TOKENS: &str = include_str!("../theme/product.tokens.json");

/// crate 埋め込みの product token 文書を解釈順で返す。
///
/// # Errors
/// 埋め込み JSON が DTCG profile として不正なとき(実装ミスのみ;
/// 起動前の `resolve` で検出される)。
pub fn documents() -> Result<Vec<TokenDocument>, TokenError> {
    Ok(vec![
        TokenDocument::from_json(BASE_TOKENS)?,
        TokenDocument::from_json(PRODUCT_TOKENS)?,
    ])
}

/// `mode` (`MODES` のいずれか)で product theme を解決する。
///
/// # Errors
/// `mode` が `MODES` に無い場合、または token 解決(alias/layer/
/// mode)が失敗した場合。
pub fn resolve(mode: &str) -> Result<ResolvedTheme, ResolveError> {
    let docs = documents().map_err(ResolveError::Document)?;
    ThemeResolver::resolve(&docs, mode)
}
