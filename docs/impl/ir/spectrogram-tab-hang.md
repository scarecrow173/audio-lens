---
id: "IMPL-034TXa4Ua60lBAZIZ0Txi2"
type: "impl"
status: "approved"
title: "Spectrogram tab hang 解消の実装記録"
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
  implements:
    - "docs/tasks/spectrogram-tab-hang.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

## 実装記録

### 変更内容

- `Cargo.lock`: ruviform 依存の解決 SHA を `a5b1db1` → `863a7a9d`
  (RF#453 canvas 0 サイズ是正・RF#451 keyboard tab 切替・RF#461
  `paint_layer` batch 化を全て含む ruviform `origin/main` HEAD)。
  `tools/update-ruviform-rev.mjs` + `mise run sync-ui-deps` で
  `ui/vendor` を同 SHA に同期。当初 `129786ba` で作業し、RF#461 の
  merge 通知を受けて `863a7a9d` へ再追従した。
- `crates/al-gui/tests/gpui_parity.rs`: workspace tab selection テストに
  drawn components へ `SPECTROGRAM_KEY` を含む assertion を追加。
- `crates/al-gui/tests/spectrogram_tab_paint.rs`(新規): 本番解像度
  512×128=65,536 cell の selectable Tabs で tab 選択→settle→drawn
  plan を検証する回帰テスト。

### root cause(調査結果)

1. pin `a5b1db1` の ruviform-ui-gpui が spectrogram canvas に
   `1280px × 0px` を与え、65,536 cell が同一 y の 1 px strip に
   全面 overlap → gpui `Scene::BoundsTree::insert` が実質停止し
   UI thread hard hang(実機 3/3 再現と一致)。
2. `129786ba` で canvas は `1280px × 737px` に修正され hang は解消
   するが、cell 毎 `paint_quad` の `BoundsTree` 挿入が O(n²) のため
   debug build で 1 paint 約 11.7〜12 秒残った。
   → ruviform#461 として起票し別 worker へ委譲。RF#461 は PR #462
   として merge(merge commit `863a7a9d`)、Spectrogram/Waveform/
   Meter/TimeAxis の quad 列が `paint_layer` で batch 化され、
   headless 実測で 65,536 cells が 6.37s → 53.3ms に改善。

### 検証結果(実測)

- `spectrogram_tab_paint`: pass(0.13s)。
- `test-ui-gpui` 全体: pass(gpui_parity 12 テスト 0.45s を含む)。
- 実機検証(debug build `target/debug/al-desktop.exe`、pin
  `863a7a9d`): Dataset → Spectrogram の pointer 選択で tab 切替が
  即時完了し、`Responding=True`・CPU almost idle(5 秒あたり 0.1s)
  を複数回確認。capture 画像に spectrogram heatmap の描画を確認
  (`F:/repositry/audio-products/.devin/evidence/al180-spectrogram-hang/`
  の `48-spectro2.png`/`53-spectro3.png`)。

### 残課題

- なし(AL#180 範囲)。観測メモ: 実機検証中、window capture が断続的に
  ほぼ単色 `#2b5f9e` の frame を返すことがあった(プロセスは応答し、
  次の pointer 操作で実描画へ復帰)。GPUI present/DComp 経路の
  capture artifact の可能性があり、AL#179(blank canvas)の調査材料
  として記録する。AL#179 本体の対応は別タスク。
