---
id: "TASK-034WuVJXa5i7mHx8qvE4zj"
type: "task"
status: "done"
title: "ir missing-section 本文補完(AL#202)"
created: "2026-11-13"
updated: "2026-11-13"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/202"
  changes:
    added:
      - type: "doc"
        path: "docs/specs/issue-202-ir-missing-sections.md"
      - type: "doc"
        path: "docs/designs/issue-202-ir-missing-sections-design.md"
      - type: "doc"
        path: "docs/plans/issue-202-ir-missing-sections-plan.md"
      - type: "doc"
        path: "docs/tasks/issue-202-ir-missing-sections.md"
      - type: "doc"
        path: "docs/test-specs/issue-202-ir-missing-sections.md"
    modified:
      - type: "doc"
        path: "docs/impl/ir/audiolens-independent-ci-issue-142.md"
      - type: "doc"
        path: "docs/impl/ir/gui-ruviform-controls-issue-131.md"
      - type: "doc"
        path: "docs/impl/ir/gui-ruviform-viz-issue-132.md"
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
        path: "docs/impl/ir/issue-76-anchor-hierarchical-fallback.md"
      - type: "doc"
        path: "docs/impl/ir/native-desktop-initial-slices.md"
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
        path: "docs/impl/ir/native-gui-workspace-tabs-selection.md"
      - type: "doc"
        path: "docs/impl/ir/perf-reeval-2026-10.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-lockfile-source-of-truth.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-logging-span-events-compat.md"
      - type: "doc"
        path: "docs/impl/ir/ruviform-mixed-radix-pin-golden-regen-issue-133.md"
      - type: "doc"
        path: "docs/impl/ir/spectrogram-tab-hang.md"
      - type: "doc"
        path: "docs/impl/ir/webview-host-api-migration.md"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "doc"
        path: "docs/impl/ir/README.md"
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
    - "docs/plans/issue-202-ir-missing-sections-plan.md"
  implemented-by: []
  depends-on:
    - "docs/plans/issue-202-ir-missing-sections-plan.md"
    - "docs/designs/issue-202-ir-missing-sections-design.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-202-ir-missing-sections-plan.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-202-ir-missing-sections.md"
  references: []
  defers: []
  deferred-by: []
---

# ir missing-section 本文補完(AL#202)

## Work

`docs/impl/ir/` の missing-section 対象 25 文書について、監査
必須の 7 セクション(Summary/Context/Implementation/Related
Experiments/Validation/Risks/Follow-ups)を、各文書の実装・検証
内容に基づく具体的な本文で補完する。既存の日本語見出し本文は
canonical `##` 見出しへの再配置・改名で活かし、front matter・
ファイル名は変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/issue-202-ir-missing-sections-plan.md`
- `docs/designs/issue-202-ir-missing-sections-design.md`
- `docs/specs/issue-202-ir-missing-sections.md`
- `docs/test-specs/issue-202-ir-missing-sections.md`

## Dependencies

- `impl-doc` Skill の `audit_impl_record.js`・`impl-conventions.md`
  が必須セクションの正本。
- `doc-maintenance` Skill の `doc_maintenance.js index` が管理
  index 再生成の正本手順。
- なし(単一 worktree 内の一連編集)。

## Affected Files

- `docs/impl/ir/*.md`(25 ファイル、本文のみ)。
- `docs/{specs,designs,plans,tasks,test-specs}/issue-202-*.md`
  (doc チェーン 5 ファイル、新規)。
- `docs/{specs,designs,plans,tasks,test-specs}/README.md`、
  必要時 `docs/impl/ir/README.md`(生成 index)。

## Done When

- [x] `audit_impl_record.js --json` の missing-section が 0 件。
- [x] 追加した全セクションに対象文書の実装・検証内容に基づく
      具体的な本文がある。
- [x] 各文書の front matter(id/status/relations/updated 含む)
      とファイル名が不変。
- [x] missing-section 以外の error code 件数が現状値
      (missing-experiment-link 20)を超えない。
- [x] `doc_maintenance.js index plan --dir docs/impl/ir` が
      `in-sync`。
- [x] review-protocol で blocker なし。

## Verification

実行環境: Windows (MINGW64) / node v26.12.1(repo 同梱 script を
直接 `node` で実行。mise task なし)

- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js
      --json` → missing-section 150→0、missing-experiment-link
      20(不変)、他 error code 増加なし。
- [x] `node .agents/skills/doc-maintenance/scripts/
      doc_maintenance.js index plan --dir docs/impl/ir` →
      `in-sync`。
- [x] `git diff` で 25 文書の front matter・ファイル名が不変。
- [x] review-protocol レビューで blocker なし。

未検証項目: 製品コード・CI の検証は対象外(変更は docs/ のみ)。
実 GitHub 上の表示確認は PR 作成後。
