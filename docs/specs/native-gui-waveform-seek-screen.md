---
id: "SPEC-XU6YkQ1VJGeG1v6i3nVzTm"
type: "spec"
status: "draft"
title: "Native GUI Waveform/Seek 画面の ViewSpec 射影と Fake 層評価 (Issue #166 第2 slice)"
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
  implements: []
  implemented-by: []
  depends-on:
    - "native-desktop-initial-slices.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/native-gui-waveform-seek-screen.md"
    - "../test-specs/native-gui-waveform-seek-screen.md"
    - "https://github.com/scarecrow173/ruviform/issues/387"
    - "https://github.com/scarecrow173/ruviform/issues/421"
    - "https://github.com/scarecrow173/ruviform/issues/426"
  refines: []
  refined-by: []
  derives-from:
    - "native-desktop-initial-slices.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/native-gui-waveform-seek-screen.md"
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Waveform/Seek 画面の ViewSpec 射影と Fake 層評価

## What Should Be Built

Issue [#166](https://github.com/scarecrow173/audio-lens/issues/166) の
AL-G3(Job/Run/Analysis/Source View Model の GPUI 投影・Waveform/Seek
の高精度 Viz 接続)の第 2 slice として、既存の Source view model
(`al_gui::view::source::WaveformView`)を RF#387 の Component/View
Spec(`ViewSpec`)へ射影し、driver-neutral な `ViewHost` で評価できる
ようにする。第 1 slice(PR #168)の数値 oracle・composition root は
変更しない。

- 射影対象: 選択 channel の `[start_s, end_s)` 範囲の解決済み
  `ViewBin` を `WaveformSpec` payload へ 1 対 1 で写す。
  統計値(min/max/rms)・`count`・gap(`NaN`/`count == 0`)は変換・
  再計算せず、bit 同一に保つ。
- 操作: source 全長 `[0, duration_s]` を範囲とする Seek
  (`ControlSpec`、target `transport.seek`)を同じ Panel に並べる。
  Seek の `Begin → Preview* → Commit` は ruviform-ui の
  `ViewHost` が intent に変換し、`CommandIntent` として観測できる。
- Theme: product theme(`al_gui::theme`)は `ViewSpec` に含めない。
  light/dark のどちらで解決しても同一の semantic event stream に
  なることを検証する(theme は renderer 側の関心事)。
- GPUI 型は `bin/al-desktop.rs`(composition root)に閉じ、この
  射影層・Domain・CLI へ露出させない。

## Non-goals(本 slice では達成しない — 未達と明記)

- GPUI driver 上での projection・intent 観測(実 window 描画・
  Seek drag の実入力)。`ruviform-ui-gpui` の観測 seam は crate 内
  `pub(crate)` のみで、consumer の test から観測できない。上流の
  #421(新 kind の Fake/GPUI parity)・#426(GPUI lab binding)で
  追跡し、本件では NotRun とする。
- TimeAxis / Meter / Spectrogram の ViewSpec 射影(ruviform の対応
  kind・GPUI 描画は #421 の範囲)。
- product screen の root 差し替え(`al-desktop` の window は従来の
  fixture surface のまま)。Dataset の選択 UI・Job/Run 一覧の投影。
- Focus/Keyboard/Accessibility の実機検証、IME、実 OS window の
  視覚 PASS、性能非回帰の実機計測(AL-G4/G5)。
- Golden 画像による精度判定。

## Constraints

- 新規外部依存は追加しない(`ruviform-ui`/`ruviform-viz` は既存)。
- 数値/Source 契約(f64/NaN/gap/PTS/timebase/Validity/Provenance)は
  変更しない。解析計算を表示層へ移さない。
- 射影は bounded。bin 数は `ruviform_ui::view::MAX_WAVEFORM_BINS`
  を超える要求を射影の入口で拒否し、読み出し前に上限を確かめる。
- 既存の `WaveformView::window` の範囲 clamp 規則は維持し、viewport
  と bins が同じ範囲を読むよう `read_frames` を単一の範囲決定とする。
