---
id: "IMPL-034zk74prmTmaxVxbHhdZR"
type: "impl"
status: "completed"
title: "analysis-integration-tests"
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
    - "docs/tasks/analysis-integration-tests.md"
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

# analysis-integration-tests

## Summary

analysis パイプラインの合成結合テストを整備し、test-spec
`docs/test-specs/analysis-stages.md` の検証項目をテストで
固定した。

## Context

task `docs/tasks/analysis-integration-tests.md`、plan
`docs/plans/analysis-stages.md`、test-spec
`docs/test-specs/analysis-stages.md` に従う。

## Implementation

### Changes

- `crates/al-analysis/tests/reference.rs`(2 件):
  受理 alignment で reference 特徴・R0-R3 残差・dataset
  出力が揃うこと、未受理 alignment が invalid 行で永続化
  されステージが完走すること
- `crates/al-analysis/tests/expensive.rs`(3 件):
  clipping 候補が expensive 区間を起動し cheap 出力を
  上書きしないこと、閾値オーバーレイで抑制された場合に
  events 空テーブルのみ書かれること、always_run が
  reference なしで全入力を解析すること
- `crates/al-cli/tests/cli.rs`(追加 2 件):
  capture-only で `ingest,cheap_analysis,dataset` が完走し
  `features/cheap` に JSON が出ること、`--stages
  cheap_analysis` の依存欠落が入口で拒否されること
- 既存の dataset 結合テスト群(`open_or_create` 不一致・
  破損 manifest の明示失敗、store 置換の冪等性)が追記
  プロトコルと resume 保証をカバー

### Decisions During Implementation

- 「クリーン信号 = イベントゼロ」のテストは、広帯域信号が
  noise-floor ヒューリスティックに正当ヒットするため成立
  しない。抑制経路の検証は config オーバーレイで閾値を
  上げる形に変えた(仕様どおりの挙動をテストする)
- ステージ書き込み中失敗→resume の項目は、dataset 層の
  `store_replacement_is_full_and_idempotent` +
  `failed_dataset_stage_is_resumable` と runner 層の
  resume テストで機構が固定済みのため、同一 (role,path) の
  重複登録防止は store 置換テストで担保する

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test --workspace`: 全クレート全テスト pass
  (al-analysis 35 件、al-cli 6 件、al-dataset、al-job、
  al-locate、al-media 含む)
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo fmt --all` 適用済み

## Risks

- 実ゲームキャプチャ規模の性能・スループットは test-spec の
  non-goal どおり未検証

## Follow-ups

- visualization / export ストリームの結合テスト
  (task `viz-export-integration-tests`)
