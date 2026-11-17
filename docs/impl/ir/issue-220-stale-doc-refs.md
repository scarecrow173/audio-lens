---
id: "IMPL-034YZx6QBW04BAak9deUjc"
type: "impl"
status: "completed"
title: "Issue #220 stale crates/al-math・al-locate source 参照修復記録"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/220"
    - "issue-101-ruviform-numeric-migration.md"
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
    - "docs/tasks/issue-220-stale-doc-refs.md"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/issue-220-stale-doc-refs-exp.jsonl"
    rejected: []
---

# Issue #220 stale crates/al-math・al-locate source 参照修復記録

- 対象 Issue: audio-lens #220
- 関連 Issue(soft): scarecrow173/synvelis#519(同型修復パターン)
- 関連 Issue: #101(al-math / al-locate::dsp → ruviform-numeric 移行、
  `docs/impl/ir/issue-101-ruviform-numeric-migration.md`)
- pinned 参照先 revision: `1824fe169460a0e7314cb73686db638bf1ee9f5e`
  (`0a1bd24^`)

## Summary

route_graph が `status: blocked` を返し続ける主因である、対象
9 文書の frontmatter `relations.source` が指す削除済み
`crates/al-math/**`(20 エントリ)・`crates/al-locate/src/dsp.rs`
(4 エントリ)の stale 参照 24 件を、削除直前 revision の pinned
GitHub blob URL へ置換した。ツール本体・docs index・コードは
変更していない。あわせて doc chain(discovery→spec→design→
plan→task→test-spec→impl-ir)と experiment log、
`evidence/al220/` の棚卸し・分類・blocker 判定・before/after
実測・残存 finding 記録を作成した。

## Context

`crates/al-math` は commit `0a1bd24`、`crates/al-locate/src/dsp.rs`
は commit `9db1153` で削除された(#101 の ruviform-numeric 移行)。
docs frontmatter の `source` は repo 相対パスで記録されており、
削除後は resolver が実在ファイルを要求するため全件
broken-relation となった。route_graph の blocker は
`artifact-graph`(派生)+ `broken-relation:*` 24 件 +
`focus-required`(no-focus 呼出の意図された hard stop)。

## Implementation

### Changes

- modified(9 文書、計 24 エントリ = al-math 20 + dsp.rs 4):
  - `docs/discovery/al-math-api-fft.md`(6:al-math 5 + dsp.rs 1)
  - `docs/discovery/al-math-simd-reduction.md`(3:al-math 2 + dsp.rs 1)
  - `docs/discovery/caller-side-optimizations.md`(5:al-math 4 + dsp.rs 1)
  - `docs/discovery/issue-40-locate-pair-plan-cache.md`(1:al-math 1)
  - `docs/discovery/mechanical-caller-cleanup.md`(2:al-math 2)
  - `docs/discovery/observability-foundation-logging-and-profiling.md`(1:al-math 1)
  - `docs/discovery/spectral-envelope-fft-size.md`(2:al-math 2)
  - `docs/specs/locate-plan-cache.md`(1:al-math 1)
  - `docs/specs/mechanical-caller-cleanup.md`(3:al-math 2 + dsp.rs 1)
  - 各 stale パス `../../crates/al-math/**`・
    `../../crates/al-locate/src/dsp.rs` を
    `https://github.com/scarecrow173/audio-lens/blob/1824fe169460a0e7314cb73686db638bf1ee9f5e/<path>`
    へ置換。`../../crates/al-locate/` の存続 path
    (`examples/bench_dsp.rs`・`src/methods.rs` 等)は未変更。
- added:
  - `docs/discovery|specs|designs|plans|tasks|test-specs|
    impl/ir/issue-220-stale-doc-refs.md`(doc chain 7 件)
  - `docs/impl/exp/issue-220-stale-doc-refs-exp.jsonl`
  - `evidence/al220/`(broken-relation 棚卸し・分類・blocker 判定・
    verification・remaining-findings の 4 件)

(文書ごとの正確なエントリ内訳は
`evidence/al220/broken-relation-inventory.md` を正本とする)

### Decisions During Implementation

- **置換形式**: 削除エントリではなく pinned blob URL への置換を
  採用。`source` の provenance(分析対象コード)を機械可読に保持
  するため。resolver は `scheme:` 始まりを external ref として
  受理することを実測確認。
- **pinned revision**: `1824fe16`(`0a1bd24^`)。全 stale path が
  存続する最後の tree state であり、削除 commit 群以降に当該
  path への変更がないことを `git log` で確認。
- **ruviform-numeric 現行ファイルへの置換は不採用**:
  ファイル単位対応が未確定(tests/examples/dsp.rs に 1:1 対応
  なし)で、provenance の意味を歪める。
- **本文の歴史的言及は原則未変更**: `crates/al-math` 等への本文
  中の言及は当時の分析記録であり finding を生じない。現在の
  事実として誤認される表現がある場合のみ最小限の注記を許容
  する方針とし、実際には注記を要する箇所はなかった。
- **承認ゲートの代替**: route_graph blocked のため
  spec→design→plan→task の承認ゲートは review-protocol の
  レビューで代行(#127 系 rev-watch slice と同じ代替手順)。

## Related Experiments

### Adopted

- `docs/impl/exp/issue-220-stale-doc-refs-exp.jsonl` — baseline
  実測、修復方針の仮説、置換実施、検証結果を時系列で記録。

### Rejected

- なし(代替案の比較は discovery doc の Alternatives に記録)。

## Validation

| 検証 | コマンド | 結果 |
| --- | --- | --- |
| route 復帰 | `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json` | `status: blocked` だが blockers = `["focus-required"]` のみ(broken-relation 24 件・artifact-graph 解消) |
| route 返却 | 同上 `--focus docs/specs/locate-plan-cache.md` | `status: edge`、`edgeId: probe-to-briefing`、delegate `briefing-flow`、taskGraph 返却 |
| lint discovery | `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/discovery` | 26 docs / 3 findings(全件新規 discovery doc の index 系。`provenance-invalid-source` 20→0) |
| lint specs | 同上 `--dir docs/specs` | 73 docs / 34 findings(`provenance-invalid-source` 4→0、他ルール baseline 以下+新規 spec の index/traceability 系) |
| lint 他 dir | 同上 `--dir docs/{designs,plans,tasks,test-specs,adr}` | baseline+新規 doc の index 系 3 件/dir のみ増加、他ルール悪化なし |
| 軽量整形 | `mise run fmt` | `cargo fmt --all -- --check` 差分なし(docs-only) |

実測の詳細は `evidence/al220/verification.md`、残存 finding の
件数・別 Issue 化要否は `evidence/al220/remaining-findings.md`。

## Risks

- 新規 doc 追加に伴う index 系 finding(`index-missing-entry`・
  `orphan-index`・`orphan-navigation`)は index 再生成まで残る。
  `index-missing-entry` は blocking warning だが、タスク仕様で
  index 未同期は許容済み(Coordinator が統合時に再生成)。
- pinned blob URL は commit `1824fe16` に固定されており、
  削除済み path の内容は不変。provenance として正しい。

## Follow-ups

- docs index(`docs/**/README.md`)の再生成 — Coordinator が統合時に
  `doc-maintenance` で実施。
- 残存 lint finding(非共通根因)の別 Issue 化要否 —
  `evidence/al220/remaining-findings.md` の判断を参照。
- 同型 stale 参照の再発防止(削除 commit checklist への docs
  frontmatter 棚卸し組み込み要否)— 同ファイルに判断を記録。
  必要なら別 Issue 起票を提案。
- Issue #220 の merge・Close は Coordinator が実施。
