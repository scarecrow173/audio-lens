---
id: "TASK-034z8kJCh6jSerzLdxIwnb"
type: "task"
status: "done"
title: "LocateStage + CLI 接続"
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
    - "docs/plans/locate-and-tracking.md"
  implemented-by: []
  depends-on:
    - "docs/plans/locate-and-tracking.md"
    - "docs/tasks/locate-consensus.md"
    - "docs/tasks/locate-residual.md"
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
---

# LocateStage + CLI 接続

## Work

`LocateStage` を実装し `al-cli` の `Locate` スタブを差し替える。`manifest.inputs[].role` で `source_asset` × `capture_*` をペアリングし、ch 数/fs を metadata から復元。fs 不一致ペアは明示マーク。USAGE を更新。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/locate-and-tracking.md`

## Dependencies

- `docs/tasks/locate-consensus.md`
- `docs/tasks/locate-residual.md`

## Affected Files

- `crates/al-locate/src/stage.rs`
- `crates/al-cli/src/main.rs`(`Locate` 差し替え + USAGE 更新)

## Done When

- [ ] `al run --base-dir <d> --input capture_bug:c.wav --input source_asset:s.wav --stages ingest,locate` で locate 成果物が manifest に記録される
- [ ] fs 不一致ペアが明示的にマークされる
- [ ] USAGE のスタブ記述が更新される

## Verification

- [ ] 上記 CLI コマンドが成功する
- [ ] `cargo test --workspace` パス
