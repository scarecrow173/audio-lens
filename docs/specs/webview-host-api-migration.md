---
id: "SPEC-KeMjbsfCWwN7IG007JihND"
type: "spec"
status: "approved"
title: "al-gui webview host の driver-neutral API 移行（CI webview-verify 修正）(Issue #176)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/176"
    - "https://github.com/scarecrow173/ruviform/issues/423"
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
  related:
    - "../designs/webview-host-api-migration.md"
    - "../test-specs/webview-host-api-migration.md"
  refines:
    - "../designs/gui-app-architecture.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/webview-host-api-migration.md"
  references: []
  defers: []
  deferred-by: []
---

# al-gui webview host の driver-neutral API 移行（CI webview-verify 修正）(Issue #176)

## What Should Be Built

`crates/al-gui/src/bin/al-gui.rs`(webview feature の実窓 host
binary)を、ruviform pin `863a7a9d` の `ruviform-ui-webview` が提供する
driver-neutral 契約(`run_multi` + `SessionConfig`/`WebViewWindowConfig`
+ `WindowDriver`/`DriverEvent`)へ移行し、CI `webview-verify` lane
(`check-ui-webview` / `lint-ui-webview` / `test-ui-webview`)が 3 OS
全てで pass する状態を実現する。

## Why Now

CI run 42431214830(main `65160914`、workflow_dispatch)で
`webview-verify` が windows/ubuntu/macos の 3 OS 全て failure となり
Issue #176 が起票された。他 job は全て success。原因は AL 側:
`al-gui.rs` が ruviform `f3f474b` 時点の legacy single-window API
(`HostConfig`/`run`)を import したままで、pin が ruviform `38b7a46e`
(RF#423 / PR #430、driver-neutral `WindowDriver` 契約への移行)を
越えた後も追従されていなかった。`error[E0432]: unresolved imports
ruviform_ui_webview::HostConfig, ruviform_ui_webview::run` で
`cargo check` が失敗する。現行 main `0f8a3b0`(pin `863a7a9d`)でも
同じ import が残っており、未修正のまま。

## Users and Value

- AudioLens 開発者: `webview-verify` lane が再び機能し、webview host
  binary の compile/lint/test が CI で検証される。
- AudioLens 利用者: webview GUI binary が現行 ruviform 契約で build
  できる状態に戻る(GPUI native desktop が主戦場だが、webview host は
  残存 lane として維持される)。

## Scope

### In Scope

- `al-gui.rs` を `run_multi` + `SessionConfig` + `DesktopSession`
  (driver-neutral、GPUI 側 `al-desktop` と同一の event routing /
  close・job policy)へ移植する。
- 移行の設計判断・検証結果・実装記録を文書チェーンとして残す。

### Out of Scope

- ruviform 側の契約変更・修正。legacy API 削除は RF#423 の意図的な
  移行であり、AL 側で恒久 workaround(旧 API の再実装・互換層)は
  置かない。
- webview frontend(assets/app.js 等)の機能変更。op 契約
  (`al.*` / `job.cancel` / `rvf://data`)は不変。
- `al-desktop`(GPUI)側の変更。`DesktopSession` は既に driver-neutral
  であり変更不要。
- 実 OS window での手動 GUI 操作検証。CI lane は compile/lint/test
  のみで実窓を開かない(既存 lane 契約どおり)。

## Requirements

- `cargo check/clippy/test -p al-gui --features webview` が pin
  `863a7a9d` で pass すること(`mise run check-ui-webview` /
  `lint-ui-webview` / `test-ui-webview`)。
- event routing は `ruviform-ui` の canonical mapping
  (`Action(Intent)` → `dispatch_window_intent`、`Tick` →
  `drain_jobs_routed`、`CloseRequested` → closing lifecycle +
  `CloseDecision`、`Destroyed` → teardown policy)に従うこと。
- window close/destroy が実行中 job を暗黙停止しないこと
  (gui-app-architecture.md ライフサイクル節、Issue #166 の受入と同一
  policy を `DesktopSession` 経由で共有する)。

## Acceptance Criteria

- [x] `webview-verify` の失敗原因を特定し root cause を記録
  (AL 側: legacy API 未追従。ruviform 側・CI 環境・flaky ではない)。
- [x] `al-gui.rs` が `run_multi`/`WindowDriver` 契約で compile でき、
  `check-ui-webview` / `lint-ui-webview` / `test-ui-webview` が
  ローカルで pass すること。
- [x] AL 側に恒久 workaround を置かず、driver-neutral 契約へ正しく
  移行すること。
- [ ] CI `webview-verify` job が PR の CI で pass すること
  (3 OS。PR 作成後の CI 結果で確認)。

## Deferred Design Concerns

- なし。

## Sources and Relations

- Issue #176(CI failure tracker)、失敗 run 42431214830
  (sha `65160914`)。
- ruviform RF#423 / PR #430(`HostConfig`/`run` 削除、
  `run_multi`/`WindowDriver` 契約への移行)、RF#181 / PR #237
  (multi-window host contracts)。
- `gui-app-architecture.md`: al-gui の責務境界とライフサイクル
  (window close と job 継続の分離)。
- `native-desktop-initial-slices.md` / `desktop.rs`:
  `DesktopSession` の driver-neutral event routing と close/job
  policy(GPUI 側で確立済みの正本実装)。
