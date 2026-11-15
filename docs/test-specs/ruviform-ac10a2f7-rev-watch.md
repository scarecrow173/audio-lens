---
id: "TSPEC-034XLUrPKy2kz5PYTSGInn"
type: "test-spec"
status: "approved"
title: "ruviform ac10a2f7 pin 追従の検証仕様"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
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
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
    - "docs/tasks/ruviform-ac10a2f7-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-ac10a2f7-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform ac10a2f7 pin 追従の検証仕様

## Purpose

pin 追従 tuple `c6e4f334` → `ac10a2f7` の検証要求を、実行した
lane・結果・未実行項目の対応として固定する。未実行 gate を Pass
に繰り上げないことを検証仕様として明文化する。

## Feature

- Feature: ruviform pin 追従（lockfile-primary）と candidate tuple
  検証（AL#127 / ruviform#219/#294 の Consumer 側実施）
- 検証対象 revision: `ac10a2f7761c08db73aebd861c0a5c1f53f269b7`
- 実行環境: Windows（worktree `al127-rev-watch`）、mise、
  Rust 1.99.1、Node 26.12.1

## Rules

| Rule | 検証 lane（正規 = mise task） | 期待 | 実測結果 |
| --- | --- | --- | --- |
| R1: lockfile が単一 resolved SHA を持つ | `mise run check-ruviform-lock` | 20 package が `ac10a2f7...` に解決 | Pass（exit 0） |
| R2: lockfile/manifest/vendor が一致 | `mise run check-ruviform-rev` | REV = lockfile SHA、不一致で fail | Pass（exit 0） |
| R3: vendor が対象 SHA から再生成される | `mise run sync-ui-deps` → `check-ui` → `test-ui` | `ui/vendor/ruviform/REV` = `ac10a2f7...`、typecheck+113 tests pass | Pass（exit 0、113 tests fail 0） |
| R4: Rust workspace が健全 | `mise run fmt` `check-full` `lint-full` `test-full` `build-full` | fmt 差分なし、clippy `-D warnings` pass、58 suite fail 0 | Pass（全て exit 0） |
| R5: headless crate に GUI 依存がない | `mise run check-cli-headless` | 10 headless crate に GUI framework 依存なし | Pass（exit 0） |
| R6: GPUI semantic/headless lane | `mise run check-ui-gpui` `lint-ui-gpui` `test-ui-gpui` `build-ui-gpui` | `gpui`/`gpui-test` feature で compile/lint/test/build pass | Pass（exit 0） |
| R7: Legacy WebView lane が維持される | `mise run check-ui-webview` `lint-ui-webview` `test-ui-webview` | `webview` feature で compile/lint/test pass | Pass（exit 0） |
| R8: CI 契約・toolchain・fetch が健全 | `mise run check-ci-workflow` `check-ci-tracker-workflows` `verify-toolchain`、`mise exec -- cargo fetch --locked` | workflow 構造契約 OK、toolchain 一致、locked fetch 成功 | Pass（全て exit 0） |
| R9: 未実行 gate を Pass にしない | 記録レビュー | 実 OS GUI/実 Pixel/macOS/Linux/cross-consumer/`gui-real`/ruviform `test-ui-plugin-dock`/`test-ui-real-scenario`/windowed `uitest` lane は NotRun/Unsupported | 本 spec と PR 本文に記録 |
| R10: Issue 指定 task 名の存在確認 | `mise tasks ls` | Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/`verify-ci` は現行 mise.toml に存在しない | 現行の `*-full`/個別 task で代替実行・記録 |

## Examples

- Example（R1/R2）: `Cargo.lock` の ruviform package `source` が
  全て `git+https://github.com/scarecrow173/ruviform.git#
  ac10a2f7761c08db73aebd861c0a5c1f53f269b7` であり、
  `ui/vendor/ruviform/REV` が同 SHA を含む。`Cargo.lock` の差分は
  SHA fragment 20 行のみで、推移的依存の変動・新規 package 追加は
  ない（`ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`
  等の新規 crate は AL の依存木に入らない）。
- Example（R6 と実機の区別）: `test-ui-gpui` は `GpuiSimulated`
  driver の semantic/headless 検証であり、実 OS 入力・実 pixel・
  実 window の証拠ではない。upstream の `ruviform-ui-test`
  `PluginHostProbe` driver・`plugin_dock` scenario（Issue #631）と
  ruviform 側 `test-ui-plugin-dock` lane（manual/opt-in、
  `rf631.plugin-dock` cases）は AL では採用・有効化しておらず、
  実機相当は NotRun。`test-ui-plugin-dock` は CI-required gate
  ではないことが upstream `mise.toml` の task description に
  明記されている。
- Example（R7）: WebView lane の Pass は Legacy 経路の維持確認で
  あり、GPUI/実 OS/pixel acceptance の代替にはならない。
  本範囲の ruviform 差分に `ruviform-ui-webview` の変更はない。

## Guarantees

- pin 追従は manifest を変更せず lockfile のみで行われ、
  `check-ruviform-lock`/`check-ruviform-rev` が単一 SHA・vendor
  一致を fail-closed に保証する。
- `ui/vendor/ruviform` は `sync-ui-deps` の生成物であり、
  `REV` が lockfile SHA と一致する。本範囲の ruviform 差分に
  `ui/*` TS source の変更はなく、vendor 内容は REV 以外不変。
- 全検証は正規 mise task 経由で実行され、cargo 直実行は補助証拠
  としてのみ扱う。
- 実機・実 Pixel・macOS/Linux・cross-consumer・`gui-real`・
  ruviform `test-ui-plugin-dock`/`test-ui-real-scenario`・
  windowed `uitest` lane は NotRun/Unsupported として記録され、
  semantic/headless の Pass を実機 Pass に読み替えない。

## Non-goals

- 実 OS GUI・実 Pixel golden・IME・DPI の検証（環境到達不能、
  NotRun）。
- macOS/Linux での lane 実行（Unsupported: 実行環境は Windows のみ）。
- cross-consumer（Synvelis 側）の非回帰確認（Coordinator 側責務、
  ruviform#219/#294 への報告は Coordinator が行う）。
- ruviform 新 API/feature（`ruviform-plugin-dock-test`・
  `ruviform-plugin-test-vst3`・ui-test plugin_dock scenario/
  `PluginHostProbe`・endpoint probe examples・io-e2e opt-in lanes・
  plugin-core MIDI event contract）の AL 採用（本 slice では
  採用しない判断を記録するのみ）。
- ruviform 側 `test-ui-plugin-dock`・`test-ui-real-scenario`・
  windowed `uitest` lane・Linux offscreen golden lane・opt-in
  実機 lane の実行（AL の検証 lane ではない）。
- ruviform 側の新規 Issue 起票（契約問題は検出されなかった）。

## Risk

- upstream に plugin-dock scenario harness・VST3 test fixture・
  endpoint probe・io-e2e opt-in lane・MIDI event contract 修正
  など大規模な新機能が入ったが AL lane では未踏 → candidate
  tuple としての継続評価（Issue #127 は open のまま）で担保。
- `ruviform-ui-test` の `ScenarioDriver` enum への variant 追加は
  additive で、AL 側に exhaustive match がないことを確認済み。
  AL の dev-dependency 経路の実動作は workspace test の Pass で
  担保する。
- doc graph の既存 broken relation（`crates/al-math` 参照切れ）は
  main 既存問題で本 slice 範囲外。route_graph は blocked を返す
  ため文書チェーンは手動作成 + review-protocol で代替。

## 実行結果サマリ（2026-11-15、Windows）

| lane | 結果 |
| --- | --- |
| check-ruviform-lock / check-ruviform-rev | Pass |
| sync-ui-deps → check-ui → test-ui | Pass（113 tests） |
| fmt / check-full / lint-full / test-full / build-full | Pass（58 suite fail 0） |
| check-cli-headless | Pass |
| check-ui-gpui / lint-ui-gpui / test-ui-gpui / build-ui-gpui | Pass |
| check-ui-webview / lint-ui-webview / test-ui-webview | Pass |
| check-ci-workflow / check-ci-tracker-workflows / verify-toolchain / cargo fetch --locked | Pass |
| 実 OS GUI / 実 Pixel / macOS / Linux / cross-consumer / gui-real / ruviform test-ui-plugin-dock / test-ui-real-scenario / uitest windowed lane | NotRun / Unsupported |

## 採用/保留判断と rollback

- 判断: **採用**（pin 追従として）。全正規検証 lane が対象
  revision で Pass し、破壊的 API/Feature/OS 変更・AL 側の恒久
  workaround 要求は検出されなかった。upstream 新 API/feature
  （`ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`・
  ui-test plugin_dock scenario/`PluginHostProbe`・endpoint probe
  examples・io-e2e opt-in lanes・plugin-core MIDI event
  contract）は test-only/opt-in/依存木外であり、AL 側の採用
  gate・需要が未定義のため本 slice では**採用しない**。実機系
  gate は NotRun のまま別途追跡する（semantic/headless の Pass
  を実機 acceptance と混同しない）。
- rollback: `Cargo.lock` を `c6e4f334242567dbd0e0d8b7713d9d480a6aab66`
  へ戻す revert + `mise run sync-ui-deps` 再実行で旧 pin 状態に
  復帰可能（製品コード差分がないため影響は lockfile と vendor
  生成物に限定）。
