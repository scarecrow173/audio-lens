---
id: "DESIGN-034VbY8pKJZK6aSmZ7mmHA"
type: "design"
status: "approved"
title: "ruviform 271e2d41 pin 追従の設計と Legacy WebView 棚卸し分類"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  changes:
    added:
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "ui/vendor/ruviform/REV"
      - type: "file"
        path: "docs/designs/ruviform-271e2d41-rev-watch.md"
      - type: "file"
        path: "docs/discovery/issue-127-legacy-webview-inventory.md"
    modified:
      - type: "file"
        path: "ui/vendor/ruviform"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements:
    - "../specs/ruviform-271e2d41-rev-watch.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-271e2d41-rev-watch.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/issue-127-legacy-webview-inventory.md"
  defers: []
  deferred-by: []
---

# ruviform 271e2d41 pin 追従の設計と Legacy WebView 棚卸し分類

## Context

AudioLens は ruviform を Git 依存（manifest に `rev` を書かない
lockfile-primary 契約）で利用し、`Cargo.lock` の `source` fragment
が解決 SHA の正本。`tools/update-ruviform-rev.mjs` が
`cargo update -p <ruviform-package> --precise <sha>` を実行し、
`tools/sync-ruviform-ui.mjs`（`mise run sync-ui-deps`）が
`ui/vendor/ruviform` を lockfile SHA から再生成して `REV` を書く。
`tools/check-ruviform-rev.mjs` が lockfile/manifest/vendor の
一致を fail-closed に検査する。

候補 `271e2d4111d3769693445bc5bffca37af9e0650b` は ruviform main
HEAD（`git ls-remote` 確認済み、merge PR #551
`chore/rf243-verdict-rerun`）。前 pin `84d21641` から 88 commit・
474 file・+108,116/−638 行の差分。

## Scope

### In Scope

- `Cargo.lock` の ruviform 20 package を `271e2d41` へ更新し、
  vendor tree を再生成する。
- 差分の AudioLens 影響評価（API/Feature/OS）と検証 lane 実行。
- Legacy WebView の実在箇所棚卸しと分類（記録のみ、物理撤去なし）。

### Out of Scope

- 製品コード変更、Legacy WebView の物理削除、他 repo 変更、
  実機/実 Pixel/macOS/Linux/cross-consumer gate の実施。

## Components and Boundaries

- `tools/update-ruviform-rev.mjs` / `tools/sync-ruviform-ui.mjs` /
  `tools/check-ruviform-rev.mjs`: pin 追従と整合検査の既存機構。
  今回は機構自体を変更しない。
- `crates/al-gui`: `ruviform-ui`/`ruviform-ui-test`/
  `ruviform-ui-gpui`/`ruviform-ui-theme`/`ruviform-ui-webview` を
  消費する唯一の GUI crate。headless 既定、`webview`/`gpui`/
  `gpui-test` feature で adapter を切替。
- `ui/`（TypeScript）: `ui/vendor/ruviform` の vendored source を
  `binding`/`guitest` 経由で利用。vendor は生成物で直接編集しない。
- headless crate 群（`al-core`/`al-media`/`al-dsp`/`al-audio-*`/
  `al-analysis`/`al-job`/`al-dataset`/`al-service`/`al-cli` 等）:
  `ruviform-media-*`/`io-*`/`observability` 等を消費。

## Data and Control Flow

1. `node tools/update-ruviform-rev.mjs 271e2d4111d3769693445bc5bffca37af9e0650b`
   で `Cargo.lock` を更新（全 20 package が同一 SHA に解決）。
2. `mise run sync-ui-deps` で `ui/vendor/ruviform` を再生成し
   `REV` に SHA を記録。
3. `check-ruviform-lock`/`check-ruviform-rev` で lockfile-primary
   契約（単一 SHA・vendor 一致）を検査。
4. 各検証 lane（下記 Verification）で対象 revision の
   compile/test/lint を実行。
5. 棚卸し結果を discovery doc に記録し、採用/保留を判断。

## 差分影響評価（84d21641..271e2d41）

88 commit の主な領域と AudioLens への影響:

| 領域 | 主な変更 | AL 影響評価 |
| --- | --- | --- |
| `ruviform-ui` projection | RF#410 projection revision floor・view command ledger bounded retention（#502/#515） | AL の ViewSpec projection/receipt 経路が消費。stale projection 誤適用の防御強化で後方互換の破壊なし。`test-ui-gpui`/`test` で receipt/projection 系テスト全 pass |
| `ruviform-ui-test` | RF#464 Scenario Runner/Report（#511）、RF#314 Golden 環境正規化・fixture registry・証拠 lane（#514/#487 系） | AL#188 で採用済みの SemanticGuiTest scenario 契約の上流発展。AL 側 contract 利用は additive で、既存 scenario test は pass |
| `ruviform-ui-gpui` | native dock slice 3（#501）、pixel capture backend（#487）、実 OS SendInput/multiwindow smoke（#541/#498/#551）、Fader/Knob・keyboard 規約（#491/#486）、Tabs FocusHandle 修正（#496） | AL の GPUI feature/parity/desktop test が消費。semantic/headless 範囲では回帰なし。実 OS smoke は ruviform 側 evidence であり AL の gate ではない |
| `ruviform-ui-theme` | DTCG token→ResolvedTheme codegen（#505） | AL theme 解決テスト（light/dark/prefs）が pass。生成 typed theme の拡張は additive |
| `ui/binding`・`ui/guitest`（TS） | late-create epoch 採用の QueryBinding/ContractModel 契約整合（#489/#493/#488）、consumer-shaped semantic oracle fixtures（#543） | AL の `ui/` は binding/query を消費。`check-ui`/`test-ui`（112 tests）pass で epoch 契約の破壊なし |
| `ruviform-observability` | OBS-03 tracing_export（#507/#523/#539）、OBS-08 alloc probe（#506）、OBS-09 Diagnostics/Evidence/Finding（#504/#540）、OBS-10 debug（#508/#542）、OBS-11 config/runner（#509/#536）、統合設計（#519/#521） | opt-in feature 群。AL は `ruviform-observability` を依存するが新規 feature を有効化していない。`check`/`lint`/`test` pass |
| `ruviform-audio`/`audio-core` | PROBE-TIME 時計 identity・整列契約（#499/#495/#517）、Generation 切替時の Probe 再 Binding（#492）、real backend input plumbing（#538）、VST3 MIDI（#537） | AL の audio pipeline が消費するが API 破壊的変更なし。`test`（58 suite）pass |
| `ruviform-io-*` | IO-02/03/04/06 設計正本 + 契約試験（#518/#520/#522/#528/#529 等）、E2E feature matrix/fault fixture（#512/#533/#550） | AL の `ruviform-io-avi`/`io-file`/`io-symphonia` 消費に影響する実装変更は限定的。契約試験追加が主体。`test` pass |
| `ruviform-plugin-host` | Editor Session 状態機械/hardening（#530/#503/#524） | AL は plugin-host を直接依存しない（推移的）。影響なし |
| CI/mise | consumer-verify tuple mode・daily-ci・ci-issue-tracker（#534/#500/#533）、MSRV lane | AL 側 CI 定義への直接影響なし。`check-ci`/`lint-ci`/`test-ci` pass |

結論: 破壊的 API/Feature/OS 変更は検出されず、AL の全検証 lane が
対象 revision で pass。採用判断は「採用」（詳細は task doc と
PR 本文）。

## Non-Functional Constraints

- lockfile-primary 契約の維持（manifest `rev` 固定への回帰禁止）。
- fail-closed 検査（check-ruviform-lock/rev）の有効性維持。
- 検証は正規 mise task 経由。cargo 直実行は補助証拠として区別。
- 未実行 gate を Pass にしない（NotRun/Unsupported 語彙の厳守）。

## Risks and Trade-offs

- 大規模差分（88 commit）に対し lane 網羅性が不十分なリスク →
  正規検証 20 task を全実行し、未到達 gate は明示的に NotRun 記録。
- vendored `ui/vendor/ruviform` は生成物のため差分 review が
  lockfile SHA と upstream git log の照合に依存 → `REV` 一致と
  `check-ruviform-rev` で担保。
- 将来 pin を戻す rollback は `Cargo.lock` revert + `sync-ui-deps`
  再実行で可能（生成物のみの変更、製品コード差分なし）。

## Deferred Design Concerns

- Legacy WebView の物理撤去設計 | RF#465 の Owner Issue 確定と
  AL 側撤去 gate が前提 | 撤去 slice の spec 起票時 |
  棚卸し表が陳腐化 | docs/discovery/issue-127-legacy-webview-inventory.md
- pixel Golden / 実 OS 入力の AL 側採用判断 | ruviform 側 backend
  は存在するが AL の独立精度 gate 未整備 | gate 設計時 |
  semantic pass の過信 | 本 slice では NotRun 記録のみ

## Verification Notes

- [x] `mise run check-ruviform-lock` / `check-ruviform-rev`:
  単一 SHA・vendor 一致（fail-closed）
- [x] `mise run sync-ui-deps` → `check-ui` → `test-ui`:
  対象 SHA で Pass（112 tests）
- [x] `mise run fmt`/`check`/`lint`/`test`/`build`/
  `check-cli-headless`: workspace 全体 Pass
- [x] `mise run check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui`:
  GPUI feature lane Pass
- [x] `mise run check-ui-webview`/`lint-ui-webview`/
  `test-ui-webview`: Legacy WebView lane Pass（撤去せず維持を確認）
- [x] `mise run check-ci`/`lint-ci`/`test-ci`: CI profile Pass
- [x] 棚卸し表の作成と分類（残存/撤去候補/独立 Issue）
