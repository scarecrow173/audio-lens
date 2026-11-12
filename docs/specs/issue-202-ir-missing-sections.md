---
id: "SPEC-034WuVtJ4jL3z3QSJ5v5RB"
type: "spec"
status: "approved"
title: "Issue #202 docs/impl/ir missing-section 本文補完"
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
  implements: []
  implemented-by:
    - "docs/designs/issue-202-ir-missing-sections-design.md"
    - "docs/plans/issue-202-ir-missing-sections-plan.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
  refines: []
  refined-by: []
  derives-from:
    - "https://github.com/scarecrow173/audio-lens/issues/202"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-202-ir-missing-sections.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #202 docs/impl/ir missing-section 本文補完

## What Should Be Built

Issue #199 で `docs/impl/ir` の front matter が schema 適合化され、
`audit_impl_record.js` の必須セクション検査が活性化した結果、
既存 25 文書で `missing-section` error が 150 件可視化された。
本 spec はこれらの文書本文に、監査が要求する 7 つの必須
`##` セクション(Summary / Context / Implementation /
Related Experiments / Validation / Risks / Follow-ups)を、
各文書の実装・検証内容に基づく具体的な本文で補完することを
定める。

## Why Now

- `audit_impl_record.js --json` の `missing-section` が 150 件
  残っており、ir 文書としての監査が完了しない。
- 各文書の本文自体は実装当時の記録を保持しているが、旧形式
  (日本語見出し・セクション欠落)のままであり、必須セクション
  への再構成が必要。
- 補完は「実態に基づく本文」でなければならず、空見出し・TODO・
  定型文の羅列は受入条件で明示的に禁止されている。

## Users and Value

- **文書監査を行う開発者/agent**: `missing-section` error が
  0 件になり、ir 文書の必須構造が一貫する。
- **実装記録の読者**: Summary→Context→Implementation→
  Validation→Risks→Follow-ups の標準的な読み順で各記録を
  把握できる。
- **レビュアー**: 追加本文が各文書の実際の変更・検証内容と
  対応していることを差分で確認できる。

## Scope

### In Scope

- `docs/impl/ir/` 配下で `missing-section` を持つ 25 文書の
  本文セクション補完(必須 7 セクションの見出し追加・既存本文の
  再配置を含む)。
- 各文書の front matter(id/status/relations 等)とファイル名は
  不変。
- 本 spec + design/plan/task/test-spec チェーンの作成と、
  影響を受ける管理 index の正規手順での再生成。

### Out of Scope

- 各 ir 文書の front matter・ファイル名の変更。
- `docs/impl/exp/`・`docs/impl/ir` 以外の docs dir の本文変更
  (管理 index の再生成を除く)。
- 製品コード・CI・依存定義の変更。
- `missing-experiment-link` 等 `missing-section` 以外の audit
  error code の修正(現状 20 件を超えないことのみ要求)。
- 本タスク自体の Implementation Record 新規作成(編集範囲は
  既存 ir 文書の本文補完に限定)。

## Requirements

- R1: `audit_impl_record.js --json` の `missing-section` が
  0 件になる。
- R2: 追加した全セクションに、対象文書の実装・検証内容に基づく
  具体的な本文がある(空見出し・TODO のみ・定型文の羅列でない)。
- R3: 各文書の front matter(id/status/relations を含む)と
  ファイル名が変わっていない。
- R4: `missing-section` 以外の error code の件数が現状値
  (missing-experiment-link 20)を超えない。
- R5: `doc_maintenance.js index plan --dir docs/impl/ir` が
  `in-sync`。drift が出た場合は `index apply` で再生成して
  同じ PR に含める。
- R6: 本タスクの doc チェーン(spec/design/plan/task/test-spec)
  を作成し、影響 dir の管理 index を正規手順で再生成する。

## Acceptance Criteria

- [ ] `node .agents/skills/impl-doc/scripts/audit_impl_record.js
      --json` の `missing-section` が 0 件。
- [ ] 追加した全セクションに対象文書の実装・検証内容に基づく
      具体的な本文がある。
- [ ] 各文書の front matter とファイル名が不変。
- [ ] `missing-section` 以外の error code 件数が現状値
      (missing-experiment-link 20)を超えない。
- [ ] `doc_maintenance.js index plan --dir docs/impl/ir` が
      `in-sync`。

## Deferred Design Concerns

- `missing-experiment-link` 20 件 | front matter の
  `metadata.experiments` が指す exp ファイル不在であり、
  本タスクの編集範囲(front matter 不変)では修正不能 | 別 Issue
  で front matter 側の参照整備を行う | audit warning が残る

## Sources and Relations

- Issue [#202](https://github.com/scarecrow173/audio-lens/issues/202):
  本変更の起点。前提として Issue #199(PR #200)の front matter
  schema 適合化が統合済みであること。
- Issue [#199](https://github.com/scarecrow173/audio-lens/issues/199):
  schema 適合化により missing-section が可視化された経緯。
  本 spec はその「別 Issue」としての本文補完を担う。
- `impl-doc` Skill の `audit_impl_record.js` が必須セクションの
  正本。`impl-conventions.md` の監査対象 `##` 見出し一覧
  (Summary/Context/Implementation/Related Experiments/Validation/
  Risks/Follow-ups)に従う。
- [issue-202-ir-missing-sections-design](../designs/issue-202-ir-missing-sections-design.md):
  補完方針(既存本文の再配置 + 不足分の追記)の設計。
