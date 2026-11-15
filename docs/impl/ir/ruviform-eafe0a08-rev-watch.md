---
id: "IMPL-034YaRbwcLs7BNyNI6lTIY"
type: "impl"
status: "completed"
title: "ruviform eafe0a08 pin 追従評価の実施記録"
created: "2026-11-16"
updated: "2026-11-16"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "code"
        path: "docs/specs/ruviform-eafe0a08-rev-watch.md"
      - type: "code"
        path: "docs/designs/ruviform-eafe0a08-rev-watch.md"
      - type: "code"
        path: "docs/tasks/ruviform-eafe0a08-rev-watch.md"
      - type: "code"
        path: "docs/test-specs/ruviform-eafe0a08-rev-watch.md"
      - type: "code"
        path: "docs/impl/ir/ruviform-eafe0a08-rev-watch.md"
      - type: "code"
        path: "evidence/al127/eafe0a08-verification.md"
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
    - "docs/tasks/ruviform-eafe0a08-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/ruviform-abb9f372-rev-watch.md"
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

# ruviform eafe0a08 pin 追従評価の実施記録

## Summary

ruviform pin を `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774` から
`eafe0a08b71bd9a74566ae6678c3e0683fce1421`（ruviform main HEAD）
へ lockfile-primary 手順で追従し、対象 revision で正規 mise 検証
lane を全実行した。`Cargo.lock` の差分は ruviform 26 package の
SHA fragment 更新と `ruviform-audio-test` の `serde`/`serde_json`
依存 edge 追加のみで、version 変動・新規 package・他 crate への
推移的依存変動はなかった。`ui/vendor/ruviform` は `sync-ui-deps`
で再生成（`ui/*` TS source に差分がないため REV 以外不変）。
製品コード変更はなく、全 lane Pass を確認。upstream 新 API/feature
（RunMetadata JSON・`StreamFault::Cancellation`・plugin state
bridge・test_editor/test_stateful fixture・plugin-dock
real-editor attach）は採用しない判断を記録。

## Context

Issue #127（Ruviform 更新の常設追跡 Issue）の rev-watch 定型
slice。前段は PR #215（pin `abb9f372` 追従 + RF#628 consumer
oracle `crates/al-analysis/tests/rf628_oracle.rs`）。本 slice の
対象範囲は pin `abb9f372` → `eafe0a08`（6 commit・93 file、
crates/ 配下は 38 file・+7,442/−259、大半は docs 45 file・
evidence 8 file）。

## Implementation

### Changes

- `Cargo.lock`: `tools/update-ruviform-rev.mjs
  eafe0a08b71bd9a74566ae6678c3e0683fce1421` で ruviform 26
  package の `source` fragment を一括更新。差分は 26 行の SHA
  fragment + `ruviform-audio-test` の `serde`/`serde_json` edge
  2 行のみ（他 crate の version 変動・新規 package 追加なし。
  両 crate は既に lockfile 内に存在）。
- `ui/vendor/ruviform`: `mise run sync-ui-deps` で lockfile SHA
  から再生成。`REV` = `eafe0a08b71bd9a74566ae6678c3e0683fce1421`。
  ruviform 差分に `ui/*` の変更がないため vendor 内容は REV
  以外前 pin と同一。
- docs: spec→design→test-spec→task→IR の文書チェーンを
  `ruviform-eafe0a08-rev-watch` 名で新規作成し、生成 index
  （docs/{specs,designs,tasks,test-specs,impl/ir}/README.md）を
  再生成。`evidence/al127/eafe0a08-verification.md` に検証証拠を
  記録。本 slice の文書セットは lease どおり 5 点で plans 文書は
  含めない。

### Decisions During Implementation

- pin 更新は完全 SHA 指定（短縮 SHA の誤展開を避ける前段の教訓に
  従う）。`git ls-remote` で refs/heads/main =
  `eafe0a08b71bd9a74566ae6678c3e0683fce1421` を確認し、local
  clone `G:/repositry/audio-products/ruviform` の `origin/main`
  HEAD と一致を確認して確定（Spec 起稿時点と同一 SHA、再解決の
  結果 upstream は進んでいなかった）。
- `abb9f372..eafe0a08` の影響評価: AL の依存木（lockfile の
  26 package）に変更があるのは dev-dependency の
  `ruviform-audio-test` のみ（`#669` close-out + `#672`
  rustfmt）。内容は全て additive: `metadata.rs`/`clock.rs` の
  serde derive + `RunMetadata` JSON 4 API、`#[non_exhaustive]`
  `StreamFault::Cancellation` variant、`backend/stream.rs` の
  `FaultEffects`/`cancelled_pump_*` 内部実装、
  `serde`/`serde_json` 依存追加（理由記録コメント付き）、
  tests 2 file 新規。AL の `rf628_oracle.rs` は `pcm`/`oracle`
  module のみ消費し、変更された `backend`/`metadata`/`clock`
  の公開面は未使用のため compile/test 影響なし。依存木外では
  `ruviform-plugin-vst3`/`ruviform-plugin-clap` の adapter state
  bridge（RF#630 `#667`）、`ruviform-plugin-test-vst3`/
  `test-clap` の test_editor/test_stateful（RF#630 `#668`/`#671`）、
  `ruviform-plugin-dock-test` の real-editor-attach（RF#631
  `#670`）が変更。破壊的変更なし・adapter 修正不要。
- 新 API/feature の採用判断: `RunMetadata` JSON 永続化は
  `rf628_oracle` の証拠強化に使える可能性があるが opt-in で
  現行 oracle は run metadata を永続化していないため見送り。
  `StreamFault::Cancellation` は scripted backend fault であり
  AL の consumer oracle が消費する経路ではないため見送り。
  plugin 系は依存木外かつ AL は Plugin Host を提供しないため
  採用しない。
- upstream main CI 証拠: candidate `eafe0a08` に対する covering
  CI（`ci`/`daily-ci`/`consumer-verify`）は未実行 — `gh run
  list --commit eafe0a08` では schedule の
  `ci-issue-tracker-reconcile`（success）のみ。最新 covering
  run は `ac10a2f7` 対象（2026-11-15）。rev-watch/test-spec に
  NotRun として記録。
- Issue 記載の `check-ci`/`lint-ci`/`test-ci`/`fmt-check`/
  `verify-ci` task 名は現行 mise.toml に存在しないため、Spec の
  正規検証コマンドに列挙された現行 task で実行した。
- 本 worktree は `ui/node_modules`・`ui/vendor` 未復元のため、
  UI lane の前に `mise run setup-ui`（`npm ci --prefix ui`）を
  実行し、`mise run sync-ui-deps` で vendor を再生成した。
- doc graph route は main 既存の broken relation
  （`crates/al-math` 参照切れ）で blocked のため、前段 PR
  #190/#193/#194/#213/#214/#215 と同じく文書チェーンを直接作成し
  review-protocol で承認相当のレビューを行う運用とした。

## Related Experiments

### Adopted

<!-- なし -->

### Rejected

<!-- なし -->

## Validation

- `mise run check-ruviform-lock`: Pass（26 package、単一 SHA
  `eafe0a08b7...`）
- `mise run check-ruviform-rev`: Pass（REV = lockfile SHA、
  lockfile-primary、fail-closed 構造維持）
- `mise run sync-ui-deps` → `check-ui` → `test-ui`: Pass
  （typecheck + node:test 113 件 fail 0）
- `mise run fmt` / `check al-analysis` / `lint al-analysis` /
  `test al-analysis`: Pass（`rf628_oracle` 7 tests・長尺
  scenario 含む全 suite fail 0）
- `mise run check-cli-headless`: Pass（headless 10 crate に GUI
  framework 依存なし。`serde`/`serde_json` の dev-dep 波及なし）
- `mise run verify-toolchain`: Pass（rustc/cargo 1.99.1）
- `mise exec -- cargo fetch --locked`: Pass
- 補助証拠: `mise run check-full` Pass（workspace check）
- NotRun/Unsupported: 実 OS GUI・実 Pixel・macOS/Linux・
  cross-consumer・`gui-real`・upstream covering CI（`eafe0a08`
  対象の `ci`/`daily-ci`/`consumer-verify`）・ruviform manual
  lane・`*-full` の lint/test/build・`check-ui-gpui`/
  `check-ui-webview` 各 lane・`check-ci-workflow`/
  `check-ci-tracker-workflows`（正規検証コマンド対象外）。
  詳細は `docs/test-specs/ruviform-eafe0a08-rev-watch.md` を
  正本とする。

## Risks

- upstream の新機能領域（plugin state bridge・test fixture・
  real-editor attach）は AL lane 未踏。candidate tuple としての
  継続評価で担保し、Issue #127 は open のまま。
- upstream main の covering CI が `eafe0a08` に対して NotRun。
  merge 後の AL 対象 SHA の CI 結果が統合検証の証拠。
- mbx cache の absolute-path mapping / build-script UTF-8 warning
  （Git checkout 由来、記録のみ）。

## Follow-ups

- `RunMetadata` JSON・`StreamFault::Cancellation`・plugin state
  bridge・test fixture・real-editor attach の AL 採用判断は別
  slice の Issue で行う（本 slice では採用しない）。
- 実機 lane（実 OS GUI・実 Pixel・macOS/Linux）と upstream
  covering CI の整備は既存の NotRun 追跡に従う。
