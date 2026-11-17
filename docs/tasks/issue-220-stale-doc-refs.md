---
id: "TASK-034YZx1dxbhs8e3HyFQIc3"
type: "task"
status: "done"
title: "stale source 参照修復と route_graph 復帰(AL#220)"
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
    - "docs/plans/issue-220-stale-doc-refs.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-220-stale-doc-refs.md"
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
    - "docs/test-specs/issue-220-stale-doc-refs.md"
  references: []
  defers: []
  deferred-by: []
---

# stale source 参照修復と route_graph 復帰(AL#220)

## Work

Issue #220 の docs-only slice。対象 9 文書の frontmatter
`relations.source` が指す削除済み `crates/al-math/**`・
`crates/al-locate/src/dsp.rs` への stale 参照 24 件を、削除直前
revision `1824fe16` の pinned GitHub blob URL へ置換し、
route_graph の `status: blocked` を解消する。あわせて doc chain・
experiment log・`evidence/al220/` 記録を作成し、検証を実施する。

## Classification

normal-plan-task

## Upstream Documents

- `docs/discovery/issue-220-stale-doc-refs.md`
- `docs/specs/issue-220-stale-doc-refs.md`
- `docs/designs/issue-220-stale-doc-refs.md`
- `docs/plans/issue-220-stale-doc-refs.md`
- `docs/test-specs/issue-220-stale-doc-refs.md`

## Dependencies

- plan `docs/plans/issue-220-stale-doc-refs.md` の実装シーケンスに
  従う。先行する外部依存なし。

## Affected Files

- modified: `docs/discovery/{al-math-api-fft,al-math-simd-reduction,
  caller-side-optimizations,issue-40-locate-pair-plan-cache,
  mechanical-caller-cleanup,
  observability-foundation-logging-and-profiling,
  spectral-envelope-fft-size}.md`、
  `docs/specs/{locate-plan-cache,mechanical-caller-cleanup}.md`
- added: `docs/{discovery,specs,designs,plans,tasks,test-specs,
  impl/ir}/issue-220-stale-doc-refs.md`、
  `docs/impl/exp/issue-220-stale-doc-refs-exp.jsonl`、
  `evidence/al220/*.md`

## Done When

- [x] broken-relation 24 件の棚卸し・分類・修復方針が
      `evidence/al220/` に記録されている
- [x] 対象 9 文書の `source` stale 参照が有効参照へ修正されている
- [x] route_graph が `broken-relation:*` 0 件・`artifact-graph`
      なしで route を返す(bare は `focus-required` のみ、
      `--focus` で `status: edge` + taskGraph)
- [x] `provenance-invalid-source` が discovery/specs で 0 件、
      他ルールが baseline 以下
- [x] 残存 finding の件数・内容・別 Issue 化要否が記録されている
- [x] `.agents/` 等生成物・`docs/**/README.md`・`crates/**` に
      変更がない

## Verification

- `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
  --json` → blockers = `["focus-required"]` のみ
- `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
  --json --focus docs/specs/locate-plan-cache.md` → `status: edge`、
  `edgeId: probe-to-briefing`、taskGraph 返却
- `node .agents/skills/doc-status/scripts/doc_status.js lint
  --dir docs/discovery` → `provenance-invalid-source` 0 件
- `node .agents/skills/doc-status/scripts/doc_status.js lint
  --dir docs/specs` → `provenance-invalid-source` 0 件、他 baseline
  以下
- `node .agents/skills/doc-status/scripts/doc_status.js lint
  --dir docs/{designs,plans,tasks,test-specs,adr}` → baseline 以下
  + 新規 doc の index 系のみ
- `mise run fmt` → 差分なし
- 実測値: `evidence/al220/verification.md`
