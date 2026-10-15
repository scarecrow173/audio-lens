---
id: "IMPL-034GthYdfmU9PeQTdLm6keAb"
type: "impl"
status: "completed"
title: "Migrate relaxed-tier call sites to atomic writes"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-media/src/stage.rs"
      - type: "code"
        path: "crates/al-locate/src/normalize.rs"
      - type: "code"
        path: "crates/al-dataset/src/table.rs"
      - type: "code"
        path: "crates/al-dataset/src/tensor.rs"
      - type: "code"
        path: "crates/al-dataset/src/audio.rs"
      - type: "code"
        path: "crates/al-dataset/src/stage.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/relaxed-tier-callsite-migration.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/artifact-durability-tiers-resume-verification.md"
    - "docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md"
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

# Migrate relaxed-tier call sites to atomic writes

## Summary

再生成可能な bulk 成果物の書込みを `write_atomic` / `rename_atomic`
(non-durable atomic)へ移行し、manifest・store メタデータ・記録系
JSON は `write_durable` を維持した。resume 検証用に単一ファイル
artifact へ `params.byte_len`、dir/store artifact へ `params.files`
を記録する。

## Context

- task: `docs/tasks/relaxed-tier-callsite-migration.md`
- design: `docs/designs/artifact-durability-tiers-resume-verification.md`
- ADR: `docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md`

## Implementation

### Changes

- `al-media/stage.rs`: `decoded.f64le` を `write_atomic` に移行、
  `decoded_pcm` レコードへ `byte_len` を記録。metadata/video JSON は
  durable 維持。
- `al-locate/normalize.rs`: normalized PCM を `write_atomic` に移行。
  `NormalizedParams` に `byte_len` を正規フィールドとして追加し、
  `validate()` で `frames * channels * 8` との一致を検査、
  `prepare_input()` で dedup lookup 前に設定。
- `al-dataset/table.rs`: table chunk (`NNNNNN.bin`) を
  `write_atomic` に移行。`schema.json` / `index.json` は durable 維持。
- `al-dataset/tensor.rs`: tensor chunk を `write_atomic` に移行。
  `array.json` は durable 維持。
- `al-dataset/audio.rs`: WAV payload ストリームの `sync_all` を除去し
  `rename_atomic` で確定。
- `al-dataset/stage.rs`: `collect_files` で store root の再帰ファイル
  列挙(`/` 正規化・ソート)を `params.files` として記録。

### Decisions During Implementation

- `byte_len` を `NormalizedParams` の外側に注入すると
  `decode.rs` が `result.params["normalization"]` と artifact params を
  直接比較する経路で不一致になるため、構造体の正規フィールドとして
  定義した(表現ドリフト回避)。これにより `pair_evidence_*` の
  座標整合性テストで検出された回帰を修正。

## Related Experiments

### Adopted

- なし。

### Rejected

- なし。

## Validation

- `cargo test --workspace` — 42 バイナリ全件 pass、0 失敗。
- `cargo clippy --workspace --all-targets -D warnings` clean。
- `cargo fmt --all -- --check` pass。

## Risks

- relaxed 成果物の停電ロストは意図された挙動。resume の artifact
  検証が検出して stage を再生成する。

## Follow-ups

- なし。
