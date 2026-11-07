---
id: "DESIGN-hZqY4x9NK3Xg22TQK3K8mB"
type: "design"
status: "draft"
title: "Native GUI Waveform/Seek 画面の ViewSpec 射影と Fake 層評価の設計"
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
    - "../specs/native-gui-waveform-seek-screen.md"
  implemented-by: []
  depends-on:
    - "../specs/native-gui-waveform-seek-screen.md"
    - "native-desktop-initial-slices.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../plans/native-gui-waveform-seek-screen.md"
    - "../test-specs/native-gui-waveform-seek-screen.md"
    - "gui-app-architecture.md"
  refines:
    - "native-desktop-initial-slices.md"
  refined-by: []
  derives-from:
    - "../specs/native-gui-waveform-seek-screen.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-waveform-seek-screen.md"
  references:
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/426"
    - "https://github.com/scarecrow173/ruviform/commit/a57cd3d3bdae2b3dfde02bf4b496132bae7f3235"
  defers: []
  deferred-by: []
---

# Native GUI Waveform/Seek 画面の ViewSpec 射影と Fake 層評価の設計

## 概要

`WaveformView`(解決済み bounded source)を RF#387 `ViewSpec` へ写す
単一の射影関数 `view::screen::waveform_seek` を置く。射影は
driver-neutral で、GPUI 型を持たない。評価は `ruviform_ui::view::ViewHost`
(GPUI driver へ渡す `ViewMount` と同じ authoritative projection host)
を Fake 層として使い、同じ spec・gesture 列の event stream を比較する。

## 責務の境界

| 層 | 所有するもの | 所有しないもの |
| --- | --- | --- |
| `al-service` / `al-dataset` | 範囲読み出し・metadata | 表示 bin の意味 |
| `al_gui::view::source` | `WaveformView`(範囲 clamp・timebase・`ViewBin` 取得) | ViewSpec の組み立て |
| `al_gui::view::screen` | `ViewBin` → `WaveformBin` の 1 対 1 写像、Seek control 定義、Panel 構成 | 統計再計算・gesture 意味論 |
| `ruviform-ui::view::ViewHost` | mount/reconcile、gesture 状態機械、intent 生成、bin 上限検証 | 解析・theme 解決 |
| `al_gui::theme` | DTCG product token の解決(ResolvedTheme) | ViewSpec 内容 |
| `bin/al-desktop`(GPUI) | GPUI driver 生成・window 配線 | 射影・評価ロジック |

## 射影規則

1. `viewport` は `WaveformView::read_frames(start_s, end_s)` で導出する。
   `window` が実際に読む範囲と同一であり、空区間は 1 frame へ広げられるため
   viewport と bins が食い違わない(`read_frames` は `window` の唯一の
   範囲決定。clamp は非公開の `frame_range` に閉じる)。
   `bins` が `MAX_WAVEFORM_BINS` を超える要求は読み出し前に拒否する。
2. `channel_bins` は `WaveformView::window(channel, start_s, end_s, bins)`
   の `ViewBin` を 1 対 1 で `WaveformBin { start, end, min, max, rms, count }`
   へ写す。`bucket` の値は変換・丸め・再計算しない。
3. `channels` は 1(選択 channel のみを payload に載せる)。`frames`
   と `sample_rate` は source metadata をそのまま使う。
4. Seek は `ValueDescriptor::ranged(0.0, frames / sample_rate)`。
   `frames == 0` の source は射影を拒否する(`ViewError::Invalid`)。
5. key は `audio-view`(Panel)/`waveform`/`seek`、Seek の target は
   `transport.seek`。定数として公開し、テストと消費側が同じ値を使う。

## 評価(Fake 層)

- fixture: `ruviform_viz::SliceWaveform`(NaN gap 区間を含む)から
  `WaveformView` を作る。
- 観測 1(数値): 射影 bins と `WaveformView::window` の bins を
  `to_bits()` で比較し、NaN を含めて同一であることを確認する。
- 観測 2(gap): `count == 0` の bin が `min/max/rms` とも NaN で
  あること。fixture が実際に gap bin を生むことも確認する。
- 観測 3(操作): `ViewHost` に mount し、`Begin → Preview → Commit` を
  Seek へ送る。`CommandIntent::Commit` の target が `transport.seek`、
  committed 値が preview 値と一致すること。
- 観測 4(theme 中立): `theme::MODES` の light/dark で ResolvedTheme が
  解決でき、同一 spec・gesture 列の event stream(`Debug` 比較)が一致
  すること。

## GPUI 側の扱い

GPUI driver の `create_view_window` は `ViewMount { host, theme }` を受け
るが、描画結果と intent の観測は `ruviform-ui-gpui` の crate 内
`pub(crate)`(`settle_for_test`/`inbox_len_for_test`)に閉じている。
consumer の test から Fake と同じ観測を取る経路が無いため、GPUI 側の
parity 評価は本 slice では NotRun とし、上流 #421/#426 の完了を前提条件
として記録する。`al-desktop` の root は変更しない。

## 代替案と棄却理由

- `WaveformView` 側に ViewSpec 生成を置く: 表示層の型を view model に
  持ち込むため棄却。射影は `view::screen` に分離する。
- viewport を `start_s`/`end_s` から別計算する: clamp 規則が二重化し、
  bins と viewport がずれうるため棄却。
- GPUI 側へ独自の観測 seam を追加する: 上流 crate の変更を consumer
  側の workaround で隠すことになるため棄却(上流 #421/#426 で扱う)。
