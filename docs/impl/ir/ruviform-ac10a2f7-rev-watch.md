---
id: "IMPL-034XLUtOnusrJZxCBd0Q6x"
type: "impl"
status: "completed"
title: "ruviform ac10a2f7 pin 追従評価の実施記録"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-ac10a2f7-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-ac10a2f7-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-ac10a2f7-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-ac10a2f7-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-ac10a2f7-rev-watch.md"
    modified:
      - type: "code"
        path: "Cargo.lock"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "code"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/tasks/ruviform-ac10a2f7-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-c6e4f334-rev-watch.md"
    - "docs/impl/ir/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/impl/ir/ruviform-b02942c2-rev-watch.md"
    - "docs/impl/ir/ruviform-pin-refresh-073a3ff-consumer-regression-issue-127.md"
    - "docs/impl/ir/ruviform-rev-bump-consumer-regression-verification-issue-127.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# ruviform ac10a2f7 pin 追従評価の実施記録

## Summary

ruviform pin を `c6e4f334242567dbd0e0d8b7713d9d480a6aab66` から
`ac10a2f7761c08db73aebd861c0a5c1f53f269b7`（ruviform main HEAD）
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を全実行した。`Cargo.lock` の差分は ruviform 20 package の
SHA fragment 更新 20 行のみで、推移的依存の変動・新規 package
追加はなかった。`ui/vendor/ruviform` は `sync-ui-deps` で再生成
（`ui/*` TS source に差分がないため REV 以外不変）。製品コード
変更はなく、全 lane Pass を確認。upstream 新 API/feature
（`ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`・
ui-test plugin_dock scenario/`PluginHostProbe`・endpoint probe
examples・io-e2e opt-in lanes・plugin-core MIDI event contract）は
採用しない判断を記録。

## Context

Issue #127（Ruviform 更新の常設追跡 Issue）の rev-watch 定型
slice。前段は PR #213（pin `c6e4f334` 追従評価）。本 slice の
対象範囲は pin `c6e4f334` → `ac10a2f7`（14 commit・148 file、
`crates/` 配下は 40 file・+9,049/−184 行、大半は docs 87 file・
evidence 18 file）。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  ac10a2f7761c08db73aebd861c0a5c1f53f269b7` で ruviform 20
  package の `source` fragment を一括更新。差分は 20 行の SHA
  fragment のみ（他 crate の version 変動・新規 package 追加・
  推移的依存の変動なし）。
- `ui/vendor/ruviform`: `mise run sync-ui-deps` で lockfile SHA
  から再生成。`REV` = `ac10a2f7761c08db73aebd861c0a5c1f53f269b7`。
  ruviform 差分に `ui/*`（core/components/binding/viz）の変更が
  ないため vendor 内容は REV 以外前 pin と同一。
- docs: spec→design→test-spec→task→IR の文書チェーンを
  `ruviform-ac10a2f7-rev-watch` 名で新規作成し、生成 index
  （docs/{specs,designs,tasks,test-specs}/README.md）を再生成、
  `docs/impl/ir/README.md` は impl-doc safe sync で更新。
  本 slice の文書セットは lease どおり 5 点で plans 文書は
  含めない。

### Decisions During Implementation

- pin 更新は完全 SHA 指定（短縮 SHA の誤展開を避ける前段の教訓に
  従う）。`git ls-remote` で refs/heads/main =
  `ac10a2f7761c08db73aebd861c0a5c1f53f269b7` を確認し、local
  clone `G:/repositry/audio-products/ruviform` の `origin/main`
  HEAD と一致を確認して確定（Spec 起稿時点と同一 SHA、再解決の
  結果 upstream は進んでいなかった）。
- `c6e4f334..ac10a2f7` の影響評価: AL の依存木（lockfile の
  20 package）に変更があるのは `ruviform-numeric`（`mod tests`
  内の型注記 + test allocator への `#[cfg(debug_assertions)]`
  付与のみ。製品コード無変更。RF#646 `#649`）と
  `ruviform-ui-test`（additive +201/−0: `scenario/plugin_dock.rs`
  新規 module・`ScenarioDriver::PluginHostProbe` variant・
  `scenario_contract` test 追加。`Cargo.toml` 依存追加なし。
  RF#631 `#647`）の 2 crate のみ。依存木外では新規
  `ruviform-plugin-dock-test`（RF#631 `#647` + RF#389 session
  cases `#652`）・`ruviform-plugin-test-vst3`（RF#630 `#641`）、
  `ruviform-audio-device`/`ruviform-midi-device` の endpoint probe
  examples（RF#629 `#640`）、`ruviform-plugin-core`/
  `ruviform-audio-plugin-bridge` の V9+V11+V12 MIDI event contract
  修正（RF#253 `#644`）、`ruviform-io-e2e` の G01 実 4GiB opt-in
  + G05 性能 baseline lane（RF#306 `#645`）、`ruviform-audio-test`
  の `EnvironmentKind` に `RealAbiTestPlugin`/`RealWindow` variant
  追加と metadata 型の crate root re-export（`#647` commit 内、
  plugin-dock-test consumer 向け語彙）が変更。破壊的変更なし・
  adapter 修正不要。
- 新 API/feature の採用判断: `ruviform-plugin-dock-test`・
  `ruviform-plugin-test-vst3`・ui-test plugin_dock scenario/
  `PluginHostProbe`・endpoint probe examples・io-e2e opt-in
  lanes・plugin-core MIDI event contract はすべて
  test-only/opt-in/依存木外であり AL 側の採用 gate・需要が
  未定義のため本 slice では採用しない。
- Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/
  `verify-ci` task 名は現行 mise.toml に存在しないため、
  `check-full`/`lint-full`/`test-full`/`build-full`（workspace
  全体）と個別 feature lane で代替実行した。
- 本 worktree は `ui/node_modules`・`ui/vendor` 未復元のため、
  UI lane の前に `mise run setup-ui`（`npm ci --prefix ui`）を
  実行し、`mise run sync-ui-deps` で vendor を再生成した。
- doc graph route は main 既存の broken relation
  （`crates/al-math` 参照切れ）で blocked のため、前段 PR #190/
  #193/#194/#213 と同じく文書チェーンを直接作成し review-protocol
  で承認相当のレビューを行う運用とした。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run check-ruviform-lock`: Pass（20 package、単一 SHA
  `ac10a2f776...`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check-full` / `lint-full` / `test-full` /
  `build-full`: Pass（workspace、test 58 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` /
  `build-ui-gpui`: Pass（`gpui`/`gpui-test` feature lane。
  `ruviform-ui-gpui@ac10a2f7` が compile/lint/test/build を通過）
- `mise run check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview`: Pass（Legacy WebView lane 維持）
- `mise run check-ci-workflow` / `check-ci-tracker-workflows` /
  `verify-toolchain`: Pass
- `mise exec -- cargo fetch --locked`: Pass
- NotRun/Unsupported: 実 OS GUI・実 Pixel・macOS/Linux・
  cross-consumer・`gui-real`・ruviform `test-ui-plugin-dock`・
  `test-ui-real-scenario`・windowed `uitest` lane（実 desktop
  session 前提の manual lane）。詳細は
  `docs/test-specs/ruviform-ac10a2f7-rev-watch.md` を正本とする。

## Risks

- upstream の新機能領域（plugin-dock scenario harness・VST3 test
  fixture・endpoint probe・io-e2e opt-in lane・plugin MIDI event
  contract）は AL lane 未踏。candidate tuple としての継続評価で
  担保し、Issue #127 は open のまま。
- `proc-macro-error2 v2.0.1` の future-incompatibility warning
  （依存由来、記録のみ）。mbx cache の absolute-path mapping
  warning（Git checkout 由来）も同様に記録のみ。

## Follow-ups

- `ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`・
  ui-test plugin_dock scenario/`PluginHostProbe`・endpoint probe
  examples・io-e2e opt-in lanes・plugin-core MIDI event
  contract の AL 採用判断は別 slice の Issue で行う（本 slice
  では採用しない）。
- 実機 lane（実 OS GUI・実 Pixel・macOS/Linux）の整備は既存の
  NotRun 追跡に従う。
