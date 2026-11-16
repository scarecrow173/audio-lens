---
id: "IMPL-034YnH9gny1R4SISLshw3K"
type: "impl"
status: "completed"
title: "ruviform cbcc8343 pin 追従評価の実施記録"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-cbcc8343-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-cbcc8343-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-cbcc8343-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-cbcc8343-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-cbcc8343-rev-watch.md"
      - type: "code"
        path: "evidence/al127/cbcc8343-verification.md"
    modified:
      - type: "code"
        path: "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "code"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/tasks/ruviform-cbcc8343-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-eafe0a08-rev-watch.md"
    - "docs/impl/ir/ruviform-ac10a2f7-rev-watch.md"
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

# ruviform cbcc8343 pin 追従評価の実施記録

## Summary

ruviform pin を `eafe0a08b71bd9a74566ae6678c3e0683fce1421` から
`cbcc83437616fdd7418da37183c6be66b6cfe225`（ruviform main HEAD）
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を全実行した。`Cargo.lock` の差分は ruviform 26 package の
SHA fragment 更新のみで、version 変動・新規 package・依存 edge
変動はなかった（in-tree crate の `Cargo.toml` に変更がないため）。
`ui/vendor/ruviform` は `sync-ui-deps` で再生成（`ui/*` TS
source に差分がないため REV 以外不変）。製品コード変更はなく、
全 lane Pass を確認。upstream 新 API/feature（plugin-core record
codec・plugin test_instrument fixture・audio-device endpoint
probe・Test Lab GUI・DComp pixel regression）は採用しない判断を
記録。upstream main CI は `consumer-verify`/`daily-ci` success、
`ci` failure（依存木外 crate のみ）を実測記録した。

## Context

Issue #127（Ruviform 更新の常設追跡 Issue）の rev-watch 定型
slice。前段は PR #216（pin `eafe0a08` 追従評価）。本 slice の
対象範囲は pin `eafe0a08` → `cbcc8343`（6 commit・97 file、
crates/ 配下は 9 crate）。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  cbcc83437616fdd7418da37183c6be66b6cfe225` で ruviform 26
  package の `source` fragment を一括更新。差分は 26 行の SHA
  fragment のみ（他 crate の version 変動・新規 package 追加・
  依存 edge 変動なし。in-tree crate の `Cargo.toml` に変更が
  ないため）。
- `ui/vendor/ruviform`: `mise run sync-ui-deps` で lockfile SHA
  から再生成。`REV` = `cbcc83437616fdd7418da37183c6be66b6cfe225`。
  ruviform 差分に `ui/*` の変更がないため vendor 内容は REV
  以外前 pin と同一。
- docs: spec→design→test-spec→task→IR の文書チェーンを
  `ruviform-cbcc8343-rev-watch` 名で新規作成。本 slice の文書
  セットは lease どおり 5 点で plans 文書は含めない。
  **docs index（`docs/{specs,designs,tasks,test-specs,impl/ir}/
  README.md`）は本 slice では再生成していない** — Worker の
  編集範囲外であり、統合時に Coordinator が再生成する運用の
  ため（PR 本文「未対応の作業」に記録）。
- `evidence/al127/cbcc8343-verification.md` に検証証拠を記録。

### Decisions During Implementation

- pin 更新は完全 SHA 指定（短縮 SHA の誤展開を避ける前段の教訓に
  従う）。`git ls-remote` で refs/heads/main =
  `cbcc83437616fdd7418da37183c6be66b6cfe225` を確認し、local
  clone `G:/repositry/audio-products/ruviform` の `origin/main`
  HEAD と一致を確認して確定（Spec 起稿時点と同一 SHA、再解決の
  結果 upstream は進んでいなかった）。
- `eafe0a08..cbcc8343` の影響評価: AL の依存木（lockfile の
  26 package）に変更があるのは `de3ed4e8`
  （`scarecrow173/ruviform#630` `#676` test_instrument）由来の
  3 crate のみ。`ruviform-plugin-core` の `wire.rs` は shm slot
  用固定長 record codec `encode_event_record`/`decode_event_record`
  の additive 追加（既存 `encode_event`/`decode_event`/定数は
  不変）、`ruviform-plugin-host` の `l1/data.rs` は内部 callsite
  の 1 行更新（`encode_event` → `encode_event_record`）、
  `ruviform-audio-test` の `backend/stream.rs` は非公開 fn
  `deliver_input_errors` の抽出リファクタ（`pub(crate)`、公開
  契約不変）。in-tree crate の `Cargo.toml` 変更なし。
  依存木外では `ruviform-audio-device`（`scarecrow173/ruviform#629`
  `#673`: WASAPI streaming verifier + endpoint_probe 拡張 +
  virtual_endpoint test +1,371）、`ruviform-plugin-clap`/
  `plugin-export`/`plugin-dock-test`/`plugin-test-clap`/
  `plugin-test-vst3`（RF#630 test_instrument `#676`、RF#631 Test
  Lab GUI `#674`、RF#532 DComp pixel regression `#677`）が変更。
  破壊的変更なし・adapter 修正不要。
- 新 API/feature の採用判断: record codec・plugin test_instrument
  fixture・audio-device endpoint probe・Test Lab GUI・DComp pixel
  regression はいずれも opt-in/test-only/依存木外であり AL 側の
  採用 gate・需要が未定義のため見送り。record codec は AL が
  plugin wire codec を直接消費しないため現状不要。
- upstream main CI 証拠: candidate `cbcc8343` に対する covering
  run が存在する（前 slice の `eafe0a08` とは異なり取得済み）。
  `consumer-verify` run 43783883894 = success（verify-synvelis/
  verify-audiolens/summary 全 job success）、`daily-ci` run
  43783844680 = success（dispatcher のみ）、`ci` run 43783872777
  = **failure**。`ci` の失敗は `verify (ubuntu-latest)`/
  `l1-real-verify (ubuntu-latest)` の `ruviform-plugin-test-vst3`
  clippy `-D warnings`（unnecessary `u32 as u32` cast ×5、
  `de3ed4e8` の instrument/events 系コードに起因する可能性）と
  `verify-macos`/`rt-verify` の `ruviform-midi-device` test
  `output_ports` compile error（`MidiGroup`/`MidiChannel`/
  `NoteKey`/`MidiValue`/`MidiKind` 未解決、当該 crate は本差分で
  未変更 — `bff3603b` 以降の既存問題の可能性）。いずれも AL
  lockfile 26 package に非含有（依存木外）であり AL 側の採用
  判断・検証結果に影響しない。`scarecrow173/ruviform#625`
  （CI failure tracker、Open）へ状況コメントを追加した。
- Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/
  `verify-ci` task 名は現行 mise.toml に存在しないため、Spec の
  正規検証コマンドに列挙された現行 task で実行した。
- 本 worktree は `ui/node_modules`・`ui/vendor` 未復元のため、
  UI lane の前に `mise run setup-ui`（`npm ci --prefix ui`）を
  実行し、`mise run sync-ui-deps` で vendor を再生成した。
- doc graph route は main 既存の broken relation
  （`crates/al-math` 参照切れ）で blocked のため、前段 PR
  #190/#193/#194/#213/#214/#215/#216 と同じく文書チェーンを直接
  作成し review-protocol で承認相当のレビューを行う運用とした。
- ブランチ名は Spec 指定の `feat/127-ruviform-cbcc8343` を使用
  （Orca 既定の `scarecrow173/al127-cbcc8343` ではなく、前回
  gate の PR head 命名相違の再発を避けるため明示的に作成・
  push する）。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run check-ruviform-lock`: Pass（26 package、単一 SHA
  `cbcc8343...`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check al-analysis` / `lint al-analysis` /
  `test al-analysis`: Pass（`rf628_oracle` 7 tests・長尺
  scenario `rf628_long_scenario_non_regression` 約 132 秒実走を
  含む全 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし）
- `mise run verify-toolchain`: Pass（rustc/cargo 1.99.1）
- `mise exec -- cargo fetch --locked`: Pass
- 補助証拠: `mise run check-full` Pass（workspace check）
- NotRun/Unsupported: 実 OS GUI・実 Pixel・macOS/Linux・
  cross-consumer・`gui-real`・ruviform manual lane・`*-full` の
  lint/test/build・`check-ui-gpui`/`check-ui-webview` 各 lane・
  `check-ci-workflow`/`check-ci-tracker-workflows`（正規検証
  コマンド対象外）。upstream CI は実測記録（`consumer-verify`/
  `daily-ci` Pass、`ci` Fail=依存木外 crate のみ）。
  詳細は `docs/test-specs/ruviform-cbcc8343-rev-watch.md` を
  正本とする。

## Risks

- upstream `ci` の `cbcc8343` failure（依存木外 crate の
  lint/compile 失敗）は `scarecrow173/ruviform#625` で追跡。
  将来これらの crate が依存木に入った場合に顕在化し得る。
- upstream の新機能領域（plugin test_instrument・endpoint probe・
  Test Lab GUI・DComp pixel regression）は AL lane 未踏。
  candidate tuple としての継続評価で担保し、Issue #127 は
  open のまま。
- mbx cache の absolute-path mapping / build-script UTF-8 warning
  （Git checkout 由来、記録のみ）。

## Follow-ups

- record codec・plugin test_instrument・audio-device endpoint
  probe・Test Lab GUI・DComp pixel regression の AL 採用判断は
  別 slice の Issue で行う（本 slice では採用しない）。
- 実機 lane（実 OS GUI・実 Pixel・macOS/Linux）と upstream `ci`
  failure の解消は既存の NotRun/Issue 追跡に従う
  （`scarecrow173/ruviform#625`）。
