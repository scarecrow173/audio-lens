---
id: "TASK-034WsMpMwgPJoeSbeFZiC5"
type: "task"
status: "done"
title: "impl index 生成対応(AL#199): ir/exp front matter 是正と managed index 化"
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
    - "docs/plans/issue-199-impl-index-generation-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-199-impl-index-generation-plan.md"
    - "docs/designs/issue-199-impl-index-generation-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "https://github.com/scarecrow173/synvelis/issues/365"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-199-impl-index-generation-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-199-impl-index-generation.md"
  references: []
  defers: []
  deferred-by: []
---

# impl index 生成対応(AL#199): ir/exp front matter 是正と managed index 化

## Work

`docs/impl/ir` 配下 18 ファイルの schema 不適合 front matter を
形式修正し、`docs/impl/ir/README.md`・`docs/impl/exp/README.md`
に generated-index marker を追加して managed index 化する。
`doc_maintenance.js index plan` → `index apply --expect` →
`index plan`(in-sync)の正規手順で両 index を再生成し、引数なし
`index apply` の exit 0 を確認する。あわせて本タスクの
doc チェーンと Implementation Record を作成し、影響を受ける
管理 index を再生成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-199-impl-index-generation-plan.md`
- `docs/designs/issue-199-impl-index-generation-design.md`
- `docs/specs/issue-199-impl-index-generation.md`
- `docs/test-specs/issue-199-impl-index-generation.md`

## Dependencies

- `impl-doc`/`doc-maintenance` Skill の script が正本
  (`audit_impl_record.js` の schema、`doc_maintenance.js index`
  の safe sync engine)。
- なし(単一 worktree 内の一連編集)。

## Affected Files

- `docs/impl/ir/*.md`(18 ファイル、front matter のみ)、
  `docs/impl/ir/README.md`、`docs/impl/exp/README.md`、
  doc チェーン 5 ファイル + IR 1 ファイル(新規)、
  `docs/{specs,designs,plans,tasks,test-specs}/README.md`
  (生成 index)。

## Done When

- [x] `index plan --dir docs/impl/ir` が `rejected` でなく、
      `index apply` 後に `in-sync`。
- [x] `docs/impl/exp` も `in-sync`。
- [x] 引数なし `index apply` が exit 0。
- [x] 既存文書の本文・ファイル名・id が不変(front matter 形式
      修正のみ)。
- [x] `audit_impl_record.js`/`audit_experiment_log.js` の error が
      新規に増えていない(invalid-front-matter 89→0、
      missing-section 39→150 は schema 適合化で可視化された
      既存不足。coordinator 承認済み解釈)。
- [x] 影響 dir の管理 index 再生成済み。
- [x] review-protocol で blocker なし(A/B 分離レビュー + Validator +
      Judge、CONFIRMED P0/P1 なし — ACCEPT WITH NON-BLOCKING NOTES)。

## Verification

実行環境: Windows (MINGW64) / node v26.12.1(repo 同梱 script を
直接 `node` で実行。mise task なし)

- [x] `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js
      index plan --dir docs/impl/ir --json` → `sync`
      (changes 125、rejected でない)
- [x] `index apply --dir docs/impl/ir --expect
      5048645c321db509667522413afde2e1bf30eabc74b059c4a16a4b4c16cfd8a8
      --json` → 成功、直後の `index plan` → `in-sync`
- [x] `index plan --dir docs/impl/exp --json` → `in-sync`
      (changes 0)、`index apply --dir docs/impl/exp --expect
      79ea5838b5d9785597bfead45f0dc6f82dafb5197854945ca453cfe42dc1ee3d
      --json` → 成功
- [x] `index apply --json`(引数なし)→ exit 0、
      `written: []`、`blocked: []`
- [x] `audit_impl_record.js --json` → error 150
      (invalid-front-matter 0、missing-section 150)。
      ベースライン 128 errors から invalid-front-matter 89 が
      解消、missing-section が 39→150 に増加(schema 適合化で
      可視化された既存本文不足。対象ファイルと不足 section は
      PR 本文「未対応の作業」に列挙)
- [x] `audit_experiment_log.js --json` → findings 0
- [x] `audit_docs.js --type all --json` → 108 findings
      (57 errors)。docs/impl は対象外のため本変更の影響なし。
      新規 doc チェーン追加後に再計測して比較する

未検証項目: 製品コード・CI の検証は対象外(変更は docs/ のみ)。
実 GitHub 上の表示確認は PR 作成後。
