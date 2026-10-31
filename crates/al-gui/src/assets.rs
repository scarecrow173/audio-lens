//! 埋め込み frontend assets。`rvf://app` から提供する
//! HTML/JS/CSS をここで `MapAssetSource` に束ねる(追加の
//! asset 埋め込み依存を増やさない)。

use std::sync::Arc;

use ruviform_ui::{AssetSource, MapAssetSource};

/// `rvf://app` 向けの asset 集合を構築する。
pub fn app_assets() -> Arc<dyn AssetSource> {
    Arc::new(
        MapAssetSource::new()
            .with_asset(
                "index.html",
                None,
                include_bytes!("../assets/index.html").as_slice(),
            )
            .with_asset(
                "app.js",
                None,
                include_bytes!("../assets/app.js").as_slice(),
            )
            .with_asset(
                "app.css",
                None,
                include_bytes!("../assets/app.css").as_slice(),
            ),
    )
}
