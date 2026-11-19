---
id: "TSPEC-034YnH8GZCnGo4aoFK6mgM"
type: "test-spec"
status: "approved"
title: "ruviform cbcc8343 pin 追従の検証仕様"
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
    - "docs/designs/ruviform-cbcc8343-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/ruviform-cbcc8343-rev-watch.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-cbcc8343-rev-watch.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform cbcc8343 pin 追従の検証仕様

## Purpose

pin 追従 tuple `eafe0a08` → `cbcc8343` の検証要求を、実行した
lane・結果・未実行項目の対応として固定する。未実行 gate を Pass
に繰り上げないことを検証仕様として明文化する。

## Feature

- Feature: ruviform pin 追従（lockfile-primary）と candidate tuple
  検証（AL#127 / ruviform#219/#294 の Consumer 側実施）
- 検証対象 revision: `cbcc83437616fdd7418da37183c6be66b6cfe225`
- 実行環境: Windows（worktree `al127-cbcc8343`）、mise、
  Rust 1.99.1、Node 26.12.1

## Rules

| Rule | 検証 lane（正規 = mise task） | 期待 | 実測結果 |
| --- | --- | --- | --- |
| R1: lockfile が単一 resolved SHA を持つ | `mise run check-ruviform-lock` | 26 package が `cbcc8343...` に解決 | Pass（exit 0） |
| R2: lockfile/manifest/vendor が一致 | `mise run check-ruviform-rev` | REV = lockfile SHA、不一致で fail | Pass（exit 0） |
| R3: vendor が対象 SHA から再生成される | `mise run sync-ui-deps` → `check-ui` → `test-ui` | `ui/vendor/ruviform/REV` = `cbcc8343...`、typecheck+113 tests pass | Pass（exit 0、113 tests fail 0） |
| R4: 対象 crate の Rust 検証が健全 | `mise run fmt` `check al-analysis` `lint al-analysis` `test al-analysis` | fmt 差分なし、clippy `-D warnings` pass、全 suite fail 0（`rf628_oracle` 7 tests・長尺 scenario 含む） | Pass（全て exit 0） |
| R5: headless crate に GUI 依存がない | `mise run check-cli-headless` | 10 headless crate に GUI framework 依存なし | Pass（exit 0） |
| R6: toolchain・locked fetch が健全 | `mise run verify-toolchain`、`mise exec -- cargo fetch --locked` | toolchain 一致、locked fetch 成功 | Pass（全て exit 0） |
| R7: 未実行 gate を Pass にしない | 記録レビュー | 実 OS GUI/実 Pixel/macOS/Linux/cross-consumer/`gui-real` は NotRun/Unsupported。upstream CI は実測値をそのまま記録（`consumer-verify`/`daily-ci` success、`ci` failure — 依存木外 crate のみ） | 本 spec と PR 本文に記録 |
| R8: Issue 指定 task 名の存在確認 | `mise tasks ls` / mise.toml 確認 | Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/`verify-ci` は現行 mise.toml に存在しない | 現行 task で代替実行・記録 |

## Examples

- Example（R1/R2）: `Cargo.lock` の ruviform package `source` が
  全て `git+https://github.com/scarecrow173/ruviform.git#
  cbcc83437616fdd7418da37183c6be66b6cfe225` であり、
  `ui/vendor/ruviform/REV` が同 SHA を含む。`Cargo.lock` の差分は
  SHA fragment 26 行のみで、新規 package 追加・version 変動・
  依存 edge 変動はない（in-tree crate の `Cargo.toml` に変更が
  ないため）。
- Example（R4）: `test al-analysis` は `tests/rf628_oracle.rs`
  の 7 tests（fixture/oracle 一致・f64 fidelity・長尺非回帰・
  gap/Validity）を含む全 suite が Pass。`rf628_long_scenario_
  non_regression`（30s contiguous scenario）は実走で約 132 秒。
- Example（upstream CI）: `gh run list --commit cbcc8343...` で
  `consumer-verify` run 43783883894（success、verify-audiolens
  含む全 job success）・`daily-ci` run 43783844680（success、
  dispatcher）・`ci` run 43783872777（failure:
  `verify (ubuntu-latest)`/`l1-real-verify (ubuntu-latest)` は
  `ruviform-plugin-test-vst3` の clippy 失敗、`verify-macos`/
  `rt-verify` は `ruviform-midi-device` test compile 失敗、共に
  AL 依存木外。windows lane は success）を確認。

## Guarantees

- pin 追従は manifest を変更せず lockfile のみで行われ、
  `check-ruviform-lock`/`check-ruviform-rev` が単一 SHA・vendor
  一致を fail-closed に保証する。
- `ui/vendor/ruviform` は `sync-ui-deps` の生成物であり、
  `REV` が lockfile SHA と一致する。本範囲の ruviform 差分に
  `ui/*` TS source の変更はなく、vendor 内容は REV 以外不変。
- 全検証は正規 mise task 経由で実行され、cargo 直実行は補助証拠
  としてのみ扱う。
- 実機・実 Pixel・macOS/Linux・cross-consumer・`gui-real` は
  NotRun/Unsupported として記録され、semantic/headless の Pass を
  実機 Pass に読み替えない。upstream CI の failure もそのまま
  記録し、依存木内/外の切り分けを明記する。

## Non-goals

- 実 OS GUI・実 Pixel golden・IME・DPI の検証（環境到達不能、
  NotRun）。
- macOS/Linux での lane 実行（Unsupported: 実行環境は Windows のみ）。
- cross-consumer（Synvelis 側）の非回帰確認（Coordinator 側責務。
  なお upstream `consumer-verify` run 43783883894 は
  verify-synvelis/verify-audiolens 共に success であり、
  ruviform#294 系の upstream 側証拠として記録する）。
- ruviform 新 API/feature（record codec・plugin test_instrument・
  audio-device endpoint probe・Test Lab GUI・DComp pixel
  regression）の AL 採用（本 slice では採用しない判断を記録する
  のみ）。
- ruviform 側 manual/opt-in lane・upstream `ci` failure の修正
  （依存木外 crate の問題は `scarecrow173/ruviform#625` で追跡）。
- ruviform 側の新規 Issue 起票（既存 `scarecrow173/ruviform#625`
  への情報追加で対応）。

## Risk

- upstream `ci` workflow が `cbcc8343` で failure。失敗は全て
  AL 依存木外 crate（`ruviform-plugin-test-vst3` clippy、
  `ruviform-midi-device` test compile）だが、upstream main の
  red 状態は `scarecrow173/ruviform#625` で追跡する。
- `ruviform-plugin-core` の record codec は additive 公開 API で
  AL の消費経路なし。`plugin-host` callsite 更新と
  `audio-test` 内部リファクタは公開契約不変。
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
| lint-full / test-full / build-full / check-ui-gpui / lint-ui-gpui / test-ui-gpui / build-ui-gpui / check-ui-webview / lint-ui-webview / test-ui-webview / check-ci-workflow / check-ci-tracker-workflows | NotRun（本 slice の正規検証コマンド対象外。変更は pin 追従 + 文書のみ） |
| 実 OS GUI / 実 Pixel / macOS / Linux / cross-consumer / gui-real | NotRun / Unsupported |
| upstream CI（`cbcc8343` 対象） | `consumer-verify` Pass（run 43783883894）・`daily-ci` Pass（run 43783844680）・`ci` **Fail**（run 43783872777、依存木外 crate のみ）— 実測値を記録 |

## 採用/保留判断と rollback

- 判断: **採用**（pin 追従として）。全正規検証 lane が対象
  revision で Pass し、破壊的 API/Feature/OS 変更・AL 側の恒久
  workaround 要求は検出されなかった。upstream `ci` の failure は
  依存木外 crate に限定され、`consumer-verify`（verify-audiolens
  含む）は success。upstream 新 API/feature（record codec・
  plugin test_instrument・audio-device endpoint probe・Test Lab
  GUI・DComp pixel regression）は opt-in/test-only/依存木外で
  あり、AL 側の採用 gate・需要が未定義のため本 slice では
  **採用しない**。実機系 gate は NotRun のまま別途追跡する
  （semantic/headless の Pass を実機 acceptance と混同しない）。
- rollback: `Cargo.lock` を `eafe0a08b71bd9a74566ae6678c3e0683fce1421`
  へ戻す revert + `mise run sync-ui-deps` 再実行で旧 pin 状態に
  復帰可能（製品コード差分がないため影響は lockfile と vendor
  生成物に限定）。
