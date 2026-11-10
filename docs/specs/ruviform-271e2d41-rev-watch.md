---
id: "SPEC-034VbY715KTXsTOz3r5xlf"
type: "spec"
status: "approved"
title: "ruviform pin 271e2d41 追従評価と Legacy WebView 棚卸し（AL#127）"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "https://github.com/scarecrow173/ruviform/issues/465"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/specs/ruviform-271e2d41-rev-watch.md"
      - type: "file"
        path: "docs/designs/ruviform-271e2d41-rev-watch.md"
      - type: "file"
        path: "docs/plans/ruviform-271e2d41-rev-watch.md"
      - type: "file"
        path: "docs/tasks/ruviform-271e2d41-rev-watch.md"
      - type: "file"
        path: "docs/test-specs/ruviform-271e2d41-rev-watch.md"
      - type: "file"
        path: "docs/discovery/issue-127-legacy-webview-inventory.md"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-271e2d41-rev-watch.md"
  references:
    - "../discovery/issue-127-legacy-webview-inventory.md"
  defers: []
  deferred-by: []
---

# ruviform pin 271e2d41 追従評価と Legacy WebView 棚卸し（AL#127）

## What Should Be Built

AudioLens の ruviform 解決 pin を `84d21641a6bd50fa570ed074ae263284d05262ca`
から候補 `271e2d4111d3769693445bc5bffca37af9e0650b`（ruviform main
HEAD、merge PR #551）へ、lockfile-primary 手順
（`tools/update-ruviform-rev.mjs` → `mise run sync-ui-deps`）で追従する。
`Cargo.lock` の 20 ruviform package が単一 SHA に解決され、
`ui/vendor/ruviform/REV` が同一 SHA を指すことを確認する。

あわせて Issue #127 の Legacy 撤去 Gate に従い、旧 WebView
Host/feature/TS vendor/IPC/CI/配布の実在箇所を棚卸しし、
「残存（維持）/ 撤去候補 / 独立 Issue 化」を分類した記録を残す。
物理撤去は本 slice の範囲外とする。

## Why Now

Issue #127 は Ruviform 更新を AudioLens が Synvelis と対等な独立
Consumer として継続評価・導入するための常設追跡 Issue である。
候補 `271e2d41` は `84d21641..271e2d41` で 88 commit・474 file の
差分を含み、`ruviform-ui` の projection revision floor（RF#410）、
`ruviform-ui-test` の Scenario Runner/Golden 統合（RF#464/#314
続き）、`ruviform-ui-gpui` の native dock・pixel capture・実 OS
SendInput/multiwindow smoke（RF#388/#313/#243）、`ruviform-ui-theme`
の DTCG codegen（RF#418）、`ruviform-observability` の OBS-03/08/09/
10/11、`ruviform-audio` の PROBE-TIME/Generation 再 Binding、IO-02/03/
04/06 の契約試験、binding/query epoch 契約修正（RF#377/#378/#490）
などを含む。AudioLens が採用済みの Scenario contract・ViewSpec
projection・theme・binding 経路に直接関係する変更を含むため、
candidate tuple として評価・記録する必要がある。

## Users and Value

- AudioLens 開発者: pin 追従の検証証拠と影響範囲の記録により、
  次候補の評価コストを下げ、rollback 判断材料を残せる。
- Coordinator / Reviewer: Issue #127 の受入条件に対応する
  検証結果（実測と NotRun の区別）を PR/Issue から追跡できる。
- 将来の Legacy 撤去作業: 棚卸し表により撤去対象の実在箇所と
  分類（維持/撤去候補/独立 Issue）が明確になる。

## Scope

### In Scope

- ruviform pin `84d21641` → `271e2d41` の lockfile-primary 追従
  （`Cargo.lock` 20 package の単一 SHA 解決、`ui/vendor/ruviform`
  の再生成と `REV` 一致）。
- `84d21641..271e2d41` の差分調査と、AudioLens に影響する
  API/Feature/OS の変更・関連 Issue の記録。
- 正規 mise 検証 lane（fmt/check/lint/test/build、CLI headless、
  UI、GPUI、WebView、CI profile）の対象 revision での実行と結果記録。
- Legacy WebView の棚卸しと分類記録（物理撤去なし）。
- 採用/保留判断、rollback 手順、Issue #127 への結果記録、PR 作成。

### Out of Scope

- 製品コード（`crates/`・`src/`）の機能変更。pin 追従でコンパイル/
  テストが壊れた場合の adapter 修正のみ範囲内（今回は不要だった）。
- Legacy WebView の物理削除、`webview` feature/依存の除去、
  `.github/` の WebView lane 撤去（RF#465 系の別 slice/別 Issue）。
- ruviform・synvelis リポジトリの変更。
- 実 OS GUI・実 Pixel・macOS/Linux・cross-consumer gate の実施
  （環境到達不能なものは NotRun/Unsupported として記録）。
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
- 採用/保留判断と rollback 情報（旧 SHA `84d21641` への復帰手順）
  を記録すること。

## Acceptance Criteria

- [x] ruviform pin が `271e2d4111d3769693445bc5bffca37af9e0650b` に
  解決され、`mise run check-ruviform-lock` / `check-ruviform-rev`
  が pass する。
- [x] `Cargo.lock` と `ui/vendor/ruviform/REV` が同一 SHA を指す。
- [x] `84d21641..271e2d41` の影響 API/Feature/OS と関連 Issue を
  docs に記録する。
- [x] 正規 mise 検証 lane を対象 revision で実行し、結果を
  test-spec の対応表どおり記録する。
- [x] Legacy WebView 棚卸し表（残存/撤去候補/独立 Issue 分類）を
  docs に作成する。
- [x] 採用/保留判断と rollback 情報を記録する。
- [x] Issue #127 へ結果を comment し、PR 本文に `Refs #127` を含める
  （Issue は Close しない）。

## Deferred Design Concerns

- Legacy WebView の物理撤去 | RF#465 の consumer 撤去 Owner Issue
  と AL 側の独立 gate が前提 | 撤去 slice 着手時 | 撤去判断材料が
  陳腐化する | docs/discovery/issue-127-legacy-webview-inventory.md
- 実 OS GUI / pixel Golden / macOS・Linux 検証 | 本 slice の実行
  環境（Windows のみ、実ウィンドウ操作なし）では到達不能 |
  実機 lane 整備時 | semantic/headless pass を実機 pass と誤認する
  リスク | PR 本文の NotRun 欄と test-spec に記録

## Sources and Relations

- Issue #127（常設追跡 Issue、本 slice の受入条件の出典）
- ruviform#219（update/Consumer 共通契約）、ruviform#294
  （candidate tuple Consumer 検証）、ruviform#465（WebView 撤去
  棚卸しの upstream 側）
- 前段採用: PR #188（pin `84d21641`、SemanticGuiTest scenario 採用）
- 棚卸し詳細: docs/discovery/issue-127-legacy-webview-inventory.md
