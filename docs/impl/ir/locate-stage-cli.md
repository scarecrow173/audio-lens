---
id: "IMPL-034z8Dg25tmGhA8pXvBQhj"
type: "impl"
status: "completed"
title: "locate-stage-cli"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/locate-stage-cli.md"
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

# locate-stage-cli

## Summary

`LocateStage` を実装し `al-cli` に接続。`manifest.inputs[].role` で
`source_asset` × `capture_bug`/`capture_reference` をペアリングし、
ingest の `decoded.f64le` + `metadata.json` から ch 数・fs・
sample_format を復元して ch 単位で定位する。fs 不一致ペアは
リサンプルせず `DecodeInputUnavailable` で明示マーク。

## Implementation

### Changes

- `crates/al-locate/src/stage.rs`: `LocateStage` / `LocateConfig`
- `crates/al-cli/src/main.rs`: `StageKind::Locate` 配線 + USAGE 更新
- `crates/al-cli/Cargo.toml`: `al-locate` 依存追加

### Decisions During Implementation

- 定位証拠は ch ペアごとに収集し、手法ごとに最良ペアを投票代表にし
  全候補をマージ(ch 対応は mixer 推定に委譲)
- decoded PCM はソースネイティブスケールなので `sample_format` で
  float 域に正規化してからメソッドへ渡す
- 受理時は一致窓で mixer 推定 + 全出現の loop occurrence / time_map を
  `derived` として result.json に同梱
- 複数の正当な出現は手法候補の残クラスタとして `occurrences` に記録

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-stage-cli.md`、plan
`docs/plans/locate-and-tracking.md`。commit `1a742a6`。

## Validation

- `al run --stages ingest,locate` で locate_result artifact が
  manifest に登録される(CLI 結合テストで検証)

## Risks

- なし

## Follow-ups

- なし
