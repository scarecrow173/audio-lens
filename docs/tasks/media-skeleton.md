---
id: "TASK-034z0J5bNO5HoVp8PkGjrg"
type: "task"
status: "todo"
title: "al-media 骨格とフォーマット検出・明示的失敗"
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
    - "docs/plans/implement-ingest-av.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-ingest-av.md"
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

# al-media 骨格とフォーマット検出・明示的失敗

## Work

`crates/al-media` をワークスペースに追加し、フォーマット検出
(magic bytes / 拡張子非依存のシグネチャ判定)と、未対応形式を
`decode/input unavailable` で明示的失敗させるエラー型を実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ingest-av.md`
- `docs/specs/media-ingest.md`

## Dependencies

- なし(先行タスクなし)

## Affected Files

- `Cargo.toml`(workspace members)
- `crates/al-media/Cargo.toml`
- `crates/al-media/src/lib.rs`
- `crates/al-media/src/format.rs`

## Done When

- [ ] `al-media` がワークスペースの一員としてビルドされる
- [ ] WAV シグネチャと未知形式が区別される
- [ ] 未知形式で `decode/input unavailable` が返る

## Verification

- [ ] `cargo test -p al-media` が通る
