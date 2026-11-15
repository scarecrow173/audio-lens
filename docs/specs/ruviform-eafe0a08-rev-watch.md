---
id: "SPEC-034YaQQiC29QG0oPeT4iVS"
type: "spec"
status: "approved"
title: "ruviform pin eafe0a08 追従評価（AL#127）"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/625"
    - "https://github.com/scarecrow173/ruviform/issues/628"
    - "https://github.com/scarecrow173/ruviform/issues/630"
    - "https://github.com/scarecrow173/ruviform/issues/631"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-eafe0a08-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-eafe0a08-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-eafe0a08-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-eafe0a08-rev-watch.md"
      - type: "file"
        path: "docs/impl/ir/ruviform-eafe0a08-rev-watch.md"
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
    - "docs/specs/ruviform-ac10a2f7-rev-watch.md"
    - "docs/specs/ruviform-abb9f372-rev-watch.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-eafe0a08-rev-watch.md"
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin eafe0a08 追従評価（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を
`abb9f372d3ee95ee824b3c4f7dc68e8083ce2774` から候補
`eafe0a08b71bd9a74566ae6678c3e0683fce1421`（ruviform main HEAD）へ、
lockfile-primary 手順（`tools/update-ruviform-rev.mjs` →
`mise run sync-ui-deps`）で追従する。`Cargo.lock` の 26 ruviform
package が単一 SHA に解決され、`ui/vendor/ruviform/REV` が同一 SHA を
指すことを確認する。

`abb9f372..eafe0a08` の 6 commit・93 file・+13,204/−317 行の
差分を調査し、AudioLens への影響を軸別に記録する。AL 依存木内で
変更されたのは dev-dependency の `ruviform-audio-test` のみ
（Issue #628 close-out: `RunMetadata`/`RunParameters`/enum への
serde derive と JSON 永続化 API、`StreamFault::Cancellation` fault
追加、stream/clock の serde 対応）であり、いずれも additive。
依存木外では `ruviform-plugin-vst3`/`ruviform-plugin-clap` の
adapter state bridge（Issue #630、VST3 `IBStream` + CLAP
`ext.state`）、`ruviform-plugin-test-vst3`/`ruviform-plugin-test-clap`
の test_editor / test_stateful slice（Issue #630）、
`ruviform-plugin-dock-test` の real-editor-attach（Issue #631）等が
変更されている。これらの採用判断は本 slice では「採用しない」と
明示する。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
前 slice（PR #215）は pin `abb9f372` 追従と `ruviform-audio-test`
dev-dep 追加・RF#628 consumer oracle（`crates/al-analysis/tests/
rf628_oracle.rs`）を同時に着地させた。本 slice はその直後の
upstream main 追従であり、`ruviform-audio-test` に新規 API
（RunMetadata JSON 永続化・Cancellation fault）が追加されたため、
採用/見送りの判断記録と standing 回帰（`rf628_oracle` を含む
`al-analysis` の check/lint/test）の対象 revision での再実走が
必要である。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の `RunMetadata` JSON 証拠永続化・Cancellation fault・
  plugin state bridge / real-editor-attach 採用判断: upstream の
  新 API・fixture の存在を記録し、採用/保留の判断材料にする。

## Scope

### In Scope

- ruviform pin `abb9f372` → `eafe0a08` の lockfile-primary 追従
  （`Cargo.lock` 26 package の単一 SHA 解決、
  `ui/vendor/ruviform` の再生成と `REV` 一致）。
- `abb9f372..eafe0a08` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- `ruviform-audio-test` 新 API（RunMetadata JSON・Cancellation
  fault・serde derive）の採用/見送り判断の記録。
- 正規 mise 検証 lane（fmt/check/lint/test、CLI headless、
  UI vendor lane、toolchain）の対象 revision での実行と結果記録。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`）の機能変更。pin 追従でコンパイル/
  テストが壊れた場合の adapter 修正は行わない（今回は不要
  だった）。
- ruviform の新 API・新 feature（RunMetadata JSON・
  `StreamFault::Cancellation`・plugin state bridge・
  test_editor/test_stateful fixture・plugin-dock real-editor
  attach）の採用。
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
- `ruviform-audio-test` の `serde`/`serde_json` 追加が dev-dep
  経路に留まり、CLI/headless 依存木へ波及しないことを
  `check-cli-headless` で確認すること。
- 検証不能事項・失敗ケース・許容差を結果へ明記し、未実行 gate を
  Pass に繰り上げないこと。
- 採用/保留判断と rollback 情報（旧 SHA `abb9f372` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `eafe0a08b71bd9a74566ae6678c3e0683fce1421` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指し、
  `Cargo.lock` の差分が ruviform 系 SHA fragment 更新と
  `ruviform-audio-test` への `serde`/`serde_json` 依存 edge 追加に
  留まる（新規 package 追加・version 変動なし）。
- [x] `abb9f372..eafe0a08` の影響 API/Feature/OS と関連 Issue を
  docs に記録し、新 API/feature の採用判断（今回は採用しない）を
  明示する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] PR 本文に resolved SHA・candidate SHA・影響表・採用判断を
  記録し、Issue #127 に candidate tuple 評価 comment を投稿する
  （Issue は Close しない）。

## Deferred Design Concerns

- `RunMetadata` JSON 永続化（`to_json`/`from_json`/`save_json`/
  `load_json`）の AL 採用 | RF#628 close-out で追加された
  non-RT test-driver API。AL の `rf628_oracle` は現在 run
  metadata を永続化していないが、oracle 証拠の再現性記録を
  強化する余地がある | 採用判断 slice の spec 起票時 |
  upstream API の存在を失念して重複実装するリスク | design doc
  の影響評価と PR 本文に記録
- `StreamFault::Cancellation` の AL 採用 | scripted backend の
  fault 注入は AL の consumer oracle が消費する経路ではない
  （`rf628_oracle` は `PcmFixture` 描画 + file-based 解析を使用）
  | backend 消費 slice の spec 起票時 | fault 語彙の見落とし
  リスク | design doc の影響評価に記録
- `ruviform-plugin-vst3`/`ruviform-plugin-clap` の adapter state
  bridge（VST3 `IBStream` + CLAP `ext.state`）・
  `ruviform-plugin-test-{vst3,clap}` の test_editor/test_stateful
  fixture・`ruviform-plugin-dock-test` の real-editor-attach の
  AL 採用 | AL は Plugin Host を提供せず依存木外。EXT-L3
  （実 Plugin ABI）要件との接続は将来の採用 slice で評価 |
  採用 Issue 起票時 | upstream fixture の存在を失念するリスク |
  design doc の影響評価に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証）、ruviform#625（CI tracker）
- ruviform#628 close-out（`#669`: RunMetadata JSON・Cancellation
  fault・docs 簿記）、ruviform#630（adapter state bridge `#667`、
  test_editor `#668`、test_stateful `#671`）、ruviform#631
  real-editor-attach（`#670`）
- 前段採用: PR #215（pin `abb9f372` 追従 + RF#628 consumer
  oracle）、PR #214（pin `ac10a2f7` 追従評価）、PR #213（pin
  `c6e4f334` 追従評価）
