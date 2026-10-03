---
id: "IMPL-034zjc5fvu2Fo7hgCWJa8u"
type: "impl"
status: "completed"
title: "analysis-dataset-foundation"
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
    - "docs/tasks/analysis-dataset-foundation.md"
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

# analysis-dataset-foundation

## Summary

解析系ステージが共有する Analysis Dataset の追記・置換基盤を
実装した。`DatasetWriter::open_or_create`、ストア置換
セマンティクス、`FeatureRecord` の resolution 拡張、
`Confidence` 順序付け、共有入力参照・パス検証、locate 結果の
`locate` ロール物化、および `al-analysis` クレート骨格
(`records` / `session`)を含む。

## Context

task `docs/tasks/analysis-dataset-foundation.md`、plan
`docs/plans/analysis-stages.md`、spec
`docs/specs/analysis-dataset.md` に従う。リリース前のため互換
レイヤ・マイグレーションは作らず、不整合は明示的エラーとする
方針(AGENTS.md 明記)。

## Implementation

### Changes

- `crates/al-dataset/src/writer.rs`: `open_or_create` を追加
  (既存 manifest は schema_version / inputs / config を照合し
  不一致・破損は `InvalidData` で失敗)。`write_table` /
  `write_tensor` / `write_audio` / `write_json` に
  `reset_store` による物理置換を追加
- `crates/al-dataset/src/manifest.rs`: `register` が
  `(role, path)` 一致で既存エントリを置換(リトライでも
  artifact 数が膨張しない)
- `crates/al-dataset/src/types.rs`: `ResolutionClass`
  (sample/fine5ms/mid20ms/broad100ms/wide500ms/seconds/segment)
  を追加し `FeatureRecord.resolution_class` を必須フィールド化
- `crates/al-dataset/src/provenance.rs`: 共有
  `input_references(&Manifest)`(pinned SHA-256 必須、未ピンは
  明示的失敗)
- `crates/al-dataset/src/stage.rs`: `DatasetStage` を追記モード化。
  `analysis/` の破損 manifest は上流解析ステージ完了済みなら
  破壊を拒否し、未完なら再生成。`materialize_locate` で
  `locate_result` JSON を `locate` ロール + `locate/summary`
  テーブルとして物化
- `crates/al-job/src/fsutil.rs`: `checked_job_path` を追加
  (絶対パス・`..` 成分を拒否)。`al-locate::decode` も共有利用に
  統一
- `crates/al-core/src/types.rs`: `Confidence` に `Ord` 導出
  (`Low < Medium < High`、下流は `min` で伝播)
- `crates/al-analysis/`: 新規クレート。`records.rs` に
  `RobustSummary` / `propagate_validity` / `propagate_confidence`
  / `RecordBuilder`、`session.rs` に `open_analysis_dataset`
  (全解析ステージ共通の検証付き open)

### Decisions During Implementation

- 共有入力参照は `al-analysis` ではなく `al-dataset::provenance`
  に配置(所有者規約: DatasetStage も解析ステージも同じ関数を
  呼ぶ)
- 破損 manifest の扱いは「上流解析ステージが Completed なら
  拒否、未完なら再生成」で固定。`open_or_create` 自体は常に
  明示的失敗(呼び出し側がポリシーを決める)
- PSD 正規化は two-sided(`|X|²/(fs·Σw²)`)。全 bin を返す
  設計と一致させるため one-sided の ×2 は採用しない

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test --workspace` 全 pass(al-dataset 結合 10 件含む:
  open_or_create 照合、ストア置換、orphan store 除去、破損
  manifest 拒否/再生成)
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo fmt --all` 適用済み

## Risks

- `materialize_locate` は `locate_result` の JSON 構造に依存。
  スキーマ変更時は両者を同時に更新する必要がある

## Follow-ups

- なし
