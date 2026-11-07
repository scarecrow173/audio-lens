---
id: "PLAN-5D5yfFUG8HGrWGJlg97kCG"
type: "plan"
status: "draft"
title: "Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity 実装計画"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    added:
      - "crates/al-gui/tests/common/mod.rs"
      - "crates/al-gui/tests/gpui_parity.rs"
    modified:
      - "Cargo.lock"
      - "crates/al-gui/Cargo.toml"
      - "crates/al-gui/src/view/screen.rs"
      - "crates/al-gui/src/bin/al-desktop.rs"
      - "crates/al-gui/tests/screen.rs"
      - "mise.toml"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "native-gui-product-screen-mount.md"
  implemented-by: []
  depends-on:
    - "../designs/native-gui-product-screen-mount.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/429"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/native-gui-product-screen-mount.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-product-screen-mount.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity 実装計画

## 前提

- audio-lens main: `7a997b5`(PR #169 merge)から新 branch を切る。
- Ruviform rev を `c31dc67c`(RF#429 `GpuiObserver`・RF#421 Meter/TimeAxis)へ更新する。
  `tools/update-ruviform-rev.mjs` で `Cargo.lock` のみ更新し、`check-ruviform-lock` /
  `check-ruviform-rev` / `sync-ui-deps` を通す。

## 作業分割

1. Ruviform rev 更新 (`Cargo.lock`) と既存検証の確認。
2. Source 画面の射影拡張(`view::screen::source_screen`): TimeAxis(ruler)と Meter を
   同じ解決済み bin から射影。Fake 層テストを先に書く(red → green)。
3. `al-desktop` の mount: fixture window を `create_view_window` + 空/Source 画面へ置換。
4. `gpui-test` feature と `GpuiObserver` を使う parity test。fixture は `tests/common` で共有。
5. mise task の feature 切替(`check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`)。
6. 文書 chain(spec/design/plan/test-spec/tasks/impl IR)と各 README の索引。
7. 検証(mise 正規タスク)と review-protocol の実施。
8. PR 作成(`Refs #166`、Issue は close しない)。

## File map

| file | 変更 |
| --- | --- |
| `Cargo.lock` | ruviform 20 package の source を `c31dc67c` へ |
| `crates/al-gui/src/view/screen.rs` | `waveform_seek` → `source_screen`(TimeAxis/Meter 追加、ruler 刻み・peak 写像) |
| `crates/al-gui/src/bin/al-desktop.rs` | Source 引数・空画面・`create_view_window` mount |
| `crates/al-gui/Cargo.toml` | feature `gpui-test` 追加 |
| `crates/al-gui/tests/common/mod.rs` | Fake/GPUI 共有 fixture |
| `crates/al-gui/tests/screen.rs` | Meter/ruler の Fake 層評価を追加 |
| `crates/al-gui/tests/gpui_parity.rs` | `GpuiObserver` による parity |
| `mise.toml` | gpui 系 task を `gpui-test` へ |

## 依存関係

- 1 → 2 → 3 → 4 → 5。6 は実装結果を反映するため最後に確定する。
- 4 は Ruviform #429 の `GpuiObserver`(rev c31dc67)に依存する。

## リスク

- `GpuiObserver` は `gpui-test` feature(`gpui/test-support`)を要し、ビルド時間が増える。
  mise の gpui 系 task を同 feature に寄せて、検証面を一つに保つ。
- 丸め誤差で ruler の tick が viewport 外に出る(実際に発生した)。frame 空間で生成し、
  最終 `at` で範囲判定して落とす。
- Job/Run/Analysis と Spectrogram は ViewSpec の kind が無く、この slice では射影できない。
  未達を明記し、Ruviform に契約拡張を依頼する。

## ロールバック

- `Cargo.lock` を `git checkout -- Cargo.lock` で戻し、`sync-ui-deps` で vendor を再生成。
- 射影の変更は `screen.rs` と `al-desktop.rs` の revert で旧 `waveform_seek` 相当へ戻せる。
