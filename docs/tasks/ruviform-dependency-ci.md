---
id: "TASK-034P3OFAlhyz4NNpeFYtTc"
type: "task"
status: "todo"
title: "Ruviform 依存固定と CI 整備(AL#99)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/99"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
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

# Ruviform 依存固定と CI 整備(AL#99)

## Work

Ruviform crate を AudioLens workspace へ導入するための依存・
toolchain・CI 基盤を整える。Issue
[#99](https://github.com/scarecrow173/audio-lens/issues/99) を正本とし、
検証済み同一 Git SHA の git 依存・private repository アクセス・
再現可能ビルド・GUI/headless の依存分離を確立する。

- Ruviform crate の git 依存を固定 SHA で導入し lockfile で再現
- private repository への最小権限 credential と CI アクセスを整備
- GUI backend 依存を al-gui 側 optional deps に隔離し、CLI/headless
  と GUI の個別ビルド・テストを CI に入れる

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/specs/ruviform-foundation-migration.md`
- Issue [#99](https://github.com/scarecrow173/audio-lens/issues/99)

## Dependencies

- `docs/plans/ruviform-foundation-migration.md`(段1 前提)

## Affected Files

- `Cargo.toml` / `Cargo.lock`(workspace 依存)
- `crates/*/Cargo.toml`(利用 crate の依存)
- CI 設定(#99 で確定するファイル)

## Done When

- [ ] 固定 SHA からの再現可能ビルドが確立
- [ ] private repository アクセスが最小権限で CI 動作
- [ ] GUI/CLI/headless の依存分離がビルド・テストで検証できる
- [ ] #99 の完了条件を満たす

## Verification

- [ ] `mise run build` が固定依存で通る
- [ ] GUI 依存なしの headless ビルドが通る
- [ ] クリーン環境で lockfile から同一 SHA が復元される
