---
id: "SPEC-034WQIsi86TSdELsIsTNHl"
type: "spec"
status: "approved"
title: "Issue #206 OS-L1 GPUI Desktop CI 統合: 3OS Native/Headless 検証 lane"
created: "2026-11-13"
updated: "2026-11-13"
owners:
  - "scarecrow173"
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/206"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/ci-os-l1-gpui-desktop.md"
    - "../plans/implement-ci-os-l1-gpui-desktop.md"
  depends-on:
    - "../specs/audiolens-independent-ci.md"
    - "../specs/full-ci-ci-summary-issue-158.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
    - "https://github.com/scarecrow173/audio-lens/issues/189"
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../designs/sccache-compile-cache-issue-148.md"
    - "../test-specs/ci-os-l1-gpui-desktop.md"
  refines:
    - "../specs/audiolens-independent-ci.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/206"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "POLICIES.md"
  defers: []
  deferred-by: []
---

# Issue #206 OS-L1 GPUI Desktop CI 統合: 3OS Native/Headless 検証 lane

## What Should Be Built

既存 `ci.yml` Full CI に、GPUI feature lane(`al-gui --features
gpui-test` の型チェック・Clippy・Headless テスト)と `al-desktop`
bin の Native Build/Link を Ubuntu/Windows/macOS の 3 OS で実行する
検証 lane を接続する。現行 `mise.toml` の `check-ui-gpui` /
`lint-ui-gpui` / `test-ui-gpui` task を CI から呼び出し、
Feature 無効による silent skip が起きない構成にする。

## Why Now

`mise.toml` には GPUI feature lane の task が定義済みだが CI に
未接続であり、Daily/手動 Full CI は GPUI backend の型・lint・
headless test・`al-desktop` bin の link を一切検証していない。
POLICIES.md の OS-L1(対象 OS での Native Build・Link・実行可能な
単体/統合/Headless テスト)は Windows/macOS/Linux 全てで必須で
あり、GPUI desktop 機能を製品として提供する以上その証拠が
必要。ローカル検証は変更範囲比例の方針(§5)のため、workspace
全体・3 OS 横断の GPUI 検証は Daily/手動 Full CI に委譲する
のが正規経路である。

## Users and Value

- AudioLens の開発者・レビュアー: GPUI desktop 機能の回帰を
  merge 前の統合検証(Daily/手動 Full CI)で検出できる。
- 受入判定者: OS-L1 の Pass/Fail/NotRun を lane 証跡と
  `ci-summary` fail-closed 集約で追跡できる。

## Scope

### In Scope

- `.github/workflows/ci.yml` への GPUI lane job 追加
  (ubuntu/windows matrix + macOS leg)と `ci-summary` /
  互換 `summary` の needs・証跡・fail-closed 集約の更新。
- `.github/actions/` への GPUI lane composite action 追加
  (既存 verify/ui-verify/webview-verify と同構造: ruviform
  access gate・mise・cargo cache・sccache・resolved SHA 証跡)。
- Linux runner への GPUI build/runtime 依存(X11/Wayland/
  fontconfig/Vulkan/lavapipe 等)の導入 step。
- `al-desktop` bin(`required-features = ["gpui"]`)の Native
  Build/Link を明示する検証 step(mise task 経由)。
- `tools/check-ci-workflow.mjs` の構造契約(job 集合・needs・
  ci-summary/summary 分離)の更新。
- `verify` lane の `check-full`/`lint-full` 重複コンパイルの
  計測と、同等の型診断/Clippy 保証が成立する範囲での整理。
- 本 doc-chain(spec/design/test-spec/plan/task)と各 docs index
  の更新。

### Out of Scope

- `crates/` 以下の実装コード、`al-gui`/`ruviform-ui-gpui` の
  feature・test 内容の変更。
- 実 OS Window を開く GUI-L2 検証(実入力・実画面)。headless
  `gpui-test` の成功を実 Window 操作の Pass と偽らない。
- 旧 WebView(`webview` feature・`webview-verify` lane)の撤去 —
  Issue #189 の責務。本変更は旧 lane を維持したまま独立して
  CI を利用可能にする。
- `daily-ci.yml`・`ci-issue-tracker*.yml` 等他 workflow、
  `mise.ci.toml`、Ruviform 共通 crate、解析精度 Oracle
  (Golden/LFS/Export/CLI)。
- macOS/Linux での GUI-L2/EXT-L3(Deferred のまま)。

## Requirements

- 3 OS 全てで `al-gui --features gpui-test` の型チェック・
  Clippy・Headless テストが実行され、Feature 無効時に対象が
  silent skip されないこと(`gpui-test` は `gpui` を内包し、
  `al-desktop` bin は `gpui` を要求する)。
- 3 OS 全てで `al-desktop` bin の Native Build/Link が成功する
  こと。
- Linux runner で GPUI の build 依存と `gpui-test` の実行時
  依存(software Vulkan = lavapipe)が整い、Windows/macOS
  runner で Native Link が検証されること。
- `tools/check-ci-workflow.mjs` の構造契約・`ci-summary` の
  全 OS fail-closed 集約・旧 `summary` 互換 scope が新 lane を
  正しく含み、OS 別 NotRun/Skipped/Fail を Pass 扱いしない
  こと。
- 既存の Golden/LFS/Export/CLI・headless TS・Ruviform resolved
  SHA 証跡・webview lane が削除・改変されないこと。

## Acceptance Criteria

- [ ] `ci.yml` に 3 OS の `al-gui --features gpui-test` 型/
  Clippy/Headless テストと `al-desktop` bin Native Build/Link
  が接続され、Feature 無効による silent skip がない。
- [ ] GPUI 用 Linux build deps、Windows/macOS runner での
  Native Link を検証し、UI-L2 の実 OS Window 操作を Pass と
  偽らない。
- [ ] `tools/check-ci-workflow.mjs` の構造検査、`ci-summary`
  の全 OS fail-closed 結果、旧 `summary` 互換との整合性が
  更新され、OS 別 NotRun/Skipped/Fail を Pass 扱いしない。
- [ ] 既存の Golden/LFS/Export/CLI、headless TS、Ruviform SHA
  証跡が削除されず、旧 WebView 撤去 Issue(#189)と独立して
  CI を利用できる。
- [ ] `check-full` と `lint-full` の重複コンパイルを計測し、
  同等の型診断/Clippy 保証で省略可能なら整理する(CI 速度
  改善は証拠付き)。
- [ ] 対象 SHA の 3 OS Full CI を手動実走し、必要 OS-L1
  ケースが Pass である証拠を示す。

## Deferred Design Concerns

- 実 OS Window の自動試験(gpui real-scenario / Xvfb offscreen
  pixel lane): Ruviform 側 `ui-gpui-offscreen-verify` が先行
  事例。AudioLens での GUI-L2 実測が必要になった時点で別
  Issue として扱う | 現行必須は OS-L1 のみ | GUI-L2 要件の
  発生 | 実 Window 回帰を headless 成功で代替してしまう |

## Sources and Relations

- Issue #206(本 spec の直接の根拠)。
- `POLICIES.md` §2(OS-L1 必須範囲)・§4(既存 Daily/手動 Full
  CI への統合方針)・§5(ローカル検証と Daily CI の責務分離)。
- `docs/specs/audiolens-independent-ci.md` / Issue #142:
  AudioLens 独立 CI の基盤 spec。
- `docs/specs/full-ci-ci-summary-issue-158.md` /
  `docs/designs/full-ci-ci-summary-issue-158.md`: Full CI
  構造・`ci-summary`/`summary` 分離の正本。本 spec はその
  job 集合契約を拡張する。
- `docs/designs/sccache-compile-cache-issue-148.md`:
  compile cache 構成(sccache GHA backend)の正本。GPUI lane
  も同一構成を踏襲する。
- Ruviform `ui-gpui-verify` lane(ruviform repo ci.yml、
  resolved SHA `fd1d4d5`): GPUI Linux build deps・sccache・
  headless `gpui-test` 構成の先行実装。
