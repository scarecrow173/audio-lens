---
id: "TSPEC-034Vc1k9l2KQszbMWnGXdx"
type: "test-spec"
status: "approved"
title: "ruviform 271e2d41 pin 追従の検証仕様"
created: "2026-11-10"
updated: "2026-11-10"
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
    - "docs/designs/ruviform-271e2d41-rev-watch.md"
    - "docs/discovery/issue-127-legacy-webview-inventory.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/ruviform-271e2d41-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-271e2d41-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform 271e2d41 pin 追従の検証仕様

## Purpose

pin 追従 tuple `84d21641` → `271e2d41` の検証要求を、実行した
lane・結果・未実行項目の対応として固定する。未実行 gate を Pass
に繰り上げないことを検証仕様として明文化する。

## Feature

- Feature: ruviform pin 追従（lockfile-primary）と candidate tuple
  検証（AL#127 / ruviform#219/#294 の Consumer 側実施）
- 検証対象 revision: `271e2d4111d3769693445bc5bffca37af9e0650b`
- 実行環境: Windows（worktree
  `al127-rev-watch`）、mise 2026.10.1、Rust 1.99.0、Node 26.12.1

## Rules

| Rule | 検証 lane（正規 = mise task） | 期待 | 実測結果 |
| --- | --- | --- | --- |
| R1: lockfile が単一 resolved SHA を持つ | `mise run check-ruviform-lock` | 20 package が `271e2d41...` に解決 | Pass（exit 0） |
| R2: lockfile/manifest/vendor が一致 | `mise run check-ruviform-rev` | REV = lockfile SHA、不一致で fail | Pass（exit 0） |
| R3: vendor が対象 SHA から再生成される | `mise run sync-ui-deps` → `check-ui` → `test-ui` | `ui/vendor/ruviform/REV` = `271e2d41...`、typecheck+112 tests pass | Pass（exit 0、112 tests fail 0） |
| R4: Rust workspace が健全 | `mise run fmt` `check` `lint` `test` `build` | fmt 差分なし、clippy `-D warnings` pass、58 suite fail 0 | Pass（全て exit 0） |
| R5: headless crate に GUI 依存がない | `mise run check-cli-headless` | 10 headless crate に GUI framework 依存なし | Pass（exit 0） |
| R6: GPUI semantic/headless lane | `mise run check-ui-gpui` `lint-ui-gpui` `test-ui-gpui` | `gpui-test` feature で compile/lint/test pass | Pass（exit 0、parity/desktop/screen/theme 等全 pass） |
| R7: Legacy WebView lane が維持される | `mise run check-ui-webview` `lint-ui-webview` `test-ui-webview` | `webview` feature で compile/lint/test pass | Pass（exit 0） |
| R8: CI profile が健全 | `mise run check-ci` `lint-ci` `test-ci` | `ci` profile で check/clippy/test pass | Pass（exit 0） |
| R9: 未実行 gate を Pass にしない | 記録レビュー | 実 OS GUI/実 Pixel/macOS/Linux/cross-consumer/`gui-real` は NotRun/Unsupported | 本 spec と PR 本文に記録 |

## Examples

- Example（R1/R2）: `Cargo.lock` の ruviform package `source` が
  全て `git+https://github.com/scarecrow173/ruviform.git#
  271e2d4111d3769693445bc5bffca37af9e0650b` であり、
  `ui/vendor/ruviform/REV` が同 SHA を含む。
- Example（R6 と実機の区別）: `test-ui-gpui` は `GpuiSimulated`
  driver の semantic/headless 検証であり、実 OS 入力・実 pixel・
  実 window の証拠ではない。実機相当は NotRun。
- Example（R7）: WebView lane の Pass は Legacy 経路の維持確認で
  あり、GPUI/実 OS/pixel acceptance の代替にはならない。

## Guarantees

- pin 追従は manifest を変更せず lockfile のみで行われ、
  `check-ruviform-lock`/`check-ruviform-rev` が単一 SHA・vendor
  一致を fail-closed に保証する。
- `ui/vendor/ruviform` は `sync-ui-deps` の生成物であり、
  `REV` が lockfile SHA と一致する。
- 全検証は正規 mise task 経由で実行され、cargo 直実行は補助証拠
  としてのみ扱う。
- 実機・実 Pixel・macOS/Linux・cross-consumer・`gui-real` は
  NotRun/Unsupported として記録され、semantic/headless の Pass を
  実機 Pass に読み替えない。

## Non-goals

- 実 OS GUI・実 Pixel golden・IME・DPI の検証（環境到達不能、
  NotRun）。
- macOS/Linux での lane 実行（Unsupported: 実行環境は Windows のみ）。
- cross-consumer（Synvelis 側）の非回帰確認（Coordinator 側責務、
  ruviform#219/#294 への報告は Coordinator が行う）。
- Legacy WebView の物理撤去と撤去後の再検証（別 slice/別 Issue）。
- ruviform 側の新規 Issue 起票（契約問題は検出されなかった）。

## Risk

- 88 commit の大規模差分に対し AL 側 lane だけでは upstream の
  全挙動を保証できない → candidate tuple としての継続評価
  （Issue #127 は open のまま）で担保。
- `proc-macro-error2 v2.0.1` の future-incompatibility warning が
  lint 時に出力される（依存由来、抑制せず記録のみ）。
- doc graph の既存 broken relation（`crates/al-math` 参照切れ）は
  main 既存問題で本 slice 範囲外。route_graph は blocked を返す
  ため文書チェーンは手動作成 + review-protocol で代替。

## 実行結果サマリ（2026-11-10、Windows）

| lane | 結果 |
| --- | --- |
| check-ruviform-lock / check-ruviform-rev | Pass |
| sync-ui-deps → check-ui → test-ui | Pass（112 tests） |
| fmt / check / lint / test / build | Pass（58 suite fail 0） |
| check-cli-headless | Pass |
| check-ui-gpui / lint-ui-gpui / test-ui-gpui | Pass |
| check-ui-webview / lint-ui-webview / test-ui-webview | Pass |
| check-ci / lint-ci / test-ci | Pass |
| 実 OS GUI / 実 Pixel / macOS / Linux / cross-consumer / gui-real | NotRun / Unsupported |

## 採用/保留判断と rollback

- 判断: **採用**。全正規検証 lane が対象 revision で Pass し、
  破壊的 API/Feature/OS 変更・AL 側の恒久 workaround 要求は
  検出されなかった。実機系 gate は NotRun のまま別途追跡する
  （semantic/headless の Pass を実機 acceptance と混同しない）。
- rollback: `Cargo.lock` を `84d21641a6bd50fa570ed074ae263284d05262ca`
  へ戻す revert + `mise run sync-ui-deps` 再実行で旧 pin 状態に
  復帰可能（製品コード差分がないため影響は lockfile と vendor
  生成物に限定）。
