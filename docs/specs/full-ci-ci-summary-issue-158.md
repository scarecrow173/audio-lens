---
id: "SPEC-034SmqnKZslN3lFq9I7ahH"
type: "spec"
status: "approved"
title: "Issue #158 Full CI 統一: 全実行化・共通 ci-summary Fail-Closed 集約・expected_sha 契約(安全な第一段階)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/full-ci-ci-summary-issue-158.md"
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
  depends-on:
    - "../specs/audiolens-independent-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/366"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/149"
    - "https://github.com/scarecrow173/audio-lens/issues/153"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
  refines:
    - "../specs/audiolens-independent-ci.md"
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #158 Full CI 統一: 全実行化・共通 ci-summary Fail-Closed 集約・expected_sha 契約(安全な第一段階)

## What Should Be Built

`ci.yml` を変更影響ベースの選択実行から、常時全 lane 実行の Full CI
へ戻す。最終集約 job `ci-summary` が必要な自動 job 全件の成功のみを
Pass とする fail-closed 判定を担い、`workflow_dispatch` に
`expected_sha` 契約(判定 SHA と checkout revision の一致検証)を
追加する。

本 spec は Issue #158 の**安全な第一段階**を対象とする:
`pull_request`/`push` トリガーと Ruleset Required context `summary`
は維持し、required context を死なせない。Scheduler(`daily-ci.yml`)
/Tracker(`ci-issue-tracker.yml`)の導入、Ruleset 変更、PR/push
トリガー削除は後続段階(Issue #159 / #147)であり、本段階で
移行完了を主張しない。

## Why Now

- Ruviform #365 の横断移行計画で、全 3 repo の CI を「Daily の
  未検証変更がある時だけ Full CI、必要時手動 Full CI」へ統一する
  フェーズ 1 が AudioLens では Issue #158 として起票されている。
- 外部 Runner 待機がクリティカルパス PR を停止させないよう、
  選択実行と PR Required Gate を引退させる方針が確定した。
- トリガー削除は Owner が `main-checkstatus` の Required を
  解除した後でなければデッドロックするため、先に Full CI 化・
  集約統一・SHA 契約を成立させる段階が必要。

## Users and Value

- **AudioLens Owner/開発者**: 全変更が常に全 lane で検証され、
  選択実行の偽陰性リスクが消える。手動 dispatch で任意 SHA の
  Full CI を即時得られる。
- **Daily Scheduler(#159 導入予定)**: `workflow_dispatch` +
  `expected_sha` 契約と `ci-summary` の単一 verdict に依存できる。
- **リリース/互換受入**: macOS legs の成否が `ci-summary` の
  Fail-Closed 判定に含まれ、対象 SHA の macOS 実測証跡が
  Job Summary に残る。

## Scope

### In Scope

- `impact` job・`tools/classify-impact.mjs`・`CI_IMPACT_SELECTIVE`
  env・Shadow Mode・期待集合照合・NotApplicable 処理の撤去。
- `verify`/`ui-verify`/`webview-verify` の ubuntu/windows legs と
  `*-macos` legs を全イベントで常時実行する。
- 新規 job `expected-sha`: `workflow_dispatch` 時に
  `inputs.expected_sha`(必須・40 hex)と checkout HEAD の一致を
  検証する fail-closed gate。
- 新規 job `ci-summary`: 全必要 job(expected-sha, ruviform-access,
  hygiene, 3 lane × ubuntu/windows/macos legs, gui-real)を
  `if: always()` + needs で待ち、success 以外(failure/cancelled/
  skipped/NotRun/unknown)を全て run 失敗にする fail-closed 集約。
  Job Summary に対象 SHA・event・resolved Ruviform SHA・lane/OS
  別結果・検証範囲(Golden/CLI/UI/WebView/gui-real)・Run URL を
  記録する。
- Required context `summary` の維持: check 名 `summary` の job を
  残し、PR gate としての判定範囲(macOS legs を含まない現行 scope)
  を変えない。
- `gui-real` の not_run 証跡・Golden/LFS/Export・CLI headless 境界・
  Cargo.lock/UI vendor revision 整合ガードの維持。
- `tools/check-ci-workflow.mjs`: 上記構造契約を静的に検査する
  回帰テスト(prek hook + hygiene job 経由で全 run が実施)。
- 関連 docs・テストの同期(本 spec + design/plan/task チェーン、
  impact 設計の superseded 化、CI 設計・macOS 検証手順の更新)。

### Out of Scope

- `pull_request`/`push` トリガーの削除(最終段階。#147 の
  Ruleset 解除確認後)。
- `daily-ci.yml` / `ci-issue-tracker.yml` の新設(Issue #159)。
- Ruleset `main-checkstatus` の Required 変更(Owner 作業)。
- `main-protect` の変更(維持する)。
- `gui-real` の実装化(Ruviform #312/#313 待ち)。
- macOS legs の Required 化(Ruleset 移行時に `ci-summary` が
  Required となる最終段階で成立する)。

## Requirements

- 全 lane が PR/main push/dispatch の全イベントで毎回実行される。
- `ci-summary` は needs 全 job の result が `success` のときのみ
  成功する。`always()` で起動し、skipped/cancelled/unknown を
  Pass にしない。
- macOS leg の失敗・未完了は `ci-summary`(= Full CI 全体)の失敗。
  Required `summary` の判定範囲は現行(ubuntu/windows legs)のまま
  維持し、macOS runner 逼迫で PR gate が滞留する #157 対策を
  後退させない。
- `workflow_dispatch.inputs.expected_sha` は必須入力。checkout した
  revision と一致しない run は即失敗する(判定 SHA ≠ 実測 SHA を
  証跡なしに通さない)。
- 既存の証拠網(各 lane の GITHUB_STEP_SUMMARY、resolved SHA
  output、gui-real not_run)を維持し、`ci-summary` が OS/lane 別に
  集約する。
- テスト無効化・検証範囲縮小・pass 偽装は行わない。

## Acceptance Criteria

- [ ] `pull_request`/`push`/`workflow_dispatch` の全イベントで
      verify/ui-verify/webview-verify の ubuntu・windows・macOS legs
      が実行される(選択実行機構なし)。
- [ ] `ci-summary` job が存在し、全必要 job を `always()` で待ち、
      1 job でも非 success なら run が失敗する。
- [ ] check 名 `summary` が発行され続ける(required context 不死)。
- [ ] `workflow_dispatch` で `expected_sha` 不一致の run が
      `expected-sha` job で明示失敗し、下流が全て skipped になり
      `ci-summary` が fail する。
- [ ] `impact`/`classify-impact.mjs`/`CI_IMPACT_SELECTIVE`/Shadow
      Mode/期待集合照合/`macos-summary` が撤去される。
- [ ] `mise exec -- node tools/check-ci-workflow.mjs` が契約違反を
      fail-closed で検出する。
- [ ] `mise exec -- actionlint .github/workflows/ci.yml` と
      `prek run --all-files` が pass する。

## Deferred Design Concerns

- `ci-summary` への Required context 切替: `main-checkstatus` の
  Required を Owner が `summary`→`ci-summary` へ変更した後、
  `summary` job を撤去する。切替前に `summary` を消すと required
  context が死に全 PR が滞留するため、本段階では両立させる。
- PR/push トリガー削除: Scheduler(#159)稼働 + Required 解除
  確認が前提。本段階では PR/push でも重い Full CI が走るため、
  移行期間中の CI コスト増は既知の trade-off。
- macOS legs の Required gate 化: 最終段階で `ci-summary` が
  Required になれば自動的に成立する。

## Sources and Relations

- Issue [#158](https://github.com/scarecrow173/audio-lens/issues/158)
  (変更 1-4・6 が本段階、5 は後続段階)。
- 横断移行計画 [Ruviform #365](https://github.com/scarecrow173/ruviform/issues/365)
  (共通決定 1-3: PR/push 廃止方向・Full CI 非選択・`ci-summary` 名)。
- 前段の選択実行設計
  [impact-based-ci-issue-149](../designs/impact-based-ci-issue-149.md)
  は本変更で superseded となる。
- 基盤設計 [audiolens-independent-ci](../specs/audiolens-independent-ci.md)
  の不変条件(hygiene 常時・paths filter なし・秘密の最小権限)を継承する。
