---
id: "SPEC-034WsMa4H4RpNbIsbWENgu"
type: "spec"
status: "approved"
title: "Issue #199 docs/impl/ir・exp 管理 index を doc-maintenance で生成可能にする"
created: "2026-11-12"
updated: "2026-11-12"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-199-impl-index-generation.md"
      - type: "doc"
        path: "docs/designs/issue-199-impl-index-generation-design.md"
      - type: "doc"
        path: "docs/plans/issue-199-impl-index-generation-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-199-impl-index-generation.md"
      - type: "doc"
        path: "docs/test-specs/issue-199-impl-index-generation.md"
      - type: "doc"
        path: "docs/impl/ir/issue-199-impl-index-generation.md"
    modified:
      - type: "doc"
        path: "docs/impl/ir/impact-based-ci-macos-separation-al-149.md"
      - type: "doc"
        path: "docs/impl/ir/issue-101-ruviform-numeric-migration.md"
      - type: "doc"
        path: "docs/impl/ir/issue-102-domain-separation.md"
      - type: "doc"
        path: "docs/impl/ir/issue-103-job-dataset-provenance-regression.md"
      - type: "doc"
        path: "docs/impl/ir/issue-105-gui-app.md"
      - type: "doc"
        path: "docs/impl/ir/issue-106-viz-removal.md"
      - type: "doc"
        path: "docs/impl/ir/issue-117-ruviform-contract-api.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-drawn-parity-run-mount.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-job-receipt-consumption.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-job-run-mount.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-product-screen-mount.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-spectrogram-analysis-projection.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-tabs-selection.md"
      - type: "doc"
        path: "docs/impl/ir/native-gui-waveform-seek-screen.md"
      - type: "doc"
        path: "docs/impl/ir/perf-reeval-2026-10.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/impl/ir/spectrogram-tab-hang.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
  implements: []
  implemented-by:
    - "../designs/issue-199-impl-index-generation-design.md"
    - "../plans/issue-199-impl-index-generation-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "https://github.com/scarecrow173/synvelis/issues/365"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-199-impl-index-generation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #199 docs/impl/ir・exp 管理 index を doc-maintenance で生成可能にする

## What Should Be Built

`docs/impl/ir`・`docs/impl/exp` の管理 index(`README.md`)を、
`doc-maintenance` Skill の安全な sync エンジン(`sync_impl_index`)
で再生成可能な managed index にする。具体的には:

1. `docs/impl/ir` 配下の front matter が実装記録 schema を満たさない
   既存文書を、意味を変えない形式修正で schema 適合させる
   (`metadata.experiments` 補完・`relations.changes` の型付き
   object 化・必須 front matter の補完・不正 status の是正)。
2. `docs/impl/ir/README.md`・`docs/impl/exp/README.md` に
   `<!-- doc-suite:generated-index -->` marker を追加し、
   手書き index を managed index へ転換する。
3. `doc_maintenance.js index plan` → `index apply --expect` →
   `index plan`(in-sync 確認)の正規手順で両 index を再生成する。
4. 既存文書の本文・ファイル名・id は変更しない(front matter の
   形式修正のみ)。

## Why Now

- Issue #199 の受入条件として、両 dir の `index plan` が
  `rejected` でなくなり `index apply` 後に `in-sync` になること、
  および引数なし `index apply` が exit 0 であることが要求されている。
- 現状 `index plan --dir docs/impl/ir` は source 文書の front matter
  不正(必須 `metadata` 欠落・`relations.changes` が文字列配列・
  front matter 全体の欠落・不正 status)で `rejected` となり、
  `docs/impl/exp` は index に generated-index marker がなく
  `hand-curated or ambiguous index` で `rejected` となる。
- front matter が schema 不正の文書は index 生成の source として
  受理されず、かつ audit 側でも検査がスキップされるため、
  修正によって潜在していた本文セクション不足が新たに可視化される
  (coordinator 判断済み: error 増分は既存不足の可視化であり、
  本文補完は別 Issue とする)。

## Users and Value

- **文書管理を行う開発者/agent**: `docs/impl/ir`・`docs/impl/exp`
  の index を `doc-maintenance index` で安全に再生成でき、
  文書追加・status 変更時の index ドリフトを機械的に解消できる。
- **doc-status / audit 利用者**: front matter が schema 適合に
  なり、invalid-front-matter error が解消される。
- **レビュアー**: managed index は決定的生成物となり、手編集との
  混在による不整合がなくなる。

## Scope

### In Scope

- `docs/impl/ir` 配下の schema 不適合 front matter の形式修正
  (18 ファイル)。
- `docs/impl/ir/README.md`・`docs/impl/exp/README.md` への
  generated-index marker 追加と managed index 化。
- `doc_maintenance.js index` による両 index の再生成と in-sync
  確認、引数なし `index apply` の exit 0 確認。
- 本 spec + design/plan/task/test-spec チェーンと本タスクの
  Implementation Record、および影響を受ける管理 index の再生成。

### Out of Scope

- 既存文書の本文・ファイル名・id の変更(front matter 形式修正を
  除く)。
- 製品コード・CI・依存定義の変更。
- `docs/impl/` 以外の docs 内容の手編集(各 dir の管理 index は
  生成物として再生成のみ)。
- audit で可視化された missing-section 等の本文不足の補完
  (coordinator 判断により別 Issue 化。PR 本文の「未対応の作業」に
  対象ファイルと不足 section を列挙する)。
- Ruviform #586・Synvelis #365(兄弟 Issue、別リポジトリの同一作業)。

## Requirements

- R1: `docs/impl/ir` の全 `.md` ソース(front matter を持つもの)が
  実装記録 schema(id/type/status/title/created/updated/owners/
  relations/metadata.experiments)を満たし、`index plan` が
  `rejected` でなくなる。
- R2: `docs/impl/ir/README.md`・`docs/impl/exp/README.md` が
  generated-index marker を持ち、`sync_impl_index` の safe な
  plan→apply 手順で再生成される。`promote_index` や force 経路は
  使わない。
- R3: `index apply --dir docs/impl/ir`・`--dir docs/impl/exp` 後に
  `index plan` がともに `in-sync` となる。
- R4: 引数なし `doc_maintenance.js index apply` が exit 0 となる
  (全管理 index が valid/in-sync)。
- R5: 既存文書の本文・ファイル名・id が変わっていない。
  front matter の形式修正は schema 適合のための機械的修正に限る。
- R6: `audit_impl_record.js`・`audit_experiment_log.js` の error が、
  これまで有効だったファイルで新規に増えていない
  (coordinator 解釈: invalid-front-matter の減少と、schema 適合化で
  可視化された既存 missing-section の増加を区別して報告する)。
- R7: 本タスクの doc チェーン(spec/design/plan/task/test-spec)と
  Implementation Record を作成し、影響を受ける管理 index を
  正規手順で再生成する。

## Acceptance Criteria

- [ ] `index plan --dir docs/impl/ir` が `rejected` でなく、
      `index apply` 後に `in-sync`。
- [ ] `index plan --dir docs/impl/exp` も同様に `in-sync`。
- [ ] 引数なし `index apply` が exit 0。
- [ ] ir/exp 既存文書の本文・ファイル名・id が不変
      (front matter 形式修正のみ)。
- [ ] `audit_impl_record.js` / `audit_experiment_log.js` の error が
      増えていない(coordinator 解釈: 新規 error なし・内訳を PR に
      記録)。

## Deferred Design Concerns

- 既存 IR 文書の本文必須セクション不足 | schema 不正で audit 検査が
  スキップされていた既存不足が front matter 修正で可視化された |
  別 Issue(coordinator が起票) | 修正しないと audit error が残る
- `docs/impl/ir/README.md` 旧来の `Task | Record` 手書き対応表 |
  managed index の `File` 列リンクと各文書の `implements` relation
  で代替可能 | 本 spec の index 再生成で置き換え済み | 情報が必要なら
  relation から復元可能

## Sources and Relations

- Issue [#199](https://github.com/scarecrow173/audio-lens/issues/199):
  本変更の起点。兄弟 Issue として Ruviform
  [#586](https://github.com/scarecrow173/ruviform/issues/586)・
  Synvelis [#365](https://github.com/scarecrow173/synvelis/issues/365)
  があり、依存関係はない。
- `impl-doc` Skill の `audit_impl_record.js` が front matter schema
  と必須セクションの正本。`doc-maintenance` Skill の
  `doc_maintenance.js index` が管理 index 再生成の正本手順。
- [issue-199-impl-index-generation-design](../designs/issue-199-impl-index-generation-design.md):
  front matter 修正方針と marker 設計。
