---
id: "PLAN-c1G8Ggtg9KLne46QN95cua"
type: "plan"
status: "draft"
title: "Native GUI Waveform/Seek 画面の ViewSpec 射影 実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - "crates/al-gui/src/view/screen.rs"
      - "crates/al-gui/tests/screen.rs"
    modified:
      - "crates/al-gui/src/view/source.rs"
      - "crates/al-gui/src/view/mod.rs"
      - "Cargo.lock"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/native-gui-waveform-seek-screen.md"
    - "../specs/native-gui-waveform-seek-screen.md"
  implemented-by: []
  depends-on:
    - "../specs/native-gui-waveform-seek-screen.md"
    - "../designs/native-gui-waveform-seek-screen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../test-specs/native-gui-waveform-seek-screen.md"
    - "../tasks/native-gui-waveform-seek-screen.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-waveform-seek-screen.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-waveform-seek-screen.md"
  references: []
---

# Native GUI Waveform/Seek 画面の ViewSpec 射影 実装計画

## 前提

- Ruviform resolved SHA を `a57cd3d3`(main HEAD)へ更新済み
  (`node tools/update-ruviform-rev.mjs`、`check-ruviform-rev`・
  `check-ruviform-lock`・`sync-ui-deps` 通過)。
- 設計: [design](../designs/native-gui-waveform-seek-screen.md)。

## 作業分割

| # | 作業 | 変更ファイル | 検証 |
| --- | --- | --- | --- |
| 1 | ruviform rev 更新 | `Cargo.lock` | `check-ruviform-lock`/`check-ruviform-rev`/`sync-ui-deps` |
| 2 | `read_frames` を読み出し範囲の単一決定として抽出 | `view/source.rs` | `test -p al-gui --test view`(既存) |
| 3 | Waveform/Seek の ViewSpec 射影 | `view/screen.rs`、`view/mod.rs` | 型チェック |
| 4 | Fake 層評価テスト | `tests/screen.rs` | `cargo test -p al-gui --test screen` |
| 5 | fmt/lint/test/gpui lane | - | `mise run fmt`/`lint`/`test`/`check-ui-gpui`/`lint-ui-gpui`/`test-ui-gpui` |

## 依存関係

1 → 2 → 3 → 4 → 5。2 と 3 は同一 PR 内で順に行う。

## リスク

- `ViewBin` の statistics 型が将来変わった場合、射影の 1 対 1 写像が
  崩れる。bit 比較テストで検知する。
- GPUI 側 parity は上流 #421/#426 に依存する。NotRun を PR 本文に明記する。

## ロールバック

射影モジュールと `read_frames` 抽出は独立しており、`view/screen.rs`
と `tests/screen.rs` の削除と `view/mod.rs` の 1 行削除で戻せる。
`Cargo.lock` は `git checkout -- Cargo.lock` で旧 rev に戻る。
