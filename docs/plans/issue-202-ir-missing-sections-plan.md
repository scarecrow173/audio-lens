---
id: "PLAN-034WuVer3DEk2J0wDG2Yn2"
type: "plan"
status: "approved"
title: "Issue #202 ir missing-section 補完 実装計画"
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
    - "docs/specs/issue-202-ir-missing-sections.md"
    - "docs/designs/issue-202-ir-missing-sections-design.md"
  implemented-by:
    - "docs/tasks/issue-202-ir-missing-sections.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/199"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-202-ir-missing-sections.md"
    - "docs/designs/issue-202-ir-missing-sections-design.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-202-ir-missing-sections.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #202 ir missing-section 補完 実装計画 Implementation Plan

## Goal

`docs/impl/ir/` の missing-section 対象 25 文書に、監査必須の
7 セクション(Summary/Context/Implementation/Related Experiments/
Validation/Risks/Follow-ups)を実内容で補完し、
`audit_impl_record.js` の missing-section を 0 件にする。

## Scope

単一の実装ストリーム(25 文書の本文セクション補完 + doc チェーン
同期 + index 再生成確認)。文書は独立しており、内容の依存関係は
ないためファイル単位で並列に進められる。

## Upstream Documents

- [docs/specs/issue-202-ir-missing-sections.md](../specs/issue-202-ir-missing-sections.md)
- Issue [#202](https://github.com/scarecrow173/audio-lens/issues/202)

## Design Inputs

- [docs/designs/issue-202-ir-missing-sections-design.md](../designs/issue-202-ir-missing-sections-design.md)
  (approved)— 必須セクションと内容源の対応表・再配置ルール・
  3 類型(残り少数/一部欠落/全欠落)の分類。

## File Map

- `docs/impl/ir/*.md`(25 ファイル、本文のみ)— 必須セクション
  補完。front matter・ファイル名は不変。
  - 一部欠落(8): audiolens-independent-ci-issue-142,
    gui-ruviform-controls-issue-131, gui-ruviform-viz-issue-132,
    issue-76-anchor-hierarchical-fallback, perf-reeval-2026-10,
    ruviform-lockfile-source-of-truth,
    ruviform-logging-span-events-compat,
    ruviform-mixed-radix-pin-golden-regen-issue-133
  - 全欠落(17): impact-based-ci-macos-separation-al-149,
    issue-101-ruviform-numeric-migration,
    issue-102-domain-separation,
    issue-103-job-dataset-provenance-regression, issue-105-gui-app,
    issue-106-viz-removal, native-desktop-initial-slices,
    native-gui-drawn-parity-run-mount,
    native-gui-job-receipt-consumption, native-gui-job-run-mount,
    native-gui-product-screen-mount,
    native-gui-spectrogram-analysis-projection,
    native-gui-tabs-selection, native-gui-waveform-seek-screen,
    native-gui-workspace-tabs-selection, spectrogram-tab-hang,
    webview-host-api-migration
- `docs/specs|designs|plans|tasks|test-specs/issue-202-*.md` —
  本 doc チェーン(新規)。
- `docs/{specs,designs,plans,tasks,test-specs}/README.md` —
  影響 dir の管理 index 再生成(生成物)。
- `docs/impl/ir/README.md` — drift 時のみ `index apply` で再生成。

## Implementation Sequence

- [x] `audit_impl_record.js --json` でベースライン記録
      (missing-section 150、missing-experiment-link 20)。
- [x] doc チェーン(spec/design/plan/task/test-spec)を作成。
- [x] 25 文書の本文セクションを補完(文書ごとに既存本文を読み、
      canonical 見出しへ再配置 + 不足分を実内容で追記)。
- [x] `audit_impl_record.js --json` で missing-section 0 件・
      他 error code 非増加を確認。
- [x] `git diff` で front matter・ファイル名不変を確認。
- [x] 影響 dir の管理 index を `doc_maintenance.js index` で
      再生成し in-sync を確認。
- [x] review-protocol レビューで blocker がないことを確認。
- [ ] origin/main 取込 → Conventional Commits で論理分割コミット
      → push → 日本語 PR。

## Dependencies and Risks

- audit の見出し判定は `^##\s+<canonical>\s*$` の完全一致で
  日本語 alias なし。見出しは必ず canonical 英語名を使う。
- 既存本文の再配置で内容を失うリスク → 見出しレベル・名称の
  変更に留め、本文行は保持する。diff で確認。
- Related Experiments は実在する `docs/impl/exp/` ログと
  front matter `metadata.experiments` のみを根拠にする。
  dangling 参照(20 件)は front matter 側の既存問題で対象外。
- index plan が本文変更で drift する可能性 → 仕様上 source は
  front matter 由来のため in-sync の見込み。drift 時は
  `index apply` で再生成し同 PR に含める。

## Task Breakdown

- [docs/tasks/issue-202-ir-missing-sections.md](../tasks/issue-202-ir-missing-sections.md)
  — 単一 task。コミットは論理境界(doc チェーン / ir 本文補完 /
  index 再生成)で分割。

## Verification

- [x] `node .agents/skills/impl-doc/scripts/audit_impl_record.js
      --json` → missing-section 0、missing-experiment-link ≤ 20、
      その他 error code がベースライン(150/20)から増えない。
- [x] `node .agents/skills/doc-maintenance/scripts/
      doc_maintenance.js index plan --dir docs/impl/ir` →
      `in-sync`(drift 時は apply 後に再確認)。
- [x] `git diff` で 25 文書の front matter・ファイル名が不変。
- [x] review-protocol レビューで blocker なし。

## Open Questions / Gaps

- `missing-experiment-link` 20 件は front matter の dangling
  参照であり本タスク範囲外。別 Issue 化を PR 本文に記録する。
- 本タスク自体の Implementation Record は編集範囲外
  (既存 ir 文書の本文補完のみ)のため作成しない。PR 本文に
  明記する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(本 worker が直接実装、
  文書単位で分割して進める)。
- Independent work streams: 25 文書は相互独立だが同一 dir の
  ため、編集は逐次または disjoint なファイル群で行う。
- User approval required before dispatch: no — worker-guide により
  review-protocol 独立レビューを承認代替とする。

## Review Handoff

- reviewer 注目点: (a) 追加本文が各文書固有の事実に基づくこと
  (定型文でないこと)、(b) front matter・ファイル名の不変、
  (c) 既存本文の意味が再配置で失われていないこと、
  (d) audit の missing-section 0 件と他 code 非増加、
  (e) Related Experiments が実在ログのみを参照していること。
