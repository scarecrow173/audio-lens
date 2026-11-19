---
id: "DESIGN-034YZwTRwjYgvFmfPjawc6"
type: "design"
status: "approved"
title: "stale source 参照修復の設計(Issue #220)"
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
    - "docs/specs/issue-220-stale-doc-refs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# stale source 参照修復の設計(Issue #220)

## Context

`crates/al-math` は commit `0a1bd24`(`refactor!: migrate generic
f64 numeric/DSP kernels from al-math to ruviform-numeric`)で削除
され、`crates/al-locate/src/dsp.rs` は commit `9db1153`(dsp facade
撤去・ruviform-numeric 直接呼出し化)で削除された。削除以前に
作成された 9 文書の frontmatter `relations.source` は、当時の
分析対象コードを repo 相対パスで記録しており、削除後は
resolver が実在ファイルを要求するため全件 broken-relation と
なった。route_graph は `source` relation のみを解決対象とし、
blocker は `artifact-graph`(graphIssues 非空の派生)+
`broken-relation:*` 24 件 + `focus-required`(no-focus 呼出の
意図された hard stop)。

## Scope

### In Scope

- 対象 9 文書の frontmatter `relations.source` stale エントリ
  24 件の修復。
- 修復方針の一律ルール化: `../../crates/al-math/**` と
  `../../crates/al-locate/src/dsp.rs` を、削除直前の最終残存
  revision `1824fe169460a0e7314cb73686db638bf1ee9f5e`
  (`0a1bd24^`)の GitHub blob URL
  (`https://github.com/scarecrow173/audio-lens/blob/<sha>/<path>`)
  へ置換する。
- `evidence/al220/` への記録と doc chain / experiment log の作成。

### Out of Scope

- ツール本体(`.agents/` 配下)・docs index・対象外 docs・コード・
  CI/lockfile 類の変更(spec の Out of Scope と同一)。
- 本文中の歴史的言及の書き換え(原則そのまま。現在の事実として
  誤認される表現がある場合のみ最小限の注記を許容)。

## Components and Boundaries

- **正本境界**: 修正対象は docs frontmatter `relations.source`
  のみ。`source` は「この文書が分析・根拠とした provenance」を
  記録する relation であり、resolver は `scheme:`/`//` 始まりを
  external ref、その他を repo 内実在ファイルとして解決する。
- **置換先の選定**: 削除済みファイルの内容は pinned commit で
  不変に取得できる。`1824fe16` は `0a1bd24` の親であり、
  `crates/al-math/**` 全 20 path と `crates/al-locate/src/dsp.rs`
  が存続する最後の tree state(削除 commit 群より後の変更が
  stale path に及んでいないことを `git log` で確認済み)。
- **ruviform-numeric 現行ファイルへの置換を採用しない理由**:
  `correlation.rs`→`corr.rs`、`filter.rs`→`bands.rs` 等の
  ファイル単位対応は API 対応表からの推測であり、tests/examples/
  `dsp.rs` には 1:1 対応がない。`source` の provenance は
  「分析対象だったコード」であり、移行先コードではない。
- **移行記録 doc への retarget を採用しない理由**: provenance の
  方向が逆(本 discovery 群は移行記録から派生したのではなく、
  同じコードを独立に分析した)。

## Data and Control Flow

1. baseline 実測: `route_graph.js --json` → `status: blocked`、
   blockers = `artifact-graph` + `broken-relation:*` 24 件 +
   `focus-required`。`doc_status.js lint` で discovery 20 件・
   specs 4 件の `provenance-invalid-source` を確認。
2. 棚卸し: broken-relation 24 件を文書・パス単位で対応表化し、
   全件が同一根因(削除済みパス)であることを確認。
3. 修復: 9 文書の frontmatter `source` エントリを pinned blob
   URL へ機械的に置換(24 エントリ、削除・残置の分類はなし)。
4. doc chain / evidence 作成。
5. 検証: route_graph 再実行 + lint 全対象 dir + `mise run fmt`。

## Non-Functional Constraints

- `.agents/` 配下の生成物は直接編集しない(commit も不可)。
- `docs/**/README.md` index は再生成しない(未同期は許容済み)。
- `crates/al-locate/` は `src/dsp.rs` 以外存続するため、
  `../../crates/al-locate/examples/bench_dsp.rs`・
  `../../crates/al-locate/src/methods.rs` 等の存続 path は
  置換対象外(消し過ぎ注意)。
- 文書・コメント・報告は日本語。

## Risks and Trade-offs

- **pinned URL の陳腐化**: `1824fe16` 時点の内容を指すため、
  削除後に同 path へ変更が入ることはない(削除済みのため)。
  provenance としては「分析当時のコード」で正しい。
- **URL の可読性**: 長い URL になるが、resolver の external ref
  契約をそのまま利用でき、ツール拡張が不要。
- **index 未同期**: 新規 doc 追加により `index-missing-entry`・
  `orphan-index`・`orphan-navigation` が各 dir に発生するが、
  index 再生成は Coordinator の統合タスクに委譲済み。

## Deferred Design Concerns

- クレート削除・ファイル移動時の docs frontmatter `source`
  棚卸しを削除 commit の checklist に含める運用の明文化は
  `evidence/al220/remaining-findings.md` に判断を記録。AGENTS.md
  / POLICIES.md の変更は本 slice の範囲外。

## Verification Notes

- `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
  --json`: `broken-relation:*` 0 件・`artifact-graph` 消滅・
  bare 実行は `focus-required` のみ(期待 terminal)。
- `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
  --json --focus docs/specs/locate-plan-cache.md`: `status: edge`・
  `edgeId: probe-to-briefing`・taskGraph 返却。
- `node .agents/skills/doc-status/scripts/doc_status.js lint
  --dir docs/discovery`: 0 findings(新規 discovery doc の
  index 系 3 件を除く — index-missing-entry は blocking warning
  として残るが index 再生成は統合時)。
- `node .agents/skills/doc-status/scripts/doc_status.js lint
  --dir docs/specs`: `provenance-invalid-source` 4→0、他ルールは
  baseline 以下。
- `mise run fmt`(`cargo fmt --all -- --check`): docs のみの
  変更のため差分なしを確認。
- 実測値の詳細は `evidence/al220/verification.md`。
