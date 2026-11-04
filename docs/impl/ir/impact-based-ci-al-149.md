---
id: "IMPL-qy44xObaluKWYoHyVuVHq1"
type: "impl"
status: "completed"
title: "変更影響ベース CI・Fail-Closed summary 整合(AL#149)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/149"
  changes:
    added:
      - type: "code"
        path: "tools/classify-impact.mjs"
      - type: "doc"
        path: "docs/designs/impact-based-ci-issue-149.md"
      - type: "doc"
        path: "docs/tasks/impact-based-ci-issue-149.md"
      - type: "doc"
        path: "docs/impl/exp/issue-149-impact-ci-shadow.jsonl"
    modified:
      - type: "code"
        path: ".github/workflows/ci.yml"
      - type: "doc"
        path: "docs/designs/audiolens-independent-ci.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/impact-based-ci-issue-149.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/impact-based-ci-issue-149.md"
    - "docs/designs/audiolens-independent-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/impact-based-ci-issue-149.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-149-impact-ci-shadow.jsonl"
    rejected: []
---

# 変更影響ベース CI・Fail-Closed summary 整合(AL#149)

## Summary

Issue #149 (AL-CI-OPT-02)。PR diff を AudioLens 変更クラスへ分類する
`tools/classify-impact.mjs` と `impact` job を導入し、
`verify` / `ui-verify` / `webview-verify` の必須集合を導出する。
`summary` は required コンテキスト名と常時実行を維持したまま期待集合を
fail-closed 再検査し、Shadow Mode(`CI_IMPACT_SELECTIVE` 未設定)では
全 lane 実行で偽陰性検知を行う。

## Context

`docs/tasks/impact-based-ci-issue-149.md` の実装。
`docs/designs/impact-based-ci-issue-149.md` に変更クラス定義・
class→lane/OS/feature/Golden fixture の Matrix・依存伝播・
Shadow→Selective 移行契約を定義。ruviform #333 の汎用 Impact 契約は
本実装時点で main 未着地のため Issue 本文契約に準拠(差異は設計書に記録)。

## Implementation

### Changes

- `tools/classify-impact.mjs` 新設: 16 変更クラス
  (rust-core/analysis/source/dataset/service/cli/observability/gui/ts/
  ui-vendor/cargo-lock/toolchain/ci/ci-scripts/lfs-golden/docs/
  docs-config)へ分類し 3 lane の required/not_applicable を導出。
  未分類・共有基盤・非 PR event・取得失敗は安全側 Full CI
- `ci.yml` `impact` job: PR では `gh api pulls/N/files` で差分取得、
  分類結果・mode・classes・lane 別 expected/class/reason を
  output + 証跡表で公開
- `ci.yml` `hygiene` job 新設: `prek run --all-files` を `verify` から
  分離し常時実行へ — selective 化後も docs-only 変更が hygiene 検査を
  素通りしない構造を維持
- `verify`/`ui-verify`/`webview-verify`: `needs: [ruviform-access, impact]`
  + PR 時 `required` のみ実行の `if`。push to main / workflow_dispatch は
  classifier が mode=full を返すため常に Full CI
- `summary`: needs に impact/hygiene を追加し、期待集合を fail-closed
  再検査 — required lane の failure/cancelled/skipped は Fail、
  NotApplicable は根拠付き skip のみ許容、期待値の欠落・不正値も Fail。
  lane 別 classified/expected/結果/備考 の証跡表を emit。
  Shadow Mode で not_applicable 判定 lane が実実行失敗した場合は
  `SHADOW FALSE-NEGATIVE` 警告を発行
- `gui-real`: 従来どおり NotRun placeholder を証跡として記録
  (Browser/headless 成功を実機 GUI PASS としない)

### Decisions During Implementation

- **hygiene 分離**: 既存設計の「prek は全差分を検査する」不変条件と
  selective lane skip を両立させるため、paths-filter ではなく
  job 構造で分離(設計書 §paths-filter 禁止の系譜)
- **Ruleset 変更不要**: `main-protect` の required context は `summary`
  のみで、lane 個別の skip は Ruleset を通さない → required 未保護期間 0
- **rust 系 → webview-verify required**: `al-gui` が backend crate に
  依存するため backend 変更は webview 回帰を維持。`cli`/`observability`
  のみの変更は `al-gui` 依存外のため webview は not_applicable
- **lfs-golden → verify のみ**: `al-gui` テストは合成 WAV 生成で
  共有 fixture 非参照のため ui/webview は not_applicable
- **fallback 多重化**: PR ファイル取得失敗・未分類パス・分類器異常・
  非 PR event の 4 系統すべて Full CI 側へ倒す

## Related Experiments

`docs/impl/exp/issue-149-impact-ci-shadow.jsonl` — Shadow run の実測記録:

- Before (main run 41827679237): runner 合計 ~1481s / wall ~327s、
  docs-only でも全 lane 実行
- Shadow (run 41834440293, PR #152): 全 15 job success、
  `classes=ci,docs,ci-scripts` → 3 lane required(ci.yml 自己変更が
  正しく Full CI を強制)。runner 1595s / wall 370s
  (+114s は impact 25s + hygiene 29s 分離 overhead)
- summary gate が lane 別証跡表を emit し expected set satisfied で pass。
  SHADOW FALSE-NEGATIVE 警告なし・gui-real は not_run 記録維持

**採用(Shadow Mode でマージ)**。selective 化は複数 PR で偽陰性なしを
確認後 `CI_IMPACT_SELECTIVE=1` で有効化。

## Validation

実施済み(ローカル):

- [x] `mise exec -- actionlint .github/workflows/ci.yml` — pass
- [x] `mise exec -- prek run --all-files` — 全 hook pass
- [x] classifier 8 シナリオ: cli/docs・rust 系・ui-only・Cargo.lock・
      docs-only・shadow・push 非 PR・未分類 fallback — 全て期待集合一致
- [x] summary enforcement bash 6 シナリオ: pass・required fail・
      required skipped・justified skip・missing expectation・
      shadow false-negative — 全て期待どおり判定

実施済み(PR CI / run 41834440293):

- [x] impact: mode=shadow・classes 出力・lane 別 expected/reason emit
- [x] hygiene: prek --all-files が独立 job で pass
- [x] verify/ui-verify/webview-verify 3 OS + ruviform-access +
      gui-real(not_run)+ summary — 全 green
- [x] summary の期待集合 gate: lane 別証跡表 emit・
      "expected set satisfied" で pass
- [x] Ruleset 未変更のまま `summary` required context が報告されること
      (gh pr checks で summary が check として出る)

未実施(NotRun/Unsupported、理由明記):

- selective 実動(`CI_IMPACT_SELECTIVE=1` 後の lane skip)— 意図的に
  未実施。複数 PR の Shadow 観測で偽陰性なし確認が先行条件
- `gui-real` 実機 GUI — ruviform #312/#313 未接続のため従来どおり
  not_run placeholder(本変更で status 非変更)

## Risks

- `gh api pulls/N/files` は 3000 ファイル上限・API 失敗時は
  fallback=Full CI(安全側)。超大規模 PR は最適化対象外となるが
  正しさは損なわない
- selective 化後に分類表の欠陥が見つかった場合の rollback は
  variable 削除のみ(YAML/Ruleset 変更不要)
- workflow 本体は PR run まで e2e 検証不可(#142/#148 と同制約) —
  本 run が最初の e2e 証拠

## Follow-ups

- 複数 PR の Shadow 観測で偽陰性なしを確認後、`CI_IMPACT_SELECTIVE=1`
  を設定し selective 化(設計書の移行契約どおり)
- ruviform #333 契約が main 着地したら class 定義・用語の整合確認
- selective 化後の実測 Before/After を exp に追記(docs-only PR 等で
  lane skip 削減量を計測)
