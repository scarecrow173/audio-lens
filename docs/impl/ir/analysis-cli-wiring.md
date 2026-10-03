---
id: "IMPL-034zk73Z7BkZaZ1UoGFyQI"
type: "impl"
status: "completed"
title: "analysis-cli-wiring"
created: "2026-10-03"
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
    - "docs/tasks/analysis-cli-wiring.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
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

# analysis-cli-wiring

## Summary

3 つの解析ステージ(Cheap/Reference/ExpensiveAnalysisStage)を
`al run` に配線し、`--stages` の kebab-case 名と StageKind の
対応・depends_on の入口検証を確認した。

## Context

task `docs/tasks/analysis-cli-wiring.md`、plan
`docs/plans/analysis-stages.md` に従う。

## Implementation

### Changes

- `crates/al-cli/src/main.rs`:
  `StageKind::CheapAnalysis` / `ReferenceAnalysis` /
  `ExpensiveAnalysis` を各ステージ実装に配線(従来の stub を
  置き換え)。USAGE のステージ一覧に
  `cheap_analysis` / `reference_analysis` /
  `expensive_analysis` を掲載
- `crates/al-cli/Cargo.toml`: `al-analysis` 依存を追加
- `crates/al-cli/tests/cli.rs`: CLI 結合テストを追加 —
  capture-only の `ingest,cheap_analysis,dataset` 完走と
  `features/cheap` 出力確認、`--stages cheap_analysis` の
  依存欠落が入口で拒否されること

### Decisions During Implementation

- depends_on の検証は runner 側の既存機構
  (`validate_plan`: 依存が plan 内で先行しないと拒否)を
  そのまま使い、CLI に独自チェックは増やさない
- `expensive_analysis` は `depends_on = [Ingest,
  CheapAnalysis]` のため
  `ingest,cheap_analysis,expensive_analysis` のような
  reference 抜きの組み合わせが受理される

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-cli`: 6 件全 pass(capture-only の
  cheap+dataset 完走、依存欠落の入口拒否を含む)
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Risks

- `visualization` / `export` は引き続き stub。USAGE には
  載っているが実行は no-op(viz ストリームで実装予定)

## Follow-ups

- `viz-*` / `export-package` タスクで残りステージを配線
