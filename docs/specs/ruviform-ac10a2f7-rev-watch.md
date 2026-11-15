---
id: "SPEC-034XLU7bVQnduz0Pr4H8aI"
type: "spec"
status: "approved"
title: "ruviform pin ac10a2f7 追従評価（AL#127）"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/631"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/629"
    - "https://github.com/scarecrow173/ruviform/issues/646"
    - "https://github.com/scarecrow173/ruviform/issues/306"
    - "https://github.com/scarecrow173/ruviform/issues/389"
    - "https://github.com/scarecrow173/ruviform/issues/650"
    - "https://github.com/scarecrow173/ruviform/issues/192"
    - "https://github.com/scarecrow173/ruviform/issues/193"
    - "https://github.com/scarecrow173/ruviform/issues/275"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-ac10a2f7-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-ac10a2f7-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-ac10a2f7-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-ac10a2f7-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-ac10a2f7-rev-watch.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/ruviform-c6e4f334-rev-watch.md"
    - "docs/specs/ruviform-fd1d4d5f-rev-watch.md"
    - "docs/specs/ruviform-b02942c2-rev-watch.md"
    - "docs/specs/ruviform-271e2d41-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-ac10a2f7-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin ac10a2f7 追従評価（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を
`c6e4f334242567dbd0e0d8b7713d9d480a6aab66` から候補
`ac10a2f7761c08db73aebd861c0a5c1f53f269b7`（ruviform main HEAD）へ、
lockfile-primary 手順（`tools/update-ruviform-rev.mjs` →
`mise run sync-ui-deps`）で追従する。`Cargo.lock` の 20 ruviform
package が単一 SHA に解決され、`ui/vendor/ruviform/REV` が同一 SHA を
指すことを確認する。

`c6e4f334..ac10a2f7` の 14 commit・148 file・+22,339/−223 行の
差分（大半は ruviform 側 docs 87 file・evidence 18 file で、
`crates/` 配下は 40 file・+9,049/−184 行）を調査し、AudioLens への
影響を軸別（f64 精度 / CLI / GUI / scenario / feature / IO 契約）に
記録する。新規 crate `ruviform-plugin-dock-test`（Issue #631、
test-only plugin-dock scenario harness）・`ruviform-plugin-test-vst3`
（Issue #630 first slice、VST3 `test_gain` fixture）・
`ruviform-ui-test` の plugin_dock scenario / `PluginHostProbe`
driver（Issue #631）・`ruviform-{audio,midi}-device` の endpoint
probe examples（Issue #629）・`ruviform-io-e2e` の G01 実 4GiB
opt-in + G05 性能 baseline lane（Issue #306）・`ruviform-numeric`
の test-only 修正（Issue #646）等の採用判断は本 slice では
「採用しない」と明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
候補 `ac10a2f7` は `c6e4f334..ac10a2f7` で 14 commit・148 file の
差分を含み、AL 依存木内（lockfile 20 package）では
`ruviform-numeric`（test のみの型注記 + test allocator への
`#[cfg(debug_assertions)]` 付与、製品コード無変更）と
`ruviform-ui-test`（additive: `scenario/plugin_dock.rs` 新規 module・
`ScenarioDriver::PluginHostProbe` variant 追加・`scenario_contract`
test 追加）のみが変更されている。依存木外では新規
`ruviform-plugin-dock-test`・`ruviform-plugin-test-vst3`、
`ruviform-audio-device`/`ruviform-midi-device` の Windows 仮想
Endpoint probe、`ruviform-plugin-core`/`ruviform-audio-plugin-bridge`
の V9+V11+V12 MIDI event contract 修正（Issue #253）、
`ruviform-io-e2e` の opt-in lane 追加（Issue #306）、
`ruviform-audio-test` の metadata 拡張が変更されている。前 slice
（c6e4f334）が記録した `ruviform-audio-test`（Issue #628 test
runtime）の plugin-dock consumer 側実装が今回着地しており、AL#127
の GUI-L2/EXT-L3 要件と接続する upstream 機能の存在を記録する
必要がある。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の plugin-dock scenario / VST3 test fixture / endpoint probe /
  io-e2e lane 採用判断: upstream の新 crate・feature・capability の
  存在を記録し、採用/保留の判断材料にする。

## Scope

### In Scope

- ruviform pin `c6e4f334` → `ac10a2f7` の lockfile-primary 追従
  （`Cargo.lock` 20 package の単一 SHA 解決、`ui/vendor/ruviform`
  の再生成と `REV` 一致）。
- `c6e4f334..ac10a2f7` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- 正規 mise 検証 lane（fmt/check/lint/test/build、CLI headless、
  UI、GPUI、WebView、CI profile）の対象 revision での実行と結果記録。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`・`al-gui/`・`ui/src/`）の機能変更。pin
  追従でコンパイル/テストが壊れた場合の adapter 修正は行わず、
  API 差分を PR 本文に記録する（今回は不要だった）。
- ruviform の新 API・新 feature（`ruviform-plugin-dock-test`、
  `ruviform-plugin-test-vst3`、`ruviform-ui-test` の
  `plugin_dock` scenario / `PluginHostProbe` driver、
  `ruviform-{audio,midi}-device` endpoint probe examples、
  `ruviform-io-e2e` opt-in lanes、`ruviform-plugin-core` の
  V9+V11+V12 MIDI event contract）の採用。
- 実 OS GUI・実 Pixel・macOS/Linux・cross-consumer gate の実施
  （環境到達不能なものは NotRun/Unsupported として記録）。
- ruviform・synvelis リポジトリの変更。
- Issue #127 の Close。

## Requirements

- manifest の恒久固定 rev へ戻さず、lockfile の `source` fragment が
  単一 resolved SHA の正本であり続けること。
- `cargo fetch --locked`・`mise run check-ruviform-lock`・
  `mise run check-ruviform-rev` が欠落・分裂・不一致を fail-closed
  に検出すること。
- `mise run sync-ui-deps` → `check-ui` → `test-ui` が対象 SHA で
  Pass し、`Cargo.lock` と `ui/vendor/ruviform/REV` が一致すること。
- 検証不能事項・失敗ケース・許容差を結果へ明記し、未実行 gate を
  Pass に繰り上げないこと。
- 採用/保留判断と rollback 情報（旧 SHA `c6e4f334` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `ac10a2f7761c08db73aebd861c0a5c1f53f269b7` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指し、
  `Cargo.lock` の差分が ruviform 系 SHA fragment 更新のみ（20 行、
  推移的依存の変動なし）に留まる。
- [x] `c6e4f334..ac10a2f7` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API/feature の採用判断（今回は採用しない）を
  明示する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] PR 本文に resolved SHA・candidate SHA・影響表・採用 PR リンク
  を記録し、Issue #127 に candidate tuple 評価 comment を投稿する
  （Issue は Close しない）。

## Deferred Design Concerns

- `ruviform-plugin-dock-test`（test-only plugin-dock scenario
  harness）・`ruviform-plugin-test-vst3`（VST3 `test_gain`
  fixture）・`ruviform-ui-test` の `plugin_dock` scenario /
  `PluginHostProbe` driver の AL 採用 | AL#127 の Windows GUI-L2
  （実 Window/入力/Focus/画素）・EXT-L3（実 Plugin ABI）要件が
  RF#631 harness と RF#630 fixture の利用を想定するが、AL 側の
  採用 slice・独立 Oracle 接続設計が未定義 | 採用判断 slice の
  spec 起票時 | upstream fixture の存在を失念して重複実装する
  リスク | design doc の影響評価と PR 本文に記録
- `ruviform-{audio,midi}-device` の Windows 仮想 Endpoint probe
  examples（Issue #629）の AL 採用 | AL は本番 OS Audio/MIDI API
  を製品機能として提供しておらず需要未定。probe は example
  binary であり library API ではない | 採用 Issue 起票時 |
  upstream probe 実装の重複リスク | design doc の影響評価に記録
- `ruviform-plugin-core` / `ruviform-audio-plugin-bridge` の
  V9+V11+V12 MIDI event contract 修正（Issue #253）の AL 採用 |
  AL は Plugin Host を提供せず依存木外。contract 変更が将来の
  plugin 系採用時に影響する可能性 | plugin 系採用 Issue 起票時 |
  contract 差分の見落としリスク | design doc の影響評価に記録
- `ruviform-io-e2e` の G01 実 4GiB opt-in lane・G05 性能
  baseline lane（Issue #306）の AL 採用 | AL の長尺 Dataset・
  性能 Oracle との接続が未評価 | 採用 Issue 起票時 | upstream
  lane の存在を失念するリスク | design doc の影響評価に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証）、ruviform#628（deterministic
  test runtime）、ruviform#631（plugin-dock scenario harness
  first slice `#647` + session-lifecycle cases `#652`）、
  ruviform#630（VST3 test_gain fixture `#641`）、ruviform#253
  （V9+V11+V12 MIDI event contract `#644`）、ruviform#629
  （Windows 仮想 Endpoint probe `#640`）、ruviform#646（numeric
  bench profile 修正 `#649`）、ruviform#306（io-e2e G01/G05 lane
  `#645`）、ruviform#650（bench-smoke lane 拡張 `#651`）、
  ruviform#389（verification-status reconcile `#652`）、
  ruviform#192/#193/#275（Virtual-first Owner remap docs
  `#653`/`#654`/`#655`）
- 前段採用: PR #213（pin `c6e4f334` 追従評価）、PR #194（pin
  `fd1d4d5f` 追従評価）、PR #193（pin `b02942c2` 追従評価）、
  PR #190（pin `271e2d41` 追従評価）
