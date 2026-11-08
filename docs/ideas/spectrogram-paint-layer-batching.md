---
id: "IDEA-034TX8bCPpz4UEP5pQHBpX"
type: "idea"
status: "promoted"
title: "Spectrogram の cell 描画を paint_layer で batch 化する（ruviform#461）"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## Idea

ruviform-ui-gpui の `spectrogram_element` が cell ごとに `window.paint_quad` を発行するため、`MAX_SPECTROGRAM_CELLS`(65,536)規模で gpui `Scene::BoundsTree` の挿入が O(n²) となり、debug build で 1 paint あたり約 12 秒かかる。cell 描画を `window.paint_layer(bounds, ...)` で囲み primitive を batch 化すると、layer 内は `BoundsTree::insert` を経由せず O(n) になる。ローカル計測で 11.74s → 43.5ms(約 270 倍)を確認済み。Waveform/Meter 等の同型 painter も監査対象。

## 背景と根拠

- AL#180 / ruviform#461。pin `129786ba` で canvas 0 サイズ(RF#453)は解消したが、primitive 数由来の O(n²) が残る。
- 実装・検証は ruviform 側の責務。AudioLens は pin 追従のみ行う。

## 再評価の条件

- ~~ruviform#461 が fix され pin を再追従した時点で本 idea は消化済みとして close する。~~
  **消化済み(2026-11-08)**: ruviform#461 は PR #462 として merge
  (merge commit `863a7a9d`)。Spectrogram/Waveform/Meter/TimeAxis の
  quad 列が `paint_layer` で batch 化され、headless 実測で Spectrogram
  65,536 cells が 6.37s → 53.3ms に改善。AudioLens 側は pin を
  `863a7a9d` へ追従済み(`docs/specs/spectrogram-tab-hang.md`)。
