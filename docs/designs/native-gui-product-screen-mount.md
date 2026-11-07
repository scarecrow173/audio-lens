---
id: "DESIGN-rmQlUpDmFZLlonLOUfAGa4"
type: "design"
status: "draft"
title: "Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity の設計"
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
    - "native-gui-product-screen-mount.md"
  implemented-by: []
  depends-on:
    - "native-gui-waveform-seek-screen.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/native-gui-product-screen-mount.md"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/429"
  refines: []
  refined-by: []
  derives-from:
    - "native-gui-product-screen-mount.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Source 画面の ViewSpec 射影・al-desktop mount・GPUI parity の設計

## 概要

第 2 slice で導入した `view::screen` の Waveform/Seek 射影を Source 画面へ広げ、
`al-desktop` の fixture window を Source 画面の view window へ置き換える。
GPUI の評価は Ruviform #429 の `GpuiObserver`(`gpui-test` feature)で行い、
Fake 層(`ViewHost` 直接評価)と同一 fixture で比較する。

## 責務の境界

- `al_gui::view::screen`(driver-neutral): 解決済み bin と Source 範囲から
  `ViewSpec` を組み立てる。GPUI 型・`ruviform-ui-gpui` 型は現れない。
- `al_gui::view::source`(driver-neutral): `WaveformView` が bounded な
  `ViewBin` 読み出しと `SampleTimebase` を保持する。数値の正本は ruviform-viz 側。
- `bin/al-desktop.rs`(composition root, feature `gpui`): dataset を開き、
  `source_screen` の結果を `ViewHost` へ mount し、`GpuiDriver::create_view_window`
  に渡す。GPUI 型はこの bin と `ruviform-ui-gpui` にのみ現れる。
- `tests/gpui_parity.rs`(feature `gpui-test`): `GpuiObserver` だけを使い、
  GPUI 型を持たずに観測する。

## 射影規則

| component | 元データ | 写像 | 再計算しない値 |
| --- | --- | --- | --- |
| `waveform` | `window(channel, start, end, bins)` の `ViewBin` | `WaveformBin` へ 1 対 1 | min/max/rms/count |
| `meter` | 同じ `ViewBin` | `MeterBin.peak = max(|min|, |max|)`、`rms`/`count` は 1 対 1 | rms/count |
| `seek` | source 全長 | `ValueDescriptor::ranged(0, duration)`、Commit は `transport.seek` | 範囲 |
| `ruler` | viewport(frames)と `sample_rate` | tick 位置 `k * step * rate` を viewport 内に限定、label は `k * step` | 解析値なし |

- `peak` は bin の `min`/`max` が有限 sample の極値なので、`max(|min|, |max|)` は
  有限 sample の最大絶対値と一致する。gap(`count == 0`)では両者が NaN のまま
  `peak` も NaN になる。これが Meter で唯一の導出であり、それ以外は写さない。
- Waveform と Meter は同じ `window` 結果を一度だけ読み、両方に写す(読み出しの二重化を避ける)。
- Ruler の刻み `step` は表示 span に応じて区間数が 10 以下になる最小値(候補表)。
  位置は丸め後に viewport 内へ収まるものだけを残す。
- 画面の bin 上限は Meter の上限 `MAX_METER_BINS`(8192)とする。Waveform と Meter が
  同じ bin を共有するため、小さい方が画面全体の上限になる。

## al-desktop の mount

- 引数 `[<dataset-root> <input-id>]` があれば `DatasetSession::open` →
  `WaveformView::open` → `source_screen(全長, 1024 bins)` で ViewSpec を得る。
- 引数が無ければ Source を載せない空の Panel を mount する(Open 導線は AL-G4)。
- `ViewHost` の `AttachEpoch` は `AttachEpochAllocator` から mint する
  (Ruviform の command-id namespace 規約に従う)。
- theme は `DesktopSession` に注入済みの `ResolvedTheme` を `ViewMount.theme` へ渡す。

## GPUI parity の評価

- 同一 `source_screen` 出力(`fixture_view` = 48 kHz・4800 frame・NaN gap あり)を
  Fake と GPUI の両方に mount する。
- Fake: `ViewHost::mount` → `dispatch_input`(Begin/Preview/Commit)→ `drain_events`。
- GPUI: `GpuiObserver::driver_mut().create_view_window` → `settle` → `dispatch_input`
  → `settle` → `observe` / `intents`。
- 比較: mount 済み key 集合、各 component の `mounted`・`phase`・`display`・`committed`
  (f64 は bit 比較)、Seek gesture で増えた command intent の `params.intent` の JSON。

## GPUI 側の扱い

- 描画 plan(Meter の bar・TimeAxis の mark)は `GpuiObserver` の公開 seam に無い。
  この slice では component の状態 parity までとし、描画 parity は NotRun。
- `al-desktop` の実 OS window 起動は CI と本作業では行わない(NotRun)。

## 代替案と棄却理由

- **Job/Run/Analysis をテキストとして ViewSpec に詰める**(Panel title や a11y label に
  文字列を入れる): 表・リストの構造が失われ、Ruviform の契約を自前の規約で
  迂回することになるため棄却。契約拡張を Ruviform に依頼する。
- **Meter を Waveform 射影とは別の集計で作る**: 同じ数値の二重計算になり、
  数値の正本が二つ生じるため棄却。`ViewBin` からの写像に限定する。
- **consumer test を GPUI 専用ヘルパーで書く**: Fake と同一の fixture を評価する
  parity の意味が弱まるため棄却。両 driver で同じ `ViewSpec` を使う。

## 上流契約のギャップ(Ruviform へ報告)

- Spectrogram を表す ViewSpec kind が無い → scarecrow173/ruviform#434
- テキスト・ラベル・表・リスト(Job/Run/Analysis の表示に必要)を表す kind が無い → scarecrow173/ruviform#435
- `GpuiObserver` が component の描画 plan(Meter/TimeAxis の drawn 結果)を公開していない → scarecrow173/ruviform#436

いずれも AudioLens 側の未達項目の依存先。AudioLens 側に恒久的な回避策(Panel title に
文字列を詰める等)は置かず、上記 Issue の解消後に射影を追加する。
