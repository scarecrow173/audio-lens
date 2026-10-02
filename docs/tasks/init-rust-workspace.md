---
id: "TASK-cwUga2iqYje7mVg950IY46"
type: "task"
status: "done"
title: "Rust ワークスペースの初期化"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
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
    - "docs/plans/implement-job-orchestration-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-job-orchestration-foundation.md"
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

# Rust ワークスペースの初期化

## Work

Rust ワークスペースを初期化し、`crates/` 配下の空 crate 群と
mise task(build / test / lint / fmt)を整備する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-job-orchestration-foundation.md`
- `docs/adr/implementation-stack.md`

## Dependencies

- なし(最初のタスク)

## Affected Files

- `Cargo.toml`(ワークスペース定義)
- `crates/al-core/Cargo.toml`・`crates/al-job/Cargo.toml`・
  `crates/al-cli/Cargo.toml`
- `mise.toml`(rust toolchain と task 定義)

## Done When

- [ ] workspace メンバーとして `al-core`・`al-job`・`al-cli` が存在する
- [ ] `mise exec -- cargo build` が成功する
- [ ] `mise run` で test / lint / fmt が実行できる

## Verification

- [ ] `mise exec -- cargo build --workspace` が通る
- [ ] `mise exec -- cargo test --workspace` が通る(空テスト含む)
