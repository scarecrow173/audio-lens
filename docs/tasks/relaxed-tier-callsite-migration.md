---
id: "TASK-034GsRN1DDq3D5ZJHgcq6d"
type: "task"
status: "done"
title: "relaxed-tier-callsite-migration"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/plans/implement-artifact-durability-tiers.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-artifact-durability-tiers.md"
    - "docs/tasks/fsutil-atomic-write-primitives.md"
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
    - "docs/test-specs/durability-tiers-resume-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# relaxed-tier-callsite-migration

## Work

bulk データ書込を relaxed 経路へ切り替え、producer が params に 検証メタを記録する。decoded.f64le(al-media)、normalized PCM (al-locate)、table/tensor chunk(al-dataset)、WAV payload (al-dataset/audio.rs)を write_atomic/rename_atomic に。 analysis_dataset record に params.files 列挙を追加。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-artifact-durability-tiers.md`
- `docs/designs/artifact-durability-tiers-resume-verification.md`

## Dependencies

- `docs/tasks/fsutil-atomic-write-primitives.md`(fsutil API)

## Affected Files

- `crates/al-media/src/stage.rs`
- `crates/al-locate/src/normalize.rs`
- `crates/al-dataset/src/{table,tensor,audio,stage}.rs`

## Done When

- [ ] relaxed 呼出点が `write_atomic`/`rename_atomic` を使う
- [ ] 記録系が `write_durable`/`rename_durable` を維持する
- [ ] decoded_pcm/normalized_pcm に byte_len、analysis_dataset に
      files が params へ記録される

## Verification

- [ ] `cargo test --workspace` がパスする
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `cargo fmt --all -- --check` clean
