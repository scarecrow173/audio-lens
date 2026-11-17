---
id: "SPEC-034YZwLV3g7PRe9pyO6a8B"
type: "spec"
status: "approved"
title: "route_graph blocked 解消: stale crates/al-math・al-locate source 参照の修復(Issue #220)"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/220"
    - "https://github.com/scarecrow173/synvelis/issues/519"
    - "https://github.com/scarecrow173/audio-lens/commit/0a1bd24ec01f55d84cedc8cfd2d081310ab1bab7"
    - "https://github.com/scarecrow173/audio-lens/commit/9db115394e80701f597ef019057d8b2a84bcd0c9"
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
  related:
    - "docs/impl/ir/issue-101-ruviform-numeric-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/issue-220-stale-doc-refs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# route_graph blocked 解消: stale crates/al-math・al-locate source 参照の修復(Issue #220)

## What Should Be Built

対象 9 文書の frontmatter `relations.source` が指す削除済み
`crates/al-math/**`(延べ 20 エントリ)と `crates/al-locate/src/dsp.rs`
(延べ 4 エントリ)への stale 参照 24 件を、有効な参照へ修正し、
doc-driven-dev-graph の route_graph が `status: blocked` でなくなり
正常に route を返す状態を回復する。あわせて棚卸し・分類・
blocker 判定・before/after 実測を `evidence/al220/` に記録し、
残存 finding の件数・内容・別 Issue 化要否を判断する。

## Why Now

route_graph が `status: blocked`・`delegate: null` を返し続ける間は
doc-driven-dev-graph フロー依存の Worker 作業が repo 全体で停止する。
blocked の主因は frontmatter `source` の stale 参照 24 件であり、
同一根因のため frontmatter 修正のみで完遂できる。
synvelis#519 と同型の障害クラスであり、修復パターンを共有する。

## Users and Value

- doc-driven-dev-graph を使う全 Worker/Coordinator: route 判定が
  復帰し、Graph Definition に基づく正規フローが再開できる。
- docs 保守者: 削除済みコードへの provenance が pinned revision の
  URL として保持され、後続の文書監査で参照先を追跡できる。

## Scope

### In Scope

- 対象 9 文書の frontmatter `relations.source` の stale エントリ
  24 件の修復(削除直前 revision `1824fe16` への pinned GitHub
  blob URL への置換)。
- 修復に伴う最小限の本文記述整合(削除済みクレートを現在の実装
  パスとして述べている箇所への注記。歴史的言及は原則そのまま)。
- Issue #220 doc chain(discovery/spec/design/plan/task/test-spec/
  impl-ir)と experiment log の作成。
- `evidence/al220/` への棚卸し・分類・blocker 判定・before/after
  実測・残存 finding 記録の作成。
- route_graph / doc_status lint / `mise run fmt` による検証。

### Out of Scope

- `.agents/` 配下ツール本体(`route_graph.js`・`doc_status.js` 等)
  の変更。`.agents/` は APM 生成物で直接編集・commit 不可。
- `docs/**/README.md` index の手編集・再生成・`index apply`
  (Coordinator が統合時に再生成)。
- 対象 9 文書以外の既存 docs の修正。
- doc_status lint の非共通根因 finding
  (`inconsistent-reciprocal-relation`・`traceability-missing-upstream`・
  `invalid-id-prefix`・`invalid-front-matter`・`invalid-status`・
  `self-relation`・`stale-upstream`・`plan-missing-test-spec-evidence`・
  `test-spec-invalid-verifies-target`・`test-spec-missing-verifies` 等)
  の修復(記録と別 Issue 化要否判断のみ)。
- `crates/**`・`tools/`・`ui/`・`.github/**`・`Cargo.toml`・
  `Cargo.lock`・`mise.toml` 系・`AGENTS.md`・`POLICIES.md` の変更。
- synvelis・ruviform リポジトリ側の変更。
- Issue #220 の Close(merge と Close は Coordinator が行う)。

## Requirements

- R1: `route_graph.js --json` が `status: blocked` でなくなり、
  `broken-relation:*` blocker が 0 件、`artifact-graph` が消える。
  `--focus <有効文書>` 指定で route(edge + delegate + taskGraph)を
  返す。bare 実行で残る `focus-required` は「明示 focus が渡される
  までの意図された hard stop」として根拠と共に記録する。
- R2: `crates/al-math` / `crates/al-locate/src/dsp.rs` への stale
  `source` 参照が解消されるか、有効な参照へ修正される。
- R3: `doc_status.js lint` で `provenance-invalid-source` が
  `docs/discovery` 20 件→0 件、`docs/specs` 4 件→0 件になり、
  他ルールの finding が修復前 baseline 以下(増加不可)。
- R4: 残存 finding の件数・内容・別 Issue 化要否を `evidence/al220/`
  に記録する。
- R5: 修復は正本(docs frontmatter)側で行い、`.agents/` 等の生成物を
  直接編集しない。
- R6: 棚卸し・分類・blocker 判定・before/after 実測を
  `evidence/al220/` に記録する。

## Acceptance Criteria

- [ ] route_graph(doc-driven-dev-graph)が blocked でなくなり、
      正常に route を返す。
- [ ] `crates/al-math` / `crates/al-locate` への stale 参照が
      解消されるか、有効な参照へ修正される。
- [ ] 残存 finding がある場合は件数と内容を記録し、別 Issue 化の
      要否を判断する。
- [ ] 修復が生成物(`.agents/` 等)の直接編集でなく正本側で
      行われている。

## Deferred Design Concerns

- 同型 stale 参照の再発防止(lint/CI への組み込み要否)は判断事項
  として `evidence/al220/` と PR 本文に記録する。ツール・CI 変更
  自体は本 slice の範囲外。

## Sources and Relations

- Issue: <https://github.com/scarecrow173/audio-lens/issues/220>
- 同型修復 Issue(soft 関係):
  <https://github.com/scarecrow173/synvelis/issues/519>
- 移行記録: `docs/impl/ir/issue-101-ruviform-numeric-migration.md`
- Discovery: `docs/discovery/issue-220-stale-doc-refs.md`
- Design: `docs/designs/issue-220-stale-doc-refs.md`
- Plan: `docs/plans/issue-220-stale-doc-refs.md`
- Task: `docs/tasks/issue-220-stale-doc-refs.md`
- Test spec: `docs/test-specs/issue-220-stale-doc-refs.md`
- Implementation record: `docs/impl/ir/issue-220-stale-doc-refs.md`
- Evidence: `evidence/al220/`
- pinned 参照先 revision: `1824fe169460a0e7314cb73686db638bf1ee9f5e`
  (`0a1bd24^` — 全 stale path が存続する最後の tree state)
