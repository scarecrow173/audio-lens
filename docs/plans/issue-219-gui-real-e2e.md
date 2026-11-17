---
id: "PLAN-034Z0TN8Ld8Is1tEhV5gVN"
type: "plan"
status: "approved"
title: "GPUI 製品 GUI 実 OS E2E 接続 lane 実装計画"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/specs/issue-219-gui-real-e2e.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-219-gui-real-e2e.md"
    - "docs/designs/issue-219-gui-real-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---
# GPUI 製品 GUI 実 OS E2E 接続 lane 実装計画 Implementation Plan

## Goal

`gpui-real` feature の test-only bin で製品 ViewSpec を実 OS
window に mount し、driver-neutral scenario + 実 OS 入力/pixel
case を JSONL 証拠として記録する opt-in lane を追加する。

## Scope

単一実装 stream。編集範囲は Spec の lease 内
(`crates/al-gui/Cargo.toml`、新規 bin、`tests/scenario_real.rs`、
`mise.toml` 追加のみ、`ci.yml` gui-real note、docs、
`evidence/al219/`)。

## Upstream Documents

- `docs/specs/issue-219-gui-real-e2e.md`

## Design Inputs

- `docs/designs/issue-219-gui-real-e2e.md`

## File Map

- `crates/al-gui/Cargo.toml` - `gpui-real` feature + bin target
- `crates/al-gui/src/bin/al-gui-real-scenario.rs` - lane 実装
- `crates/al-gui/tests/scenario_real.rs` - headless 検証 test
- `mise.toml` - `test-ui-gpui-real` / `test-ui-gpui-real-headless`
- `.github/workflows/ci.yml` - gui-real stale note 更新
- `evidence/al219/*.jsonl` - lane 実走証拠
- `docs/impl/ir|exp/issue-219-*` - 実装記録・実験記録

## Implementation Sequence

- [x] Cargo.toml に `gpui-real` feature + `[[bin]]` 追加
- [x] bin 実装(mount・case catalog・verdict 写像・JSONL・
      blocked-probe・引数処理)
- [x] `scenario_real.rs` integration test 追加
- [x] mise lane 2 本追加
- [x] ci.yml gui-real note 更新
- [x] docs chain + evidence + IR/exp 記録

## Dependencies and Risks

- ruviform pin `cbcc8343` の API が前提(変更なし)。
- `Cargo.lock` 変更不要(全依存 resolved 済み — 実績: 差分なし)。
- 実走 Pass は interactive session 前提 — headless では
  NotRun が正しい結果。

## Task Breakdown

- `docs/tasks/issue-219-gui-real-e2e.md` に集約。

## Verification

- `mise run fmt` / `check|lint|test al-gui`(既定) /
  `check|lint|test al-gui --features gpui-real` /
  `check|lint|test|build-ui-gpui`(非回帰)/
  `check-cli-headless` / `check-ci-workflow` / actionlint /
  `test-ui-gpui-real-headless` / `test-ui-gpui-real`。
