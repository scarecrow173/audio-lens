---
id: "DESIGN-034Smu60yQfTlBNu23J8PP"
type: "design"
status: "approved"
title: "Issue #158 Full CI 統一(安全な第一段階): 全実行化・共通 ci-summary・expected_sha 契約"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/158"
  changes:
    added:
      - type: "file"
        path: "tools/check-ci-workflow.mjs"
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: ".github/actions/verify/action.yml"
      - type: "file"
        path: ".github/actions/ui-verify/action.yml"
      - type: "file"
        path: ".github/actions/webview-verify/action.yml"
      - type: "file"
        path: ".pre-commit-config.yaml"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "docs/designs/audiolens-independent-ci.md"
      - type: "file"
        path: "docs/guides/release-macos-verification.md"
      - type: "file"
        path: "docs/designs/impact-based-ci-issue-149.md"
      - type: "file"
        path: "docs/tasks/impact-based-ci-issue-149.md"
    deleted:
      - type: "file"
        path: "tools/classify-impact.mjs"
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/full-ci-ci-summary-issue-158.md"
  implemented-by:
    - "../plans/implement-full-ci-ci-summary-issue-158.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
  blocks: []
  supersedes:
    - "../designs/impact-based-ci-issue-149.md"
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/365"
    - "https://github.com/scarecrow173/ruviform/issues/366"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/159"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
  refines:
    - "../designs/audiolens-independent-ci.md"
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

# Issue #158 Full CI 統一(安全な第一段階): 全実行化・共通 ci-summary・expected_sha 契約

## Context

`ci.yml` は Issue #149/#153 で導入した変更影響ベース選択実行
(`impact` job + `tools/classify-impact.mjs` + `CI_IMPACT_SELECTIVE`)
を持ち、PR diff から lane 期待集合を導出して lane を skip する。
Ruviform #365 の横断決定により選択実行は廃止し、Full CI を
`workflow_dispatch` でも常時全 lane 実行にする。本設計は安全な
第一段階 — 既存 trigger と Required context `summary` を維持した
まま、全実行化と共通 `ci-summary` 集約・`expected_sha` 契約を
実現する。

問題境界:

- `main-checkstatus` Ruleset は check 名 `summary` を Required と
  する(実測値、Ruviform #365)。`summary` check を消すと全 PR が
  required 未達で滞留する — 本段階で消してはならない。
- #157 で macOS legs は Required `summary` から物理分離済み
  (macOS runner 逼迫時の滞留防止)。一方 #158 では macOS 失敗/
  未完了を Full CI 全体の失敗とする。両立のため「Full CI verdict
  (macOS 込み fail-closed)」と「PR Required gate(macOS 非含有)」
  を別 job として分離する。
- Scheduler/Tracker(#159)は未導入、Ruleset も未変更 — 本段階の
  成果だけでは移行完了でないことを設計・証跡の両面で明記する。

## Scope

### In Scope

- 選択実行機構の撤去(`impact` job、`tools/classify-impact.mjs`、
  `CI_IMPACT_SELECTIVE` env、Shadow Mode、期待集合照合、
  NotApplicable 変換、関連コメント)。
- 全 lane 常時実行(3 lane × ubuntu/windows/macos)と `expected-sha`
  gate job、`ci-summary` 集約 job の追加。
- Required context 互換 job `summary` の維持(判定範囲は現行の
  macOS 非含有 scope)。
- `macos-summary` job 撤去(集約責務は `ci-summary` に統一)。
- `tools/check-ci-workflow.mjs` 構造契約テスト + prek hook + mise
  task `check-ci-workflow`。
- 関連 docs(独立 CI 設計、macOS 検証手順、impact 設計/task の
  superseded 化)の同期。

### Out of Scope

- `pull_request`/`push` トリガー削除・`daily-ci.yml`・
  `ci-issue-tracker.yml`・Ruleset 変更(#159/#147 の後続段階)。
- `summary` job の撤去(`ci-summary` が Required 化された後)。
- lane 内部の検証内容の変更(Golden/CLI/UI/WebView 各 step は
  現行 composite action を維持)。

## Components and Boundaries

```
expected-sha (ubuntu)      # dispatch のみ expected_sha vs HEAD 照合(他 event は no-op success)
ruviform-access (ubuntu)   # access mode 解決(常時・現行維持)
hygiene (ubuntu)           # prek --all-files(常時・現行維持)
  ├─ verify (ubuntu/windows matrix)      # Rust 本体+Golden+CLI(常時)
  ├─ verify-macos (macos)                # 同一 composite(常時)
  ├─ ui-verify (ubuntu/windows matrix)   # ui/ TS + vendor 整合(常時)
  ├─ ui-verify-macos (macos)             # 同一 composite(常時)
  ├─ webview-verify (ubuntu/windows)     # al-gui webview feature(常時)
  ├─ webview-verify-macos (macos)        # 同一 composite(常時)
  └─ gui-real (ubuntu)                   # 実 GUI: not_run 記録 placeholder(常時)
       ├─ summary (ubuntu)               # Required context 互換 gate(macOS 非含有・fail-closed)
       └─ ci-summary (ubuntu)            # Full CI verdict(全 job fail-closed + 証跡集約)
```

- **expected-sha**: `workflow_dispatch` 時のみ checkout →
  `git rev-parse HEAD` が `inputs.expected_sha`(required string、
  40-hex 検証)と一致することを確認。不一致・不正形式は
  `::error::` + exit 1。pull_request/push では no-op success とし、
  DAG を event 非依存に保つ。全検証 job が needs するため、不一致
  run は lane 全 skip + `ci-summary` fail となる(lane 実行コストを
  掛けず fail-closed)。
- **lane jobs**: `needs: [ruviform-access, expected-sha]`、
  `if: ${{ !cancelled() && needs.ruviform-access.result == 'success'
  && needs.expected-sha.result == 'success' }}`。`impact` output
  への参照を完全撤去し常時実行。ubuntu/windows legs と `*-macos`
  legs の job 分割は維持 — Required `summary` が macOS を待たない
  #157 の性質を保つため matrix 再統合しない(#158 は独立 job /
  matrix 再統合のどちらも許容するが、Required gate に macOS を
  混入させない制約から独立 job 維持が唯一の両立案)。
- **ci-summary**: `needs: [expected-sha, ruviform-access, hygiene,
  verify, verify-macos, ui-verify, ui-verify-macos, webview-verify,
  webview-verify-macos, gui-real]` + `if: always()`。集約責務は
  旧 `summary` + 旧 `macos-summary` を統一したもの:
  - Job Summary: 対象 SHA(`github.sha`)・event・dispatch 時の
    `expected_sha`・access mode・resolved Ruviform SHA(全 lane
    output から非空値を採用)・lane×OS 別 result・検証範囲
    (Golden/LFS/Export = test-ci、CLI headless = verify、UI =
    ui-verify、WebView = webview-verify、実 GUI = gui-real
    not_run)・Run URL。
  - Fail-Closed gate: needs 全 job の result が `success` 以外
    (failure/cancelled/skipped/NotRun/unknown)なら `::error::` +
    exit 1。macOS legs を needs に含むため、macOS 失敗・滞留は
    Full CI 全体の失敗/未完了となる(check が Pending のまま
    残る性質も `always()` + needs 完走で維持される)。
  - 未検証入力の境界: `if: always()` のため expected-sha job が
    input を拒否しても本 job は実行される。`github.event.inputs.*`
    を run 本文へ式展開すると引用符等を含む入力が shell コード
    として解釈され得るため、dispatch inputs は `env:` 値として
    渡し、shell にはデータとしてのみ到達させる
    (`check-ci-workflow.mjs` が run 本文への入力式展開を検査)。
- **summary**: `needs: [expected-sha, ruviform-access, hygiene,
  verify, ui-verify, webview-verify, gui-real]` + `if: always()`。
  `main-checkstatus` Required context `summary` を継続発行する
  互換 gate。期待集合照合は撤去し、needs 全 job success のみを
  要求する単純 fail-closed。macOS legs を needs に含めない
  (現行契約どおり)。証跡の主本は `ci-summary` とし、本 job の
  表は PR gate 判定範囲のみを記録し Full CI verdict は
  `ci-summary` を参照する旨を明記する。
- **tools/check-ci-workflow.mjs**: ci.yml を読み、本設計の構造
  契約(trigger 3 種・expected_sha input 必須・job 集合・
  ci-summary/summary の needs・`always()`・impact 機構不在)を
  indentation ベースで構造的に検査する依存ゼロ Node スクリプト。
  違反検出は非 0 exit(fail-closed)。prek local hook
  (`check-ci-workflow`, `pass_filenames: false`, files: workflow
  自身とスクリプト)として登録し、`hygiene` job の
  `prek run --all-files` 経由で CI 全 run が実施する。
  `mise run check-ci-workflow` を検証入口として提供する。

## Data and Control Flow

- event = pull_request/push: `expected-sha` は no-op success、
  全 lane 常時実行、`summary`/`ci-summary` が集約・判定する。
- event = workflow_dispatch: 起動時に選択された ref の tip が
  `github.sha`/`github.ref` となる。`expected-sha` が checkout HEAD
  と入力 `expected_sha` を照合 → 一致なら lane 実行、不一致なら
  lane 全 skip、`ci-summary` が `expected-sha` failure +
  lane skipped を拾って run を fail にする。Scheduler(#159)は
  判定 SHA を `expected_sha` に渡すことで「判定した SHA を
  実測した」ことを機械的に保証する。
- resolved Ruviform SHA は各 lane composite が既存どおり
  `steps.guard.outputs.resolved_sha` → job output
  `resolved_sha` で公開し、`ci-summary` が収集する。

## Non-Functional Constraints

- `permissions: contents: read` のみ、secret は Deploy Key のみで
  現行経路を維持。
- `concurrency` 現行維持(PR は最新のみ、main は cancel しない)。
- mise scope・sccache・cargo cache 等の実測済み高速化は維持する
  (#365「実測済み高速化は残す」)。
- 証跡正直性: not_run/skipped/未実行を Pass に見せない
  (gui-real not_run、lane skipped の記録、macOS 未完了の
  Pending 維持)。

## Risks and Trade-offs

- **PR/push での CI コスト増**: 選択実行撤去で doc-only PR でも
  全 lane が走る。#158 が意図する移行期間中の既知 trade-off で、
  trigger 削除(最終段階)で解消する。
- **`summary`/`ci-summary` 二重集約**: 移行期間だけの重複。
  `summary` の判定範囲を現行どおりに限定することで required
  context の意味を変えず、`ci-summary` を Full CI 正本として
  将来の Required 切替(→ `summary` 撤去)に備える。
- **expected_sha の人的入力負荷**: required 入力のため手動
  dispatch は SHA 提示が必須。誤 SHA 入力は run 失敗で早期に
  表れる(fail-closed)。利便性のため optional 化すると契約が
  無効化し得るため採らない。
- **Ruleset が `summary` 以外を Required にしない**: `ci-summary`
  が PR 上で赤くても merge を阻害しない — これは旧 `macos-summary`
  の非ブロッキング性質と等価で、意図どおり。Full CI 正本の verdict
  は run 一覧・Checks タブで追跡可能。

## Deferred Design Concerns

- `summary` job 撤去 | `ci-summary` が Required context になれば
  不要 | `main-checkstatus` Required 変更(#147 Owner 作業)後 |
  放置すると恒久的な二重 gate。
- `pull_request`/`push` trigger 削除 | #158 変更 5、最終段階 |
  Scheduler 稼働 + Required 解除確認後 | 先行削除は required
  context デッドロック。
- `daily-ci.yml`/`ci-issue-tracker.yml` | #159 導入対象 | #158 の
  Full CI 契約成立後 | 未導入のままでは Daily 検証がない。
- macOS legs の Required 化 | `ci-summary` Required 化に内包 |
  最終段階 | 先行すると PR gate が macOS runner 待ちに逆戻り。

## Verification Notes

- [x] `ci.yml` 変更後の job 集合・needs・`if:` が本設計 §Components
      と一致する(`check-ci-workflow.mjs` で機械検証)。
- [x] `ci-summary` が macOS legs を含む全 job を needs し
      `always()` で fail-closed 判定する(同)。
- [x] `summary` の needs に macOS legs が含まれない(#157 性質維持、同)。
- [x] `impact`/`CI_IMPACT_SELECTIVE`/`macos-summary`/`classify-impact`
      参照が workflow から消える(同)。
- [x] `expected_sha` input が required で宣言される(同)。
- [x] `actionlint` が pass する。
- [ ] PR run で check 名 `summary`/`ci-summary` が発行される実証
      (本変更の push 後の CI で確認 — worker 環境では未検証)。
