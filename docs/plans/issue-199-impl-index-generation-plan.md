---
id: "PLAN-034WsMb80vBXm59wk7FuFn"
type: "plan"
status: "approved"
title: "Issue #199 impl index 生成対応 実装計画"
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
  implements:
    - "../specs/issue-199-impl-index-generation.md"
    - "../designs/issue-199-impl-index-generation-design.md"
  implemented-by:
    - "../tasks/issue-199-impl-index-generation.md"
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
    - "../specs/issue-199-impl-index-generation.md"
    - "../designs/issue-199-impl-index-generation-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/issue-199-impl-index-generation.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #199 impl index 生成対応 実装計画 Implementation Plan

## Goal

`docs/impl/ir`・`docs/impl/exp` の管理 index を `doc-maintenance`
の safe sync engine で再生成可能にし、Issue #199 の受入条件
(index plan 非 rejected・apply 後 in-sync・引数なし apply exit 0)
を満たす。

## Scope

単一の実装ストリーム(front matter 是正 + marker 追加 + index
再生成 + doc チェーン同期)。複数 plan への分割は不要。

## Upstream Documents

- [docs/specs/issue-199-impl-index-generation.md](../specs/issue-199-impl-index-generation.md)
- Issue [#199](https://github.com/scarecrow173/audio-lens/issues/199)

## Design Inputs

- [docs/designs/issue-199-impl-index-generation-design.md](../designs/issue-199-impl-index-generation-design.md)
  (approved)— front matter 修正分類・managed index 化・検証手順。

## File Map

- `docs/impl/ir/*.md`(18 ファイル)— schema 不適合 front matter の
  形式修正。本文・ファイル名・id は不変。
- `docs/impl/ir/README.md` — generated-index marker 追加後、
  `sync_impl_index` で 4 列 managed index へ再生成。
- `docs/impl/exp/README.md` — 同様に marker 追加 + 1 列 index 再生成。
- `docs/specs|designs|plans|tasks|test-specs/issue-199-*.md` —
  本 doc チェーン(新規)。
- `docs/impl/ir/issue-199-impl-index-generation.md` — 本タスクの
  Implementation Record(新規)。
- `docs/{specs,designs,plans,tasks,test-specs,impl/ir}/README.md` —
  影響 dir の管理 index 再生成(生成物)。

## Implementation Sequence

- [x] `audit_impl_record.js --json` でベースライン error 数を記録
      (128 errors: invalid-front-matter 89 + missing-section 39、
      warning 21)。
- [x] `docs/impl/ir` 18 ファイルの front matter を形式修正
      (metadata 補完・changes object 化・全欠落 FM 生成・status
      正規化)。
- [x] `docs/impl/ir/README.md`・`docs/impl/exp/README.md` に
      generated-index marker を追加。
- [x] `index plan` → `index apply --expect` → `index plan` で
      ir/exp を in-sync 化。引数なし `index apply` exit 0 を確認。
- [x] doc チェーン(spec/design/plan/task/test-spec)と IR 記録を作成。
- [x] 影響 dir の管理 index を再生成し in-sync を確認
      (specs/designs/plans/tasks/test-specs/impl-ir の 6 dir、
      全 10 index in-sync)。
- [x] audit 再実行・ベースライン比較、review-protocol 実施
      (ACCEPT WITH NON-BLOCKING NOTES)。
- [ ] Conventional Commits で論理分割コミット → origin/main 取込
      → push → 日本語 PR。

## Dependencies and Risks

- `sync_impl_index` は fail-closed: marker なし index・schema 不正
  source は `rejected`。promote/force 経路は存在しないため、
  marker 追加と source 修正が唯一の経路(設計どおり)。
- front matter 修正で audit の missing-section 検査が活性化し
  error 件数が増える → coordinator 判断「新規 error なし」解釈で
  進め、内訳を PR 本文に記録。本文補完は別 Issue。
- plan hash は apply 時に再計算されるため、`--expect` には
  `index plan` の**トップレベル** planHash を使う(対象別 hash
  ではない)。

## Task Breakdown

- [docs/tasks/issue-199-impl-index-generation.md](../tasks/issue-199-impl-index-generation.md)
  — 単一 task。コミットは論理境界(front matter 是正 / index
  marker+再生成 / doc チェーン)で分割。

## Verification

- [x] `index plan --dir docs/impl/ir --json` → `sync`(rejected
      でない)、`index apply --expect <top planHash>` 成功後
      `index plan` → `in-sync`。
- [x] `docs/impl/exp`: marker 追加後 `index plan` → `in-sync`、
      `index apply --expect` も成功。
- [x] 引数なし `doc_maintenance.js index apply --json` → exit 0
      (written: []、blocked: [])。
- [x] `audit_impl_record.js --json`: error 128 → 150。
      invalid-front-matter 89→0、missing-section 39→150。
      差分は schema 適合化で可視化された既存本文不足
      (coordinator 承認済み解釈)。
- [x] `audit_experiment_log.js --json`: findings 0。
- [x] `audit_docs.js --type all --json`: 108 findings(57 errors)。
      新規 doc チェーン追加後も 108 findings・issue-199 由来の
      finding 0(ベースラインと同数、新規 error なし)。
- [x] review-protocol レビューで blocker なし
      (ACCEPT WITH NON-BLOCKING NOTES)。

## Open Questions / Gaps

- missing-section の本文補完は coordinator が別 Issue 化する。
  本 plan の完了条件には含めない。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(本 worker が直接実装)。
- Independent work streams: none(単一 task)。
- User approval required before dispatch: no — worker-guide により
  review-protocol 独立レビューを承認代替とし、本 worker が実装まで
  担う。

## Review Handoff

- reviewer 注目点: (a) front matter 修正が本文・id・ファイル名を
  変えていないこと、(b) `--expect` に top-level planHash を使う
  こと、(c) ir/README の旧 `Task | Record` 表を managed index で
  置き換えた判断、(d) audit error 増分の内訳説明が正確なこと、
  (e) 新規 ID の一意性。
