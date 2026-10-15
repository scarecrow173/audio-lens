---
id: "IMPL-034Gtg2ZZrjJ8phrjjizfx"
type: "impl"
status: "completed"
title: "Add atomic write primitives to fsutil"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-job/src/fsutil.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/fsutil-atomic-write-primitives.md"
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

# Add atomic write primitives to fsutil

## Summary

`fsutil` に non-durable な atomic 書込みプリミティブ `write_atomic` /
`rename_atomic` を追加した。temp ファイルへの書込み + rename による
原子性(読者は旧内容か新内容の完全などちらかを見る)は維持しつつ、
`sync_all` / 親ディレクトリ sync を省略して耐久性を意図的に緩和する
relaxed 等級の基盤。

## Context

- task: `docs/tasks/fsutil-atomic-write-primitives.md`
- design: `docs/designs/artifact-durability-tiers-resume-verification.md`
- ADR: `docs/adr/artifact-durability-tiers-relaxed-bulk-writes.md`
- spec: `docs/specs/durability-tiered-artifact-writes.md`

## Implementation

### Changes

- `write_atomic(path, bytes)`: `<name>.tmp` への書込み → rename。
  `write_durable` と同じ temp-file cleanup / 置換セマンティクスを持つが、
  ファイル `sync_all` と親ディレクトリ `sync_all` を行わない。
- `rename_atomic(tmp, path)`: rename のみ。`al-dataset` の WAV 出力が
  既にストリーム書込みしている temp を確定させる用途。
- `write_durable` / `rename_durable` は不変。manifest・記録系 JSON は
  引き続き durable 経路を使う。
- `fsutil` のユニットテストで atomic 書込み・上書き・temp 残存なしを
  検証。

### Decisions During Implementation

- 失敗時の temp cleanup を `write_durable` と同一経路に揃え、
  atomic/耐用で振る舞いが分岐しないようにした。

## Related Experiments

### Adopted

- なし。

### Rejected

- なし。

## Validation

- `cargo test -p al-job` — fsutil 関連テストを含め全件 pass。
- `cargo clippy --workspace --all-targets -D warnings` clean。

## Risks

- 停電・OS クラッシュ時に relaxed 成果物が消失しうる。これは
  ADR で意図的に受け入れたトレードオフで、resume 側の artifact
  検証(TASK-034GsRMzsYQltCJEffwBfb)が補償する。

## Follow-ups

- relaxed 呼出点の移行は TASK-034GsRN1DDq3D5ZJHgcq6d。
