---
id: "TASK-JW0iKTumsryZvMQUhfLfIf"
type: "task"
status: "in-progress"
title: "Issue #166 Job/Run tab mount と Tile::columns 消費 実装タスク (第6 slice)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/native-gui-job-run-mount.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-job-run-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Job/Run tab mount と Tile::columns 消費 実装タスク

## T1: ruviform rev を d3be1ad9 へ更新

- [x] `node tools/update-ruviform-rev.mjs d3be1ad9d57ee9fb8d5a549fb2a4aa810b611a92`(Cargo.lock の 20 source のみ)
- [x] `mise run sync-ui-deps` → `check-ruviform-lock` → `check-ruviform-rev` を通す

## T2: RF443 の Tile::columns 消費

- [x] `tests/spectrogram.rs` に `Tile::columns` 一致と空 tile の境界を追加
- [x] `view/screen.rs` の `spectrogram_columns` を削除し、`tile.columns()` を写す

## T3: 共有 helper と RunOutcome::from_manifest

- [x] `tabs`・`unavailable` を `view/mod.rs` へ移し、workspace を移行
- [x] `RunOutcome::from_manifest` を追加

## T4: view::desktop と al-desktop の mount

- [x] `view/desktop.rs` に Jobs・Run・Dataset の Tabs と fallback を実装
- [x] `al-desktop` を `[<base-dir> [<job-id>]]` にし、`desktop_screen` を mount
- [x] 旧 `<dataset-root> <input-id>` の経路を削除

## T5: テスト

- [x] `tests/common::make_job` を追加
- [x] `tests/desktop_screen.rs` で tab 構造・Job 一覧・Run・Dataset・fallback を検証
- [x] `tests/gpui_parity.rs` に desktop の drawn parity を追加し、`find` を Tabs 対応に

## T6: 検証・文書・レビュー

- [x] 全 mise task(fmt・check・lint・test・build・ui-gpui 系・headless・ruviform・ui)をコード HEAD `adbd5ed` で実行(全件通過。NotRun は IR に記録)
- [x] docs の索引(specs・designs・plans・test-specs・tasks・impl)を更新
- [x] IR(`impl/ir/native-gui-job-run-mount.md`)へ実装結果と受入対応表を記録
- [x] review-protocol を独立サブエージェント 2 件で実施し、P2 の B1 を修正。残りは非 blocker として IR に記録
- [x] Ruviform 側の Issue(行選択・command 起動: ruviform#448)を起票し、ruviform#443 へ消費状況を追記

## T7: PR

- [ ] branch を push し、PR(日本語・`Refs #166`)を作成。Issue は close しない
