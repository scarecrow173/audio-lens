---
id: "DISC-034YZwAZmWZ9St4bhxeGLS"
type: "discovery"
status: "resolved"
title: "route_graph blocked 要因の stale crates/al-math・al-locate 参照棚卸しと修復方針(Issue #220)"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/220"
    - "https://github.com/scarecrow173/synvelis/issues/519"
    - "https://github.com/scarecrow173/audio-lens/commit/0a1bd24ec01f55d84cedc8cfd2d081310ab1bab7"
    - "https://github.com/scarecrow173/audio-lens/commit/9db115394e80701f597ef019057d8b2a84bcd0c9"
    - "../impl/ir/issue-101-ruviform-numeric-migration.md"
  changes:
    added:
      - type: "doc"
        path: "docs/discovery/issue-220-stale-doc-refs.md"
    modified: []
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
  derives-from: []
  derived-by:
    - "docs/specs/issue-220-stale-doc-refs.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# route_graph blocked 要因の stale crates/al-math・al-locate 参照棚卸しと修復方針(Issue #220)

## Summary

`route_graph.js --json` が repo 全体で `status: blocked` を返す主因は、
9 文書の frontmatter `relations.source` が削除済みの
`crates/al-math/**`(延べ 20 エントリ)と
`crates/al-locate/src/dsp.rs`(延べ 4 エントリ)を指す
broken-relation 24 件と特定した。全件が同一根因であり、
frontmatter 修正のみで解消可能。`source` が記録する provenance
(文書が分析対象としたコード)を失わないよう、削除直前の最終残存
revision `1824fe16`(`0a1bd24^`)への pinned GitHub blob URL へ
置き換える方針を採用した。

## Exploration Goal

Issue #220: doc-driven-dev-graph の route_graph が `status: blocked`
を返し続け、Graph Definition による正規 route 判定が repo 全体で
停止している状態の原因棚卸しと修復方針の決定。synvelis#519 と
同型の障害クラスであり、修復パターンを共有する。

## Key Issues and Assumptions

- `crates/al-math` は commit `0a1bd24`(`refactor!: migrate generic
  f64 numeric/DSP kernels from al-math to ruviform-numeric`)で削除、
  `crates/al-locate/src/dsp.rs` は commit `9db1153`(dsp facade 撤去・
  ruviform-numeric 直接呼出し化)で削除。移行記録は
  `../impl/ir/issue-101-ruviform-numeric-migration.md`。
- `crates/al-locate/` は `src/dsp.rs` 以外存続する
  (`src/methods.rs`・`src/prepared.rs`・`examples/bench_dsp.rs` 等は
  実在し broken でない。過剰な削除を避けること)。
- resolver の受理形式: `scheme:`・`//` 始まりは external ref として
  受理される(実例: `issue-40-locate-pair-plan-cache.md` の `source`
  内 GitHub Issue URL は broken 扱いされない)。それ以外は repo 内の
  実在ファイル必須で、ディレクトリ参照は broken。
- route_graph の blocker 構成: `artifact-graph`(graphIssues 非空に
  連動する派生 blocker)+ `broken-relation:*` 24 件 +
  `focus-required`(`--focus` 未指定時の意図された hard stop)。
- 対象 9 文書の本文中の `crates/al-math/...` 言及は当時の分析記録と
  しての歴史的言及であり finding にならない。AC の「stale 参照」は
  frontmatter `source` を正本とする。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (i) frontmatter エントリ削除 | 最小 diff・lint 即 clean | provenance の機械可読な記録を喪失。`source` が「現存 evidence のみ」を意味するようになり、分析対象の記録が本文の散文のみに残る | no |
| (ii) pinned blob URL へ置換(削除直前 revision) | provenance を完全保持・永久に解決可能・一律ルールで検証容易 | URL が長い。削除済みファイルへの参照が残るが pinned commit で内容は不変 | yes |
| (iii) ruviform-numeric 現行ファイル URL へ置換 | 「コードの現在地」を指す | ファイル単位の対応が未確定(`correlation.rs`→`corr.rs`、`filter.rs`→`bands.rs` 等は API 対応表からの推測。tests/examples/`dsp.rs` は 1:1 対応なし)。誤った等価性を主張するリスク | no |
| (iv) 移行記録 doc への retarget | 実在 doc で解決可能 | provenance の方向が逆(本 discovery は移行記録から派生したのではなく、同じコードを分析した) | no |
| (v) 意図的例外として残置 | 変更なし | broken-relation が残り route_graph は blocked のまま。AC 不適合 | no |

## Tentative Conclusions and Hypotheses

- confirmed: broken-relation 24 件は全て `relations.source` が
  削除済みパスを指すもので同一根因。frontmatter 修正のみで
  `broken-relation:*` 0 件・`artifact-graph` 解消を実測確認した。
- confirmed: `focus-required` は `--focus` 未指定時の意図された
  hard stop であり、`--focus docs/specs/locate-plan-cache.md` 指定で
  `status: edge`・`edgeId: probe-to-briefing`・taskGraph 返却を
  実測。修復後の route_graph は正常に route を返す。
- hypothesis: 同型 stale 参照の再発防止は、クレート削除・ファイル
  移動を伴う変更時に docs frontmatter `source` の棚卸しを削除
  commit の checklist に含める運用で十分。検出自体は既存の
  `doc_status.js lint`(`provenance-invalid-source`)と
  route_graph(`broken-relation`)が担っており、ツール側拡張は
  不要と判断した(詳細は `evidence/al220/remaining-findings.md`)。

## Open Questions

- なし。残存する doc_status lint の非共通根因 finding
  (`inconsistent-reciprocal-relation`・`traceability-missing-upstream`・
  `invalid-id-prefix` 等)は本 Issue 対象外として件数と内容を
  `evidence/al220/` に記録し、別 Issue 化の要否を判断した。

## Promotion Candidates

- [x] spec-doc needed: stale `source` 参照を pinned blob URL へ
      修復し route_graph を復帰させる仕様 →
      `docs/specs/issue-220-stale-doc-refs.md`
- [ ] adr-doc needed: なし(frontmatter relation の参照形式選択は
      既存 resolver 契約の範囲内であり、新規アーキテクチャ判断を
      要しない)

## Sources and Relations

- Issue: <https://github.com/scarecrow173/audio-lens/issues/220>
- 同型修復 Issue(soft 関係):
  <https://github.com/scarecrow173/synvelis/issues/519>
- 削除 commit: `0a1bd24ec01f55d84cedc8cfd2d081310ab1bab7`
  (al-math 撤去)、`9db115394e80701f597ef019057d8b2a84bcd0c9`
  (dsp.rs 撤去)
- 移行記録: `docs/impl/ir/issue-101-ruviform-numeric-migration.md`
- pinned 参照先 revision: `1824fe169460a0e7314cb73686db638bf1ee9f5e`
  (`0a1bd24^` — 全 stale path が存続する最後の tree state)

## Evidence

- 2026-11-17 baseline 実測(main `7afabbc`):
  `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
  --json` → `status: blocked`、blockers = `artifact-graph` +
  `broken-relation:*` 24 件 + `focus-required`。内訳は discovery
  7 文書 20 件 + specs 2 文書 4 件。`doc_status.js lint` baseline:
  `--dir docs/discovery` = 20 findings(全件
  `provenance-invalid-source`)、`--dir docs/specs` = 34
  (`provenance-invalid-source` 4 + `inconsistent-reciprocal-relation`
  10 + `traceability-missing-upstream` 17 + `invalid-id-prefix` 3)、
  `--dir docs/designs` = 16 / `docs/plans` = 20 /
  `docs/tasks` = 16 / `docs/test-specs` = 10 / `docs/adr` = 3
  (後者はいずれも本 Issue 対象外の別根因)。
- 修復後実測: `broken-relation:*` 0 件、`artifact-graph` 消滅、
  bare 実行は `focus-required` のみ残存(期待 terminal)。
  `--focus docs/specs/locate-plan-cache.md` で `status: edge`・
  route 返却を確認。`doc_status lint --dir docs/discovery` =
  3 findings(全件本 doc の index 系 — `provenance-invalid-source`
  は 20→0)、`--dir docs/specs` = 33 findings
  (`provenance-invalid-source` 4→0、他ルールは baseline 以下)。
  実測値の詳細は `evidence/al220/verification.md` を正本とする。
- 修復対象 9 文書と 24 エントリの対応表・分類判断・blocker 判定・
  before/after 実測の詳細は `evidence/al220/` を参照。
