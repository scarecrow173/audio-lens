---
id: "TSPEC-034YaRb4ebaeqPddL37Fqv"
type: "test-spec"
status: "approved"
title: "ruviform eafe0a08 pin 追従の検証仕様"
created: "2026-11-16"
updated: "2026-11-16"
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
    - "docs/designs/ruviform-eafe0a08-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-eafe0a08-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-eafe0a08-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform eafe0a08 pin 追従の検証仕様

## Purpose

pin 追従 tuple `abb9f372` → `eafe0a08` の検証要求を、実行した
lane・結果・未実行項目の対応として固定する。未実行 gate を Pass
に繰り上げないことを検証仕様として明文化する。

## Feature

- Feature: ruviform pin 追従（lockfile-primary）と candidate tuple
  検証（AL#127 / ruviform#219/#294 の Consumer 側実施）
- 検証対象 revision: `eafe0a08b71bd9a74566ae6678c3e0683fce1421`
- 実行環境: Windows（worktree `al127-ruviform-eafe0a08`）、mise、
  Rust 1.99.1、Node 26.12.1

## Rules

| Rule | 検証 lane（正規 = mise task） | 期待 | 実測結果 |
| --- | --- | --- | --- |
| R1: lockfile が単一 resolved SHA を持つ | `mise run check-ruviform-lock` | 26 package が `eafe0a08...` に解決 | Pass（exit 0） |
| R2: lockfile/manifest/vendor が一致 | `mise run check-ruviform-rev` | REV = lockfile SHA、不一致で fail | Pass（exit 0） |
| R3: vendor が対象 SHA から再生成される | `mise run sync-ui-deps` → `check-ui` → `test-ui` | `ui/vendor/ruviform/REV` = `eafe0a08...`、typecheck+113 tests pass | Pass（exit 0、113 tests fail 0） |
| R4: 対象 crate の Rust 検証が健全 | `mise run fmt` `check al-analysis` `lint al-analysis` `test al-analysis` | fmt 差分なし、clippy `-D warnings` pass、全 suite fail 0（`rf628_oracle` 7 tests・長尺 scenario 含む） | Pass（全て exit 0） |
| R5: headless crate に GUI 依存がない | `mise run check-cli-headless` | 10 headless crate に GUI framework 依存なし（`serde`/`serde_json` の dev-dep 波及なし） | Pass（exit 0） |
| R6: toolchain・locked fetch が健全 | `mise run verify-toolchain`、`mise exec -- cargo fetch --locked` | toolchain 一致、locked fetch 成功 | Pass（全て exit 0） |
| R7: 未実行 gate を Pass にしない | 記録レビュー | 実 OS GUI/実 Pixel/macOS/Linux/cross-consumer/`gui-real`/upstream covering CI（`eafe0a08` 対象の `ci`/`daily-ci`/`consumer-verify`）/ruviform manual lane は NotRun/Unsupported | 本 spec と PR 本文に記録 |
| R8: Issue 指定 task 名の存在確認 | `mise tasks ls` / mise.toml 確認 | Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/`verify-ci` は現行 mise.toml に存在しない | 現行 task で代替実行・記録 |

## Examples

- Example（R1/R2）: `Cargo.lock` の ruviform package `source` が
  全て `git+https://github.com/scarecrow173/ruviform.git#
  eafe0a08b71bd9a74566ae6678c3e0683fce1421` であり、
  `ui/vendor/ruviform/REV` が同 SHA を含む。`Cargo.lock` の差分は
  SHA fragment 26 行 + `ruviform-audio-test` の `serde`/
  `serde_json` 依存 edge 2 行のみで、新規 package 追加・version
  変動・他 crate への推移的依存変動はない。
- Example（R4）: `test al-analysis` は `tests/rf628_oracle.rs`
  の 7 tests（fixture/oracle 一致・f64 fidelity・長尺非回帰・
  gap/Validity）を含む全 suite が Pass。`rf628_long_scenario_
  non_regression`（30s contiguous scenario）は実走で約 84 秒。
- Example（R5）: `ruviform-audio-test` への `serde`/`serde_json`
  追加は同 crate の `[dependencies]` に閉じ、al-analysis の
  `[dev-dependencies]` 経路でのみ解決される。headless 10 crate
  の依存木に GUI framework 依存がないことを
  `check-cli-headless` が確認。

## Guarantees

- pin 追従は manifest を変更せず lockfile のみで行われ、
  `check-ruviform-lock`/`check-ruviform-rev` が単一 SHA・vendor
  一致を fail-closed に保証する。
- `ui/vendor/ruviform` は `sync-ui-deps` の生成物であり、
  `REV` が lockfile SHA と一致する。本範囲の ruviform 差分に
  `ui/*` TS source の変更はなく、vendor 内容は REV 以外不変。
- 全検証は正規 mise task 経由で実行され、cargo 直実行は補助証拠
  としてのみ扱う。
- 実機・実 Pixel・macOS/Linux・cross-consumer・upstream covering
  CI・ruviform manual lane は NotRun/Unsupported として記録され、
  semantic/headless の Pass を実機 Pass に読み替えない。

## Non-goals

- 実 OS GUI・実 Pixel golden・IME・DPI の検証（環境到達不能、
  NotRun）。
- macOS/Linux での lane 実行（Unsupported: 実行環境は Windows のみ）。
- cross-consumer（Synvelis 側）の非回帰確認（Coordinator 側責務、
  ruviform#219/#294 への報告は Coordinator が行う）。
- ruviform 新 API/feature（RunMetadata JSON・`StreamFault::
  Cancellation`・plugin state bridge・test_editor/test_stateful
  fixture・plugin-dock real-editor attach）の AL 採用（本 slice
  では採用しない判断を記録するのみ）。
- ruviform 側 `test-l1-real-plugin` 等 manual/opt-in lane の実行
  （AL の検証 lane ではない）。
- ruviform 側の新規 Issue 起票（契約問題は検出されなかった）。

## Risk

- upstream に plugin state bridge・test fixture・real-editor
  attach など EXT-L3 関連の大規模新機能が入ったが AL lane では
  未踏 → candidate tuple としての継続評価（Issue #127 は open の
  まま）で担保。
- `ruviform-audio-test` の `#[non_exhaustive]` enum への variant
  追加は additive で、AL 側に exhaustive match は存在しない
  ことを確認済み。serde derive 追加も非破壊。
- upstream main の covering CI は `eafe0a08` に対して NotRun
  （最新 covering run は `ac10a2f7` 対象）。AL merge 後の対象
  SHA の CI 結果が統合検証の証拠。
- doc graph の既存 broken relation（`crates/al-math` 参照切れ）は
  main 既存問題で本 slice 範囲外。文書チェーンは前段と同じく
  直接作成 + review-protocol で代替。

## 実行結果サマリ（2026-11-16、Windows）

| lane | 結果 |
| --- | --- |
| check-ruviform-lock / check-ruviform-rev | Pass |
| sync-ui-deps → check-ui → test-ui | Pass（113 tests） |
| fmt / check al-analysis / lint al-analysis / test al-analysis | Pass（`rf628_oracle` 含む全 suite fail 0） |
| check-cli-headless | Pass |
| verify-toolchain / cargo fetch --locked | Pass |
| check-full（補助証拠・workspace check） | Pass |
| lint-full / test-full / build-full / check-ui-gpui / lint-ui-gpui / test-ui-gpui / build-ui-gpui / check-ui-webview / lint-ui-webview / test-ui-webview / check-ci-workflow / check-ci-tracker-workflows | NotRun（本 slice の正規検証コマンド対象外。変更は al-analysis dev-dep 経路に限定） |
| 実 OS GUI / 実 Pixel / macOS / Linux / cross-consumer / gui-real / upstream covering CI（eafe0a08 対象）/ ruviform manual lane | NotRun / Unsupported |

## 採用/保留判断と rollback

- 判断: **採用**（pin 追従として）。全正規検証 lane が対象
  revision で Pass し、破壊的 API/Feature/OS 変更・AL 側の恒久
  workaround 要求は検出されなかった。upstream 新 API/feature
  （RunMetadata JSON・`StreamFault::Cancellation`・plugin state
  bridge・test_editor/test_stateful fixture・plugin-dock
  real-editor attach）は opt-in/test-only/依存木外であり、AL
  側の採用 gate・需要が未定義のため本 slice では**採用しない**。
  実機系 gate は NotRun のまま別途追跡する（semantic/headless
  の Pass を実機 acceptance と混同しない）。
- rollback: `Cargo.lock` を `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`
  へ戻す revert + `mise run sync-ui-deps` 再実行で旧 pin 状態に
  復帰可能（製品コード差分がないため影響は lockfile と vendor
  生成物に限定）。
