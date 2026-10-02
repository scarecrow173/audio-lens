---
id: "IMPL-034z12Mzh3B8moSY9E3Dtt"
type: "impl"
status: "completed"
title: "ingest integration tests"
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
    - "docs/tasks/ingest-integration-tests.md"
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

# ingest integration tests

## Summary

合成 WAV fixture + 実 runner を使った ingest 結合テスト 5 件を
`crates/al-media/tests/ingest.rs` に実装した。

## Context

task `docs/tasks/ingest-integration-tests.md`、test-spec
`ingest-wav-decode` / `av-timing-observation` に従う。

## Implementation

### Changes

- WAV 生成 → run → manifest/成果物ファイル検証(PCM サイズ・
  codec・hash・注釈分離)
- 同一入力で同一 source_hash の再現性
- 未知形式が `decode/input unavailable` で Failed 記録
- resume が完了済み ingest を再実行しない(プローブステージで観測)
- VFR 系列の観測が辞書注釈と別フィールドに記録される

### Decisions During Implementation

- fixture は全てプログラム生成(バイナリ fixture をリポジトリに
  置かない)

## Validation

- `cargo test -p al-media`: 20 unit + 5 integration pass
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean

## Follow-ups

- 実コンテナ対応後は実ファイル golden test を `fixtures/` に追加

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Risks

- なし
