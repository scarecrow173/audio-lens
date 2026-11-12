---
id: "DESIGN-034WuVuaQKt33yJv9vseJb"
type: "design"
status: "approved"
title: "Issue #202 ir missing-section 補完設計"
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
  implemented-by:
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
    - "docs/specs/issue-202-ir-missing-sections.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/issue-202-ir-missing-sections.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue #202 ir missing-section 補完設計

## Context

`impl-doc` の `audit_impl_record.js` は各 ir 文書に 7 つの必須
`##` セクションを要求する: Summary / Context / Implementation /
Related Experiments / Validation / Risks / Follow-ups。見出しは
`^##\s+<canonical>\s*$`(case-insensitive)の完全一致で、
impl-doc 向けの日本語 alias は存在しない。

Issue #199 で 18 ファイルの front matter が schema 適合化され
audit 検査が活性化した結果、既存文書の多くが旧形式(日本語
見出し主体・必須セクション欠落)のまま残り、missing-section
error が 150 件可視化された。本文自体には実装・検証の記録が
残っているため、本タスクは「見出し構造の正規化 + 不足セクション
の実内容追記」で対応する。

対象 25 文書は 3 類型に分かれる:

- **型 A(残り少数)**: `ruviform-lockfile-source-of-truth`
  (Risks のみ欠落)、`ruviform-mixed-radix-pin-golden-regen-
  issue-133`(Risks/Follow-ups 欠落)等、canonical セクションが
  ほぼ揃っている文書。
- **型 B(一部欠落)**: `audiolens-independent-ci-issue-142`、
  `gui-ruviform-*-issue-13x`、`issue-76-*`、`perf-reeval-2026-10`、
  `ruviform-logging-span-events-compat` 等、Summary/Context/
  Implementation はあるが Related Experiments/Validation/Risks/
  Follow-ups が欠ける文書(見出し名の揺れ `## Verification` を
  含む)。
- **型 C(全欠落)**: `issue-10x` 系・`native-*` 系・
  `spectrogram-tab-hang`・`webview-host-api-migration` 等、
  本文は日本語見出し(変更内容/検証/残課題等)で記述され必須
  セクションが 1 つもない文書。

## Scope

### In Scope

- 上記 25 文書の本文セクション補完。既存本文の削除・要約化は
  行わず、canonical `##` 見出しへの再配置・改名と、不足分の
  実内容追記で対応する。
- `### Changes` / `### Decisions During Implementation` 等の
  Implementation 配下構造、Related Experiments の
  Adopted/Rejected 記述は `impl-conventions.md` の慣行に倣う。

### Out of Scope

- front matter・ファイル名・id の変更、本文の意味内容の改変
  (再配置・見出し改名・不足分追記のみ)。
- `docs/impl/exp/` の変更、missing-experiment-link の修正、
  製品コード変更、他 docs dir の本文変更。

## Components and Boundaries

### 1. 必須セクションと内容源

| セクション | 内容源(各文書内) |
| --- | --- |
| Summary | H1 直下の導入段落・文書の目的記述 |
| Context | 上流文書 relations・Issue 参照・変更の背景記述 |
| Implementation | 変更内容/実装済み/構築した構成 等の既存本文を `### Changes`/`### Decisions During Implementation` に再配置 |
| Related Experiments | `metadata.experiments`(参照のみ)と `docs/impl/exp/` の関連ログ。存在しない場合はその旨と理由を明記 |
| Validation | 検証/検証結果/検証記録/実 run 観測 等の既存本文 |
| Risks | 残課題・制限・リスク・判断記録 等から抽出。なければ文書記述に基づく実リスク |
| Follow-ups | 残件・未実施・Blocked・起票案/残課題 等の既存本文 |

### 2. 再配置ルール

- 既存の日本語 `##` 見出しは、対応する canonical セクションへ
  改名するか `###` に降格して親セクション配下へ移す。
  本文テキストは原則保持し、見出しレベルと名称のみ調整する。
- canonical セクション同士の順序は conventions 記載順
  (Summary→Context→Implementation→Related Experiments→
  Validation→Risks→Follow-ups)に揃える。
- 既存の追加 `##` セクション(Golden 差分分析・効果実測等)は
  内容が Implementation/Validation の詳細であれば `###` に
  降格して配下へ収めるか、独立セクションとして残す
  (audit は欠落のみ検査するため残置も可。ただし読み順を
  損なう場合は再配置する)。
- 新規追記は「その文書が実際に記録している事実」のみに
  限る。文書に根拠がない一般論・推測は書かない。

### 3. 検証

- `audit_impl_record.js --json` で missing-section 0 件、
  他 error code が増えないことを確認。
- `git diff` で front matter・ファイル名の不変を確認。
- `doc_maintenance.js index plan --dir docs/impl/ir` で
  `in-sync`(本文のみの変更なら drift しない想定。drift 時は
  `index apply` で再生成)。

## Data and Control Flow

文書ごとに独立: 各 ir 文書を読み、既存セクションを canonical
構造へ写像し、不足セクションを文書内証拠または `docs/impl/exp/`
の関連ログから補完する。横断的な状態は持たない。

## Non-Functional Constraints

- **意味保存**: 既存本文の事実関係を変えない。補完は構造化と
  不足分の追記であり、歴史的記録の改竄ではない。
- **fail-closed**: audit が要求する見出し完全一致を満たす
  (alias への依存・曖昧な見出しを避ける)。
- **front matter 凍結**: `updated` を含む front matter 全体を
  変更しない(Issue #202 の編集範囲制約。通常の updated 更新
  義務より範囲制約を優先)。

## Risks and Trade-offs

- 25 文書一括の構造変更で既存本文を誤って欠落させるリスク →
  見出し改名・降格のみとし本文行は保持。git diff で行単位の
  消失がないことを確認する。
- 「具体的本文」の判断が主観的になり定型文化するリスク →
  各セクションを当該文書の固有情報(対象ファイル・Issue・
  検証コマンド・実測値)で記述する方針を plan/task に明記し、
  review-protocol で点検する。
- Related Experiments で存在しない exp を参照するリスク →
  `docs/impl/exp/` の実ファイルと front matter の
  `metadata.experiments` を突合し、実在するものだけを記す。
  欠落参照(missing-experiment-link 20 件)は front matter 側の
  既存問題であり本文では実在ログのみ扱う。

## Deferred Design Concerns

- `metadata.experiments` の dangling 参照 20 件 | front matter
  は本タスクの編集範囲外 | 別 Issue で参照整備 | audit
  warning(missing-experiment-link)が残る
- `updated` front matter の更新 | 範囲制約で front matter 凍結
  | 次回の正規編集時に自然更新 | index/鮮度表示が古いまま

## Verification Notes

- [ ] `audit_impl_record.js --json`: missing-section 0 件、
      missing-experiment-link ≤ 20、他 error code 増加なし。
- [ ] `git diff` で 25 文書の front matter・ファイル名が不変。
- [ ] 追加セクション本文が各文書固有の事実を含むことを
      review-protocol で確認。
- [ ] `doc_maintenance.js index plan --dir docs/impl/ir` が
      `in-sync`。
