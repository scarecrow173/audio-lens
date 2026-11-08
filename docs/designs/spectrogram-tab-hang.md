---
id: "DESIGN-034TX8udVdtrH0SBVRBMly"
type: "design"
status: "approved"
title: "Spectrogram tab hang 解消の設計（ruviform pin 追従と回帰検証）"
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
    - "docs/specs/spectrogram-tab-hang.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## 概要

hang の root cause は 2 段階。(1) pin `a5b1db1` の ruviform-ui-gpui が
spectrogram の paint canvas に 0 px 高さを与え(RF#453 既知不具合)、
65,536 cell 全てが同一 y の 1 px strip に縮退・全面 overlap して
gpui `Scene::BoundsTree::insert` が実質停止。(2) canvas が正しい
bounds を得ても、cell 毎 `paint_quad` は `BoundsTree` で O(n²) となり
debug build で 1 paint 約 11.7〜12 秒かかった。

修正方針は「AL 側に workaround を置かず、上流修正を pin で取り込む」
一点。(1) は RF#453 / PR #459、(2) は RF#461 / PR #462
(`paint_layer` batch 化、Spectrogram 65,536 cells が headless 実測
6.37s → 53.3ms)で ruviform 側が解消済み。AL は ruviform main HEAD
`863a7a9d` への pin 追従で両方を同時に取り込む。

## 設計判断

- **pin 更新のみ**: `Cargo.lock` の ruviform 解決 SHA を `a5b1db1` →
  `863a7a9d` へ `tools/update-ruviform-rev.mjs` で更新し、
  `mise run sync-ui-deps` で `ui/vendor` を同 SHA に同期する。
  manifest に rev は書かない(lockfile が正本、既存契約どおり)。
- **spec 解像度は不変**: `SPECTROGRAM_COLS=512`/`ROWS=128` は
  `MAX_SPECTROGRAM_CELLS` の契約内で正しく、描画コストは renderer
  側の責務。AL 側で解像度を下げるのは workaround にあたるため行わない。
- **回帰テスト**: 実データ経路(`desktop_screen`)は 1 秒 fixture では
  cell 数が不足するため、本番解像度(512×128=65,536 cell)を持つ
  selectable Tabs spec を `spectrogram_screen` 出力から構成し、
  `GpuiObserver` の `select_tab`→`settle`→`drawn` で paint 完了を
  検証する `spectrogram_tab_paint` テストを追加。pin 退行時は
  `settle` が戻らず hang する。
- **parity 補強**: 既存 `workspace_tab_selection_*` テストに
  spectrogram の drawn plan 含有 assertion を追加する。

## 検証

- `mise run test-ui-gpui`(gpui-test feature)で Fake/GPUI parity と
  回帰テストが pass すること。
- `mise run check-ruviform-lock` / `check-ruviform-rev` で pin 整合。
- 実 OS window(debug build `al-desktop`)で Dataset→Spectrogram tab を
  pointer 選択し、切替完了・`Responding=True`・spectrogram heatmap の
  描画を pixel/log で確認済み(pin `863a7a9d`、debug build でも
  paint は即時完了)。
