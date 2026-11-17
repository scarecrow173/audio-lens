---
id: "PLAN-034YZx0x20snSt7KgRKYot"
type: "plan"
status: "approved"
title: "stale source 参照修復 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/220"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-220-stale-doc-refs.md"
      - type: "doc"
        path: "docs/specs/issue-220-stale-doc-refs.md"
      - type: "doc"
        path: "docs/designs/issue-220-stale-doc-refs.md"
      - type: "doc"
        path: "docs/plans/issue-220-stale-doc-refs.md"
      - type: "doc"
        path: "docs/tasks/issue-220-stale-doc-refs.md"
      - type: "doc"
        path: "docs/test-specs/issue-220-stale-doc-refs.md"
      - type: "doc"
        path: "docs/impl/ir/issue-220-stale-doc-refs.md"
    modified:
      - type: "doc"
        path: "docs/discovery/al-math-api-fft.md"
      - type: "doc"
        path: "docs/discovery/al-math-simd-reduction.md"
      - type: "doc"
        path: "docs/discovery/caller-side-optimizations.md"
      - type: "doc"
        path: "docs/discovery/issue-40-locate-pair-plan-cache.md"
      - type: "doc"
        path: "docs/discovery/mechanical-caller-cleanup.md"
      - type: "doc"
        path: "docs/discovery/observability-foundation-logging-and-profiling.md"
      - type: "doc"
        path: "docs/discovery/spectral-envelope-fft-size.md"
      - type: "doc"
        path: "docs/specs/locate-plan-cache.md"
      - type: "doc"
        path: "docs/specs/mechanical-caller-cleanup.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/issue-220-stale-doc-refs.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-220-stale-doc-refs.md"
    - "docs/designs/issue-220-stale-doc-refs.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-220-stale-doc-refs.md"
  references: []
  defers: []
  deferred-by: []
---

# stale source 参照修復 実装計画 Implementation Plan

## Goal

対象 9 文書の frontmatter `source` stale 参照 24 件を pinned blob
URL へ修復し、route_graph の blocked 状態を解消して doc graph の
正常な route 判定を回復する。

## Scope

spec `docs/specs/issue-220-stale-doc-refs.md` の In Scope に一致。
docs frontmatter と evidence のみを変更し、コード・ツール・
index は変更しない。

## Upstream Documents

- `docs/discovery/issue-220-stale-doc-refs.md`(棚卸し・分類・
  修復方針)
- `docs/specs/issue-220-stale-doc-refs.md`(要件・AC)
- `docs/designs/issue-220-stale-doc-refs.md`(設計判断)

## Design Inputs

- resolver 契約: `scheme:`/`//` 始まりは external ref、その他は
  repo 内実在ファイル必須。
- pinned revision: `1824fe169460a0e7314cb73686db638bf1ee9f5e`
  (`0a1bd24^`)。
- 分類基準: synvelis#519 と同型(frontmatter 記述の削除 / 有効参照
  への置換 / 意図的例外の明文化)。

## File Map

| 種別 | パス | 内容 |
| --- | --- | --- |
| modified | `docs/discovery/al-math-api-fft.md` | source 6 件修復 |
| modified | `docs/discovery/al-math-simd-reduction.md` | source 修復 |
| modified | `docs/discovery/caller-side-optimizations.md` | source 修復 |
| modified | `docs/discovery/issue-40-locate-pair-plan-cache.md` | source 修復 |
| modified | `docs/discovery/mechanical-caller-cleanup.md` | source 修復 |
| modified | `docs/discovery/observability-foundation-logging-and-profiling.md` | source 修復 |
| modified | `docs/discovery/spectral-envelope-fft-size.md` | source 修復 |
| modified | `docs/specs/locate-plan-cache.md` | source 2 件修復 |
| modified | `docs/specs/mechanical-caller-cleanup.md` | source 2 件修復 |
| added | `docs/{discovery,specs,designs,plans,tasks,test-specs,impl/ir}/issue-220-stale-doc-refs.md` | doc chain 7 件 |
| added | `docs/impl/exp/issue-220-stale-doc-refs-exp.jsonl` | experiment log |
| added | `evidence/al220/*.md` | 棚卸し・分類・blocker 判定・検証・残存 finding 記録 |

(正確なエントリ単位の対応表は `evidence/al220/broken-relation-inventory.md`)

## Implementation Sequence

- [x] 1. baseline 実測(route_graph `--json`・doc_status lint 全対象
      dir)と broken-relation 24 件の棚卸し
- [x] 2. 分類判断: 全件「有効参照への置換」(pinned blob URL)に
      一律決定
- [x] 3. 対象 9 文書の frontmatter `source` 修復(24 エントリ)
- [x] 4. doc chain(discovery→spec→design→plan→task→test-spec→
      impl-ir)と experiment log の作成・記述
- [x] 5. `evidence/al220/` への記録作成
- [x] 6. 検証(route_graph bare/--focus、lint 全 dir、`mise run fmt`)
- [x] 7. review-protocol 相当のレビューと blocker 解消
- [ ] 8. コミット・push・PR 作成(`Refs #220`)

## Dependency Graph

- 棚卸し → 分類 → frontmatter 修復 → doc chain / evidence → 検証
  → レビュー → PR。各段は逐次依存。frontmatter 修復と doc chain
  作成は独立だが、検証は両方の完了後に実施。

## Risk Register

| リスク | 影響 | 対策 |
| --- | --- | --- |
| pinned URL の選定ミス(存続しない revision) | provenance 参照が無効 | `1824fe16` = `0a1bd24^` で全 stale path 存続を確認済み。`git cat-file -e` 相当で存在確認 |
| 消し過ぎ(al-locate 存続 path の誤置換) | 実在ファイル参照の破壊 | 対象を `al-math/**` と `al-locate/src/dsp.rs` に限定。lint で再検証 |
| 新規 doc の index 系 finding | lint blocking warning 増 | 許容済み(index 再生成は統合時)。evidence に記録 |
| frontmatter 以外の本文誤記 | 歴史的言及の誤認 | 原則変更なし。誤認表現のみ最小注記 |

## Task Breakdown

`docs/tasks/issue-220-stale-doc-refs.md` に集約(本 slice は単一
Worker の逐次作業のため task doc 1 件)。

## Verification

| 検証 | コマンド | 期待 |
| --- | --- | --- |
| route 復帰 | `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json` | `broken-relation:*` 0・`artifact-graph` なし・`focus-required` のみ |
| route 返却 | 同上 `--focus docs/specs/locate-plan-cache.md` | `status: edge` + taskGraph |
| lint discovery | `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/discovery` | `provenance-invalid-source` 0 |
| lint specs | 同上 `--dir docs/specs` | `provenance-invalid-source` 0、他 baseline 以下 |
| lint 他 dir | 同上 `--dir docs/{designs,plans,tasks,test-specs,adr}` | baseline 以下+index 系のみ |
| 軽量整形 | `mise run fmt` | 差分なし |

実測値は `evidence/al220/verification.md` に記録。

## Open Questions / Gaps

- なし。残存 finding の別 Issue 化要否は
  `evidence/al220/remaining-findings.md` に判断を記録。

## Self-Review

- spec の R1〜R6 と AC が検証表に対応していることを確認。
- 編集範囲はタスク仕様の許可リストに限定(`docs/**/README.md`・
  `.agents/`・`crates/**` 等は未変更)。

## Implementation Handoff

- 実装者: 本 Worker(task `task_ba562f28761f` / dispatch
  `ctx_f394dbadbeb1`)。
- 成果物: 9 文書修復 + doc chain 7 件 + exp jsonl +
  `evidence/al220/` 4 件。

## Review Handoff

- route_graph blocked のため spec→design→plan→task の承認ゲートは
  `review-protocol` のレビューで代行(#127 系 rev-watch slice と
  同じ代替手順)。レビュー結果は PR 本文に記録。
