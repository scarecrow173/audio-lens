---
id: "IMPL-034WsMwd7e8Pcoe1xb5okf"
type: "impl"
status: "completed"
title: "Issue #199 docs/impl/ir・exp 管理 index の doc-maintenance 生成対応"
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
    - "docs/tasks/issue-199-impl-index-generation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/issue-199-impl-index-generation.md"
    - "docs/designs/issue-199-impl-index-generation-design.md"
    - "docs/plans/issue-199-impl-index-generation-plan.md"
    - "docs/test-specs/issue-199-impl-index-generation.md"
    - "https://github.com/scarecrow173/ruviform/issues/586"
    - "https://github.com/scarecrow173/synvelis/issues/365"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/issue-199-impl-index-generation-plan.md"
    - "docs/designs/issue-199-impl-index-generation-design.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Issue #199 docs/impl/ir・exp 管理 index の doc-maintenance 生成対応

## Summary

`docs/impl/ir`・`docs/impl/exp` の管理 index を `doc-maintenance`
の safe sync engine(`sync_impl_index`)で再生成可能にした。
`docs/impl/ir` 配下 18 ファイルの schema 不適合 front matter を
形式修正し、両 `README.md` に generated-index marker を追加して
managed index 化、正規の plan→apply→plan 手順で再生成した。

## Context

Issue #199 の受入条件に基づく docs 保守タスク。
[spec](../../specs/issue-199-impl-index-generation.md) /
[design](../../designs/issue-199-impl-index-generation-design.md) /
[plan](../../plans/issue-199-impl-index-generation-plan.md) /
[task](../../tasks/issue-199-impl-index-generation.md) /
[test-spec](../../test-specs/issue-199-impl-index-generation.md)
のチェーンに従う。`index plan` が `rejected` だった原因は
source 側の front matter 不正(ir)と index の marker 欠落(exp)
であり、renderer の問題ではなかった。

## Implementation

### Changes

- `docs/impl/ir` 18 ファイルの front matter 形式修正:
  - `metadata.experiments.{adopted,rejected}` 補完(7 ファイル)。
  - `relations.changes` の文字列配列を `{type, path}` object 化
    (2 ファイル)。type は path の拡張子・性質から機械的に判定。
  - front matter 不完全・全欠落の補完(9 ファイル)。全欠落の
    7 ファイル(issue-101/102/103/105/106/117・perf-reeval)には
    H1・Issue 参照・日付から `id`(新規 `IMPL-…` 採番)、
    `type: "impl"`、`status: "completed"`、`title`、
    `created`/`updated`、`owners`、`relations`、
    `metadata.experiments` を生成。
  - `spectrogram-tab-hang.md` の不正 status を `completed` に
    正規化。
- `docs/impl/ir/README.md`: 旧手書き `Task | Record` 表を廃し、
  title+説明の preamble を保持したまま generated-index marker +
  `Directory:` + 4 列 managed index(active + `## Settled`)へ
  再生成。
- `docs/impl/exp/README.md`: marker 追加 + 1 列 File 表を
  filename 順で再生成。
- doc チェーン 5 ファイル + 本 IR を新規作成。

### Decisions During Implementation

- `index apply --expect` には `index plan` 出力の**トップレベル**
  planHash を使用(対象別 planHash では hash 不一致で拒否される)。
- ir/README の旧 `Task | Record` 対応表は managed index で置き換え。
  task→file 対応は各文書の `implements` relation と新 index の
  File リンクで代替可能と判断。
- front matter 全欠落 7 ファイルの `created`/`updated` は本文中の
  日付記述と git 履歴から保守的に推定(2026-10-31/2026-11-01)。
  `implements` は対応 task doc が実在する場合のみ設定
  (issue-117・perf-reeval は対応 task なしのため空)。
- audit error 増分(missing-section 39→150)は schema 適合化で
  可視化された既存本文不足であり、coordinator 判断により
  「新規 error なし」として受理。本文補完は別 Issue。

## Related Experiments

### Adopted

- なし(機械的な文書修正タスクのため実験ログは作成していない)。

### Rejected

- なし。

## Validation

実行環境: Windows (MINGW64) / node v26.12.1(repo 同梱 script を
`node` で直接実行。mise task なし)

- `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js
  index plan --dir docs/impl/ir --json` → `sync`(changes 125)。
- `index apply --dir docs/impl/ir --expect
  5048645c321db509667522413afde2e1bf30eabc74b059c4a16a4b4c16cfd8a8
  --json` → 成功。直後の `index plan` → `in-sync`。
- `index plan --dir docs/impl/exp --json` → `in-sync`。
  `index apply --dir docs/impl/exp --expect
  79ea5838b5d9785597bfead45f0dc6f82dafb5197854945ca453cfe42dc1ee3d
  --json` → 成功。
- `index apply --json`(引数なし)→ exit 0、`written: []`、
  `blocked: []`。
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js
  --json` → findings 170(error 150 / warning 20)。
  ベースライン(findings 149 / error 128 / warning 21)との比較:
  invalid-front-matter 89→0、index-not-managed 1→0、
  missing-section 39→150(可視化された既存不足)、
  missing-experiment-link warning 20→20。
- `node .agents/skills/impl-doc/scripts/audit_experiment_log.js
  --json` → findings 0。
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all
  --json` → findings 108(error 57 / warning 51)。docs/impl は
  対象外。新規 doc チェーン追加後の再計測値は task/PR に記録。

## Risks

- missing-section error が残存する → 既存本文不足であり別 Issue
  で補完予定(coordinator 対応)。本タスクの範囲外。
- 新規採番 `IMPL-…` ID の衝突 → 既存 id 全件との重複がないことを
  生成時に確認済み。

## Follow-ups

- 既存 IR 文書の必須セクション補完(coordinator が別 Issue 化)。
- `relations.changes` の type 語彙の正規化(schema に enum が
  導入された場合)。
