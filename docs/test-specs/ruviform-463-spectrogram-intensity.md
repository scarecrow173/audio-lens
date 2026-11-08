---
id: "TSPEC-034U1kXWDA80w3D11XAkva"
type: "test-spec"
status: "approved"
title: "ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証のテスト仕様"
created: "2026-11-08"
updated: "2026-11-08"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/ruviform-463-spectrogram-intensity.md"
  derived-by: []
  verifies:
    - "docs/specs/ruviform-463-spectrogram-intensity.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ruviform pin 54439fdc 追従と Spectrogram intensity 実窓検証のテスト仕様

## Purpose

ruviform#463(Spectrogram cell の intensity alpha 累積飽和)の consumer
側受入として、pin 追従後に Spectrogram intensity が実窓 pixel で
勾配として観測できること、および既存回帰テストが pass することを
保証する。pin 退行や renderer 修正の無効化を検出するための仕様で、
ruviform 側の単体保証(`spectrogram_cell_color_is_opaque_intensity_composite`)
は ruviform 側 test-spec が担う。

## Feature

ruviform pin `54439fdc` 追従後の Spectrogram tab 実窓描画

## Rules

- `Cargo.lock` の ruviform 解決 SHA は単一で `54439fdc` 以降を指し、
  `mise run check-ruviform-lock` / `check-ruviform-rev` が pass する。
- `ui/vendor` は lockfile と同一 SHA に同期されている
  (`mise run sync-ui-deps` の `--check` が pass)。
- 実 OS window の Spectrogram canvas 領域で、intensity に対応する
  中間色・複数色が観測できる(2 色 `#4A90D9`/`#498FD8` のみへの
  飽和ではない)。
- `spectrogram_tab_paint`・`gpui_parity` を含む既存テストが pass する。

## Examples

- Given pin `54439fdc` の al-desktop debug build、When Dataset
  workspace の Spectrogram tab を pointer 選択して settle 後に
  capture する、Then canvas 領域の pixel 色分布に `viz.waveform`
  系の中間 alpha 合成色(暗色背景への source-over 結果)が複数含まれ、
  2 色飽和でないこと。
- Given 同 build で Spectrogram tab を選択する、When tab 切替が
  完了する、Then window は `Responding=True` を維持すること
  (AL#180 hang の回帰がないこと)。

## Guarantees

- pin 追従と vendor 同期が lockfile 正本契約どおり完了している。
- intensity 勾配の実窓確認が pixel 解析の証拠として残る。
- 既存の自動回帰テストが新 pin で pass し、描画経路の退行がない。

## Non-goals

- intensity 値と pixel 色の厳密な数値対応の検証(概略の勾配確認に
  留める。厳密な合成色検証は ruviform 側 unit test の責務)。
- Spectrogram 以外の canvas component の再検証(AL#179 で実施済み)。
- headless/Fake 層での intensity 描画検証(renderer の pixel 出力は
  GPUI 実窓でのみ観測可能)。

## Risk

- 実窓 pixel 解析を省略すると、pin 追従だけでは「2 色飽和が解消した」
  ことを consumer として確認できず、ruviform#463 の受入が宙に浮く。
- capture artifact(単色 frame)を実描画と誤認するリスク → 複数回
  capture と `Responding` 確認で切り分ける。
