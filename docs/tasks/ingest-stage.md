---
id: "TASK-034z0J84S1ChoUOODLF67z"
type: "task"
status: "todo"
title: "IngestStage 実装と CLI 接続"
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
    - "docs/tasks/wav-decode.md"
    - "docs/tasks/timing-observation.md"
    - "docs/tasks/artifact-dictionary.md"
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
    - "docs/test-specs/ingest-wav-decode.md"
  references: []
  defers: []
  deferred-by: []
---

# IngestStage 実装と CLI 接続

## Work

`IngestStage`(Stage trait 実装)を作成し、入力ファイルの検出・
デコード・メタデータ・ハッシュ・辞書注釈を成果物として manifest に
登録する。`al-cli` の Ingest スタブをこの実装に差し替える。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ingest-av.md`
- `docs/designs/ingest-av.md`
- `docs/test-specs/ingest-wav-decode.md`

## Dependencies

- `docs/tasks/wav-decode.md`
- `docs/tasks/timing-observation.md`
- `docs/tasks/artifact-dictionary.md`

## Affected Files

- `crates/al-media/src/stage.rs`
- `crates/al-cli/src/main.rs`
- `crates/al-cli/Cargo.toml`(al-media 依存)

## Done When

- [ ] `al run --stages ingest` で実 WAV が ingest され成果物が
  manifest に記録される
- [ ] 他ステージのスタブ動作を維持したまま Ingest のみ差し替わる
- [ ] resume が Ingest 成果物を再利用する

## Verification

- [ ] `cargo test --workspace` が通る
- [ ] CLI で run → resume → inspect が動作する
