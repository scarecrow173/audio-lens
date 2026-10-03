---
id: "TASK-034z9oJ2ngcyfGSgbNPcrX"
type: "task"
status: "done"
title: "viz/export 結合 + ゴールデンテスト"
created: "2026-10-02"
updated: "2026-10-03"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/visualization-export.md"
  implemented-by: []
  depends-on:
    - "docs/plans/visualization-export.md"
    - "docs/tasks/export-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/visualization-export.md"
  references: []
  defers: []
  deferred-by: []
---

# viz/export 結合 + ゴールデンテスト

## Work

visualization/export 結合テスト + ゴールデンスナップショット
(test-spec `visualization-export.md` 全項目)。

- dataset のみから visualization 再生成(生 PCM 不使用)
- 決定性: 同一 dataset → 同一バイト
- confidence/validity 保持の確認
- 再配置: パッケージを別 dir に移して読める
- 原メディア不在での可読性
- schema 不一致の明示的失敗
- `al run --stages ingest,locate,cheap_analysis,reference_analysis,
  dataset,visualization,export` E2E
- ゴールデン: 代表ジョブの summary / パッケージ manifest を
  スナップショット比較

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/visualization-export.md`
- `docs/test-specs/visualization-export.md`

## Dependencies

- `docs/tasks/export-package.md`

## Affected Files

- `crates/al-viz/tests/`(新規)
- `crates/al-cli/tests/`(E2E)
- `tests/golden/`(スナップショット)

## Done When

- [ ] test-spec の全項目に対応するテストが緑
- [ ] `cargo test --workspace` / clippy / fmt が clean

## Verification

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
