---
id: "IMPL-034yZRlHVYt3tuCopmkzRI"
type: "impl"
status: "completed"
title: "minimal cli"
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
    - "docs/tasks/minimal-cli.md"
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

# minimal cli

## Summary

`al` バイナリ(`al-cli`)に `run` / `resume` / `inspect` の 3 サブコマンドを
実装し、ジョブ基盤を CLI から end-to-end で動かせるようにした。

## Context

task `docs/tasks/minimal-cli.md`、design `job-orchestration` に従う。

## Implementation

### Changes

- `crates/al-cli/src/main.rs`: 引数を手書きパースし、依存追加を避けた。
- `run`: `--base-dir` / `--input role:path,...` / `--stages` /
  `--config <file>` を受け、`JobDefinition::create` で不変ジョブを発行して
  `StageRunner::run` を実行し、`job_id` と `job_dir` を出力。
- `resume` / `inspect`: `--job-id` から manifest を読み、
  前者は `StageRunner::resume`、後者は manifest JSON を標準出力へ。
- ステージ実装は全種共通のスタブ(marker 成果物を1件発行)。実ステージは
  後続プランで差し替える。

### Decisions During Implementation

- resume は manifest の `inputs` / `config` / `stages` から
  `JobDefinition` を再構成する方式とし、ジョブ定義ファイルの別途保存を
  不要にした。
- CLI 引数パースに外部クレート(clap 等)を入れず、基盤段階の依存を
  `al-core` / `al-job` / `serde_json` に留めた。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `al run --base-dir <dir> --input capture_bug:cap.mkv,source_asset:src.wav --stages ingest,locate` でジョブ作成を確認。
- `al resume --job-id <id>` が no-op で Completed を表示することを確認。
- `al inspect --job-id <id>` が manifest JSON を出力することを確認。
- `cargo build` / `cargo test --workspace` / `clippy -D warnings` /
  `cargo fmt --check` が全て通過。

## Risks

- ステージはスタブのため実データは生成されない。実装後に差し替える前提。

## Follow-ups

- 実ステージ実装時に `StubStage` を廃止し、実装登録テーブルに置き換える。
