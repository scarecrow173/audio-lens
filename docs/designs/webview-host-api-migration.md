---
id: "DESIGN-HzMSjoqoRH2ibQWn8inlUp"
type: "design"
status: "approved"
title: "al-gui webview host の driver-neutral API 移行の設計"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/webview-host-api-migration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## 概要

`al-gui.rs` が import していた `ruviform_ui_webview::{HostConfig, run}`
は、ruviform `38b7a46e`(RF#423 / PR #430)で削除済みの legacy
single-window API である。後継は multi-window の driver-neutral 契約:
`run_multi(&SessionConfig, Arc<dyn AssetSource>, DataRegistry,
on_event)` が `DriverEvent` + `&mut dyn WindowDriver` を UI thread で
配送し、consumer が `WindowDriver` で window を制御する。

## 設計判断

- **`DesktopSession` を再利用する**: `al_gui::desktop::DesktopSession`
  は `&mut dyn WindowDriver` 経由で `DriverEvent` を消費する
  driver-neutral session であり、canonical mapping
  (`Action` → `dispatch_window_intent`、`Tick` → `drain_jobs_routed`、
  `CloseRequested` → closing + `CloseDecision`、`Destroyed` →
  teardown)と close/job policy(close は job を暗黙停止しない、
  最終 window + job 実行中は `Defer`、last-window `exit()`)を
  実装済み。`al-desktop`(GPUI)と同一の routing/policy を webview
  host にも適用し、第二の実装を作らない(DRY・責務の一元化)。
  webview frontend は `rvf.ui.command.v1` の command intent を発行
  しないため、session の command 語彙(select/cancel)分岐は
  webview 経路では休眠し、`job.cancel` op は従来どおり runtime の
  reserved op が処理する。
- **`SessionConfig` の写像**: 旧 `HostConfig` 既定値を維持する —
  `tick: Some(16ms)`、`data_dir: None`(process-unique temp UDF)、
  `devtools: cfg!(debug_assertions)`。window は 1 枚、
  `WindowSpec { title: "AudioLens", ..Default }`
  (logical 1280x800・resizable は `WindowSpec::default()` どおり)、
  `url`/`devtools` は `WebViewWindowConfig::default()` どおり
  `rvf://app/index.html`。
- **配送経路**: 旧 `host.eval(dispatch_script(&out))` は adapter の
  `WindowDriver::post_event` が `evaluate_script(dispatch_script(..))`
  として内包するため、consumer 側の写像は `DesktopSession::handle`
  の `deliver` に集約される。`RoutedEvent.window = None`(ownerless)
  は全 live window へ配送される session 側規則に従う。
- **プロセス終了**: `run_multi` は `WindowDriver::exit()` でのみ
  loop を抜ける。`DesktopSession` の `exit_on_last_window` policy
  (既定 true)と `maybe_exit`(live window 無し + job 無しでのみ
  `exit()`)がそのまま効く — job 実行中の window close は `Defer` で
  保留され、job 終端 drain 後に close が再開される。旧
  `UiRuntime::handle(CloseRequested)` の「全 job cancel」は
  gui-app-architecture.md のライフサイクル契約
  (window close と job 継続の分離)に反するため継承しない。

## 検証

- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`(cargo check/clippy/test `-p al-gui
  --features webview`)が pass すること。
- event routing / close・job policy の正しさは `DesktopSession` の
  既存 headless テスト(`crates/al-gui/tests/desktop.rs`、FakeDriver
  fixture)が担保する — webview binary は composition root であり
  独自の routing を持たない。
- 実 OS window での起動確認は本 lane の対象外(CI lane 契約どおり
  compile/lint/test のみ)。
