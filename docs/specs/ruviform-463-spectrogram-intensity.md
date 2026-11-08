---
id: "SPEC-034U1irN0QulZX3YeqAYVY"
type: "spec"
status: "approved"
title: "ruviform pin 54439fdc 追従と Spectrogram cell intensity の実窓検証（ruviform#463 consumer）"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/ruviform/issues/463"
    - "https://github.com/scarecrow173/audio-lens/issues/179"
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
  related:
    - "../designs/ruviform-463-spectrogram-intensity.md"
    - "../test-specs/ruviform-463-spectrogram-intensity.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/ruviform-463-spectrogram-intensity.md"
  references:
    - "spectrogram-tab-hang.md"
    - "canvas-paint-empty.md"
  defers: []
  deferred-by: []
---

# ruviform pin 54439fdc 追従と Spectrogram cell intensity の実窓検証（ruviform#463 consumer）

## What Should Be Built

AudioLens の ruviform pin を `863a7a9d` から `54439fdc`(ruviform main
HEAD、RF#463 / PR #466「Spectrogram cell の intensity(alpha)を実窓へ
反映」を含む)へ追従し、Dataset workspace の Spectrogram tab を実 OS
window で開いたとき、cell の intensity 差が pixel の中間色・複数色の
勾配として観測できることを確認する。AudioLens 側のコード変更は
行わず、pin 追従と検証証拠の記録に限定する。

## Why Now

AL#179 の実窓検証で、Spectrogram spec 上の cell.intensity が
0.2〜1.0 に散るのに対し pixel は `viz.waveform` 系の 2 色
(`#4A90D9`/`#498FD8`)に飽和し、中間 alpha が観測されないことが
判明した。root cause は ruviform-ui-gpui の Spectrogram painter が
cell 毎に半透明 quad を発行し、`SpectrogramColumn` の frame 区間が
契約上重なりうる(`Tile::columns` が cols > frames のとき frame を
共有)ため同一 pixel の alpha が `1-(1-a)^n` で累積・飽和すること
だった。ruviform 側は RF#463 / PR #466 で cell の paint 色を
`surface_bg` への source-over 事前合成による不透明色に変更して
解消済みであり、consumer である AudioLens は pin 追従と実窓での
最終確認を行う。

## Users and Value

- AudioLens 利用者: Spectrogram の intensity 勾配が実窓で忠実に
  表示され、音声エネルギー分布の視認性が回復する。
- 開発者: ruviform#463 の consumer 側受入として、pin 追従と
  実窓 pixel 解析の証拠が揃う。

## Scope

### In Scope

- `Cargo.lock` の ruviform 解決 SHA を `863a7a9d` → `54439fdc` へ
  `tools/update-ruviform-rev.mjs` で更新し、`mise run sync-ui-deps`
  で `ui/vendor` を同 SHA に同期する。
- Dataset workspace Spectrogram tab の実 OS window pixel 解析で
  intensity 勾配(中間色・複数色、2 色飽和でないこと)を確認し、
  証拠を `.devin/evidence/` に保存する。
- 既存回帰テスト(`spectrogram_tab_paint`、`gpui_parity`)が新 pin で
  pass することを確認する。
- 本 pin 追従の文書チェーン(spec/design/plan/task/test-spec/IR)を
  作成する。

### Out of Scope

- ruviform 側の renderer 修正そのもの(RF#463 / PR #466 で完了済み)。
  AL 側に恒久 workaround は置かない。
- Spectrogram spec の解像度・データ供給経路の変更(AL projection は
  ruviform 契約内で正しい)。
- 新規の自動回帰テスト追加(ruviform 側に
  `spectrogram_cell_color_is_opaque_intensity_composite` が追加済みで、
  AL 側は既存 `spectrogram_tab_paint`/`gpui_parity` の pass 確認で
  十分)。
- audio-lens 側 Issue の起票(consumer 後続検証として本 spec で扱う)。

## Requirements

- `Cargo.lock` の ruviform 解決 SHA が `54439fdc` 以降で単一である
  こと(`mise run check-ruviform-lock` / `check-ruviform-rev` が pass)。
- `ui/vendor` が同 SHA で同期されていること(`mise run sync-ui-deps`)。
- 実 OS window で Dataset workspace の Spectrogram tab を開き、
  canvas 領域の pixel 解析で intensity に対応する中間色・複数色の
  勾配が確認できること(修正前の 2 色飽和 `#4A90D9`/`#498FD8` のみ
  ではないこと)。
- 証拠 screenshot・pixel 解析結果を `.devin/evidence/` に保存する
  こと。
- `spectrogram_tab_paint`・`gpui_parity` を含む既存テストが pass
  すること。

## Acceptance Criteria

- [x] ruviform pin が `54439fdc` 以降に更新され、
  `check-ruviform-lock`/`check-ruviform-rev` が pass すること。
- [x] Dataset workspace Spectrogram tab の実窓 pixel 解析で
  intensity 勾配(中間色・複数色)が確認できること。
- [x] 証拠が `.devin/evidence/` に保存されていること。
- [x] `spectrogram_tab_paint`・`gpui_parity` 等の既存回帰テストが
  pass すること。

## Deferred Design Concerns

- なし。

## Sources and Relations

- ruviform Issue #463 / PR #466(Spectrogram cell intensity の
  alpha 累積飽和を surface_bg 事前合成で解消)。
- AL#179 実窓検証で観測した 2 色飽和
  (`F:/repositry/audio-products/.devin/evidence/al179-canvas-paint/`、
  verdict.md「異常(別件)」節)。
- `spectrogram-tab-hang.md`(AL#180): pin `863a7a9d` 追従と
  Spectrogram tab hang 解消の前提。
- `canvas-paint-empty.md`(AL#179): DatasetWaveform 単位振幅正規化。
