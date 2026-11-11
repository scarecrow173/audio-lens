---
id: "SPEC-034VFSKyms68sXggutXJmw"
type: "spec"
status: "approved"
title: "ruviform pin fd1d4d5f 追従評価（AL#127）"
created: "2026-11-11"
updated: "2026-11-11"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/253"
    - "https://github.com/scarecrow173/ruviform/issues/313"
    - "https://github.com/scarecrow173/ruviform/issues/314"
    - "https://github.com/scarecrow173/ruviform/issues/251"
    - "https://github.com/scarecrow173/ruviform/issues/243"
    - "https://github.com/scarecrow173/ruviform/issues/565"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-fd1d4d5f-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-fd1d4d5f-rev-watch.md"
      - type: "file"
        path: "docs/plans/ruviform-fd1d4d5f-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-fd1d4d5f-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-fd1d4d5f-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-fd1d4d5f-rev-watch.md"
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
    - "docs/specs/ruviform-b02942c2-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-fd1d4d5f-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin fd1d4d5f 追従評価（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を `b02942c2f9e6e195d0d752803b077866468fe58e`
から候補 `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38`（ruviform main
HEAD）へ、lockfile-primary 手順
（`tools/update-ruviform-rev.mjs` → `mise run sync-ui-deps`）で追従する。
`Cargo.lock` の 20 ruviform package が単一 SHA に解決され、
`ui/vendor/ruviform/REV` が同一 SHA を指すことを確認する。

`b02942c2..fd1d4d5f` の 7 commit（ruviform PR #564 plugin L1 wire
report verb・#567 WASAPI-native input frame_position・#566 GPUI
lane の CI 収載 + Linux offscreen golden・#568 Stage W 実機
closeout + WASAPI callback timestamp 修正・#569
RealOsPointerOrKeyboard capability + Windows 実入力 E2E・#570
multiwindow verdict 再実走 closeout・#571 bridge unsupported
計数透過 + opt-in 実機 lane scaffold）の差分を調査し、AudioLens
への影響を軸別（f64 精度 / CLI / GUI / scenario / feature）に
記録する。新 API（RealOsPointerOrKeyboard / os_input preflight
session cache / Linux offscreen golden / L1 wire report verb /
WASAPI-native input frame_position / bridge unsupported 透過
等）の採用判断は本 slice では「採用しない」と明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
候補 `fd1d4d5f` は `b02942c2..fd1d4d5f` で 7 commit・110 file
（大半は ruviform 側の docs/evidence）・+12,674/−301 行の差分を
含み、AL 依存木内では `ruviform-ui-gpui`（OS-window pixel
capture の X11/Windows backend・`OsInputOutcome` additive・
`real-scenario` 配下の OS input ops・optional dep `x11rb` 追加、
RF#566/#569）と `ruviform-audio-core`（`midi_input` mock の
`inject_unsupported` additive、RF#253 residual）に変更がある。
依存木外では `ruviform-audio-device`（WASAPI-native input
path の `frame_position` 供給と callback timestamp 修正、
RF#565/#251）、`ruviform-plugin-*`（L1 wire report verb、
RF#253）、`ruviform-audio-plugin-bridge`（unsupported 計数透過、
RF#253 residual）、`ruviform-midi-device`（test のみ）が変更
されている。AudioLens が採用済みの GPUI driver 経路に隣接する
変更を含むため、candidate tuple として評価・記録する必要がある。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の real-scenario / OS input 採用判断: upstream の opt-in
  feature・capability・証拠 lane の存在を記録し、採用/保留の
  判断材料にする。

## Scope

### In Scope

- ruviform pin `b02942c2` → `fd1d4d5f` の lockfile-primary 追従
  （`Cargo.lock` 20 package の単一 SHA 解決、`ui/vendor/ruviform`
  の再生成と `REV` 一致）。
- `b02942c2..fd1d4d5f` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- 正規 mise 検証 lane（fmt/check/lint/test/build、CLI headless、
  UI、GPUI、WebView、CI profile）の対象 revision での実行と結果記録。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`・`al-gui/`・`ui/src/`）の機能変更。pin
  追従でコンパイル/テストが壊れた場合の adapter 修正は行わず、
  API 差分を PR 本文に記録する（今回は不要だった）。
- ruviform の新 API（`real-scenario` 配下の OS input ops・
  `OsInputOutcome`・SendInput preflight session cache・Linux
  offscreen golden・L1 wire report verb・WASAPI-native input・
  bridge unsupported 透過）の採用。
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
- 採用/保留判断と rollback 情報（旧 SHA `b02942c2` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指し、
  `Cargo.lock` の差分が ruviform 系 SHA fragment 更新＋推移的依存の
  必要更新（`ruviform-ui-gpui` の optional dep `x11rb` 追加 1 行）
  に留まる。
- [x] `b02942c2..fd1d4d5f` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API の採用判断（今回は採用しない）を明示する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] PR 本文に resolved SHA・candidate SHA・影響表・採用 PR リンク
  を記録する（Issue #127 への comment は Coordinator が確認するため
  PR 本文で代行。Issue は Close しない）。

## Deferred Design Concerns

- `real-scenario` feature / real-window adapter・OS input ops
  （RealOsPointerOrKeyboard）の AL 採用 | upstream は opt-in で
  実 OS window・pixel capture・OS 入力注入を提供するが、AL の
  独立精度 gate・採用 slice が未定義 | 採用判断 slice の spec
  起票時 | 実機相当の証拠なしに semantic/headless pass を実機
  acceptance と誤認するリスク | design doc の影響評価と PR 本文
  に記録
- WASAPI-native input frame_position / L1 wire report verb /
  bridge unsupported 透過の AL 採用 | AL は audio-device・
  plugin host・MIDI bridge を依存木に持たず需要未定 | 各機能の
  採用 Issue 起票時 | upstream 機能の存在を失念して重複実装する
  リスク | design doc の影響評価に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証）、ruviform#253（plugin report /
  MIDI 系 residual の upstream Issue）、ruviform#313（real OS
  input capability）、ruviform#314（GPUI lane / real-scenario）、
  ruviform#251（audio-device Stage W）、ruviform#243
  （multiwindow）、ruviform#565（WASAPI input frame_position）
- 前段採用: PR #193（pin `b02942c2` 追従評価）、PR #190（pin
  `271e2d41` 追従評価 + Legacy WebView 棚卸し）
