---
id: "TSPEC-034YZwUon7PniX1lnJv9Hf"
type: "test-spec"
status: "approved"
title: "route_graph・doc_status lint の stale 参照非再発契約"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-220-stale-doc-refs.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-220-stale-doc-refs.md"
    - "docs/designs/issue-220-stale-doc-refs.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# route_graph・doc_status lint の stale 参照非再発契約

## Purpose

Issue #220 の修復が「stale `source` 参照の除去」と「route_graph の
復帰」を外部から観測可能な形で保証する契約を定義する。コードの
単体テストではなく、doc graph ツールの実行結果を検証対象とする
(対象が docs のみの変更であり、Rust テストは適用外)。

## Feature

doc-driven-dev-graph の route_graph と doc-status の lint が、
削除済み `crates/al-math/**`・`crates/al-locate/src/dsp.rs` への
frontmatter `source` 参照を broken-relation /
`provenance-invalid-source` として報告しないこと。

## Rules

- RULE-1: `route_graph.js --json` の `blockers` に
  `broken-relation:*` が 1 件も含まれない。
- RULE-2: `route_graph.js --json` の `blockers` に
  `artifact-graph` が含まれない(graphIssues が空)。
- RULE-3: `route_graph.js --json --focus <有効文書>` が
  `status: "edge"`(または定義済み terminal)と `taskGraph` を
  返す。
- RULE-4: `doc_status.js lint --dir docs/discovery` の
  `provenance-invalid-source` が 0 件。
- RULE-5: `doc_status.js lint --dir docs/specs` の
  `provenance-invalid-source` が 0 件。
- RULE-6: `doc_status.js lint` の他ルール finding が修復前
  baseline を超えない(新規 doc 由来の index 系 finding を除く —
  index 再生成は統合時に Coordinator が実施)。
- RULE-7: bare 実行の `focus-required` blocker は「明示 focus が
  渡されるまでの意図された hard stop」であり違反ではない。

## Examples

| Scenario | Expected |
| --- | --- |
| `route_graph.js --json`(focus なし) | `status: "blocked"`、blockers = `["focus-required"]` のみ |
| `route_graph.js --json --focus docs/specs/locate-plan-cache.md` | `status: "edge"`、`edgeId: "probe-to-briefing"`、taskGraph 非 null |
| `doc_status.js lint --dir docs/discovery` | `provenance-invalid-source` 0 件(新規 doc の index 系を除く) |
| `doc_status.js lint --dir docs/specs` | `provenance-invalid-source` 0 件、他ルールは baseline 以下 |

## Guarantees

- 修復後、doc graph の blocked 状態は stale 参照に起因しない。
- provenance 情報は pinned revision の URL として保持され、
  参照先コードは commit `1824fe16` で不変に取得可能。
- 対象外 finding(非共通根因)は修復前後で悪化しない。

## Non-goals

- `.agents/` ツール自体の resolver 仕様変更。
- docs index(`README.md`)の再生成。
- 対象外 lint finding の修復。
- Rust コードのビルド・テスト(docs-only 変更のため検証対象外)。

## Risk

- pinned blob URL が将来の repo 構成変更で無効化される可能性は
  低い(commit は不変)。万が一 GitHub 側で参照が失われても、
  commit SHA `1824fe16` と path は frontmatter に残り、git 履歴から
  復元可能。
- index 系 finding(`index-missing-entry` 等)は index 再生成まで
  blocking warning として残るが、タスク仕様で許容済み。
