---
id: "SPEC-034VnTwTGyjylLQ7s4DigL"
type: "spec"
status: "approved"
title: "ruviform pin b02942c2 追従評価（AL#127）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/314"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-b02942c2-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-b02942c2-rev-watch.md"
      - type: "file"
        path: "docs/plans/ruviform-b02942c2-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-b02942c2-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-b02942c2-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-b02942c2-rev-watch.md"
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
    - "docs/specs/ruviform-271e2d41-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-b02942c2-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin b02942c2 追従評価（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を `271e2d4111d3769693445bc5bffca37af9e0650b`
から候補 `b02942c2f9e6e195d0d752803b077866468fe58e`（ruviform main
HEAD）へ、lockfile-primary 手順
（`tools/update-ruviform-rev.mjs` → `mise run sync-ui-deps`）で追従する。
`Cargo.lock` の 20 ruviform package が単一 SHA に解決され、
`ui/vendor/ruviform/REV` が同一 SHA を指すことを確認する。

`271e2d41..b02942c2` の 6 commit（ruviform PR #554 real-window
adapter・#555 VST3 MIDI・#556 SendInput preflight・#557
real-scenario lane・#561 OS MIDI backend・#563 plugin→host
report）の差分を調査し、AudioLens への影響を軸別（f64 精度 /
CLI / GUI / scenario / feature）に記録する。新 API（real-scenario
adapter・SendInput preflight・OS MIDI backend・plugin→host
report 等）の採用判断は本 slice では「採用しない」と明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
候補 `b02942c2` は `271e2d41..b02942c2` で 6 commit・123 file
（大半は ruviform 側の docs/evidence）の差分を含み、
`ruviform-ui-gpui` の real-window `SemanticGuiTest`/`VisualGuiTest`
adapter（opt-in `real-scenario` feature、RF#314 slice 4）、
OS-level input の SendInput preflight（RF#552/#556）、
`ruviform-audio-core` の MIDI input 契約 additive
（`MidiPortOrigin::Unknown`・`MidiReadStats::unsupported`、RF#253
slice a）、VST3 MIDI 正規経路と plugin→host report 配線
（RF#253/#555/#563）を含む。AudioLens が採用済みの Scenario
contract・GPUI driver 経路に隣接する変更を含むため、candidate
tuple として評価・記録する必要がある。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の real-scenario / OS input 採用判断: upstream の opt-in
  feature と証拠 lane の存在を記録し、採用/保留の判断材料にする。

## Scope

### In Scope

- ruviform pin `271e2d41` → `b02942c2` の lockfile-primary 追従
  （`Cargo.lock` 20 package の単一 SHA 解決、`ui/vendor/ruviform`
  の再生成と `REV` 一致）。
- `271e2d41..b02942c2` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- 正規 mise 検証 lane（fmt/check/lint/test/build、CLI headless、
  UI、GPUI、WebView、CI profile）の対象 revision での実行と結果記録。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`・`al-gui/`・`ui/src/`）の機能変更。pin
  追従でコンパイル/テストが壊れた場合の adapter 修正は行わず、
  API 差分を PR 本文に記録する（今回は不要だった）。
- ruviform の新 API（`real-scenario` feature・SendInput
  preflight・OS MIDI backend・plugin→host report）の採用。
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
- 採用/保留判断と rollback 情報（旧 SHA `271e2d41` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [ ] ruviform pin が `b02942c2f9e6e195d0d752803b077866468fe58e` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [ ] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指し、
  `Cargo.lock` の差分が ruviform 系 SHA fragment 更新＋推移的依存の
  必要更新に留まる。
- [ ] `271e2d41..b02942c2` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API の採用判断（今回は採用しない）を明示する。
- [ ] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [ ] 採用/保留判断と rollback 情報を記録する。
- [ ] PR 本文に resolved SHA・candidate SHA・影響表・採用 PR リンク
  を記録する（Issue #127 への comment は Coordinator が確認するため
  PR 本文で代行。Issue は Close しない）。

## Deferred Design Concerns

- `real-scenario` feature / real-window adapter の AL 採用 |
  upstream は opt-in で実 OS window・pixel capture を提供するが、
  AL の独立精度 gate・採用 slice が未定義 | 採用判断 slice の
  spec 起票時 | 実機相当の証拠なしに semantic/headless pass を
  実機 acceptance と誤認するリスク | design doc の影響評価と
  PR 本文に記録
- SendInput preflight / OS MIDI backend / plugin→host report の
  AL 採用 | AL は MIDI・plugin host を依存木に持たず、OS input
  注入の製品利用も未定 | 各機能の採用 Issue 起票時 |
  upstream 機能の存在を失念して重複実装するリスク | design doc
  の影響評価に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証）、ruviform#253（plugin→host
  report / MIDI 系 slice の upstream Issue）、ruviform#314
  （real-scenario lane の upstream Issue）
- 前段採用: PR #190（pin `271e2d41` 追従評価 + Legacy WebView
  棚卸し）、PR #188（pin `84d21641`、SemanticGuiTest scenario 採用）
