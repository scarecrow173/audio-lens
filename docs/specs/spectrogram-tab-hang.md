---
id: "SPEC-034TX7soOs7I0ovUdhRVaw"
type: "spec"
status: "approved"
title: "Spectrogram tab 選択時の UI thread hang 解消（ruviform pin 追従）(Issue #180)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/180"
    - "https://github.com/scarecrow173/ruviform/issues/461"
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
    - "../designs/spectrogram-tab-hang.md"
    - "../test-specs/spectrogram-tab-hang.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/spectrogram-tab-hang.md"
  references:
    - "native-gui-workspace-tabs-selection.md"
  defers:
    - "../ideas/spectrogram-paint-layer-batching.md"
  deferred-by: []
---

# Spectrogram tab 選択時の UI thread hang 解消（ruviform pin 追従）(Issue #180)

## What Should Be Built

Dataset workspace の Spectrogram tab を pointer で選択したとき、tab 切替が
完了し UI thread が hang しない状態を実現する。原因は AudioLens 側の
spec 生成・投影ではなく、pin していた ruviform `a5b1db1` の GPUI renderer が
spectrogram の paint canvas に 0 px の高さを与え、`MAX_SPECTROGRAM_CELLS`
上限いっぱいの cell が全て同一位置の重なり quad となって gpui `BoundsTree`
の挿入が実質停止することであった。canvas 0 サイズは ruviform RF#453 /
PR #459 で、cell 毎 `paint_quad` の O(n²) `BoundsTree` コストは RF#461 /
PR #462(`paint_layer` batch 化)で修正済みであり、本 spec はその両修正を
含む ruviform `863a7a9d` への pin 追従と、回帰を防ぐ検証の追加を定める。

## Why Now

AL-G5 実機受入検証で Spectrogram tab の pointer 選択が 3/3 で
`Responding=False`・CPU 上昇継続の hard hang を起こすことが確認された
(Issue #180)。Source・Analysis tab の切替は正常で、spectrogram は
`MAX_SPECTROGRAM_CELLS = 65536` の cell を持つ唯一の viz であり、
tab が表示されるまで paint されないため選択時にのみ顕在化した。

## Users and Value

- AudioLens 利用者: Dataset workspace で Spectrogram tab を開いても
  アプリが応答不能にならない。
- 開発者: hang の再発を GPUI parity テストで検出できる。

## Scope

### In Scope

- ruviform pin を `a5b1db1` から `863a7a9d`(RF#453 canvas 0 サイズ
  是正・RF#451 keyboard tab 切替・RF#461 `paint_layer` batch 化を
  含む ruviform main HEAD)へ更新する。
- Spectrogram tab 選択 → settle → 描画 plan の回帰テストを
  `gpui-test` lane に追加する。
- 実機検証で観測した描画の経緯(0 サイズ canvas hang → 正常 bounds での
  O(n²) paint 遅延 → `paint_layer` batch 化による解消)を記録する。

### Out of Scope

- ruviform 側の renderer 修正そのもの。RF#453 / RF#461 は ruviform 側で
  完了済みであり、AudioLens 側に恒久 workaround は置かない。
- Spectrogram の spec 解像度・データ供給経路の変更(AL 側の spec は
  ruviform 契約 `MAX_SPECTROGRAM_CELLS` の範囲内で正しい)。
- AL#179(blank canvas)の個別対応 — 同一根因(RF#453 の canvas 0 サイズ)
  のため pin 追従で canvas は正しい bounds を得る。本 spec の受入範囲は
  #180 とし、#179 の症状確認は別タスクで扱う。

## Requirements

- `Cargo.lock` の ruviform 解決 SHA が `863a7a9d` で単一であること
  (`mise run check-ruviform-lock` / `check-ruviform-rev` が pass)。
- `ui/vendor` が同 SHA で同期されていること(`mise run sync-ui-deps`)。
- headless/Fake・GPUI parity で Spectrogram mount・tab 切替・描画 plan
  を検証するテストが存在し pass すること。
- 実 OS window で Spectrogram tab の pointer 切替が完了し、window が
  応答し続ける証拠(pixel/log)を残すこと。

## Acceptance Criteria

- [x] Spectrogram tab を選択して切替が完了し、恒久的な hang が
  解消されること(pin 追従後の GPUI parity・実機で確認)。
- [x] headless/Fake と GPUI parity での Spectrogram mount・切替・
  描画 plan 検証(`gpui_parity` workspace tab test +
  `spectrogram_tab_paint` 回帰テスト)。
- [x] hang の root cause を特定し、正しい境界(上流 ruviform の修正を
  pin で取り込む)で対処。AL 側に恒久 workaround を置かない。
- [x] 実 OS window での再検証証拠(pixel/log)。
  `target/debug/al-desktop.exe`(debug build)で Dataset → Spectrogram の
  pointer 選択を実施し、`Responding=True`・CPU almost idle を確認。
  capture 画像(`48-spectro2.png`/`53-spectro3.png`)に spectrogram の
  heatmap が描画されている。証拠:
  `F:/repositry/audio-products/.devin/evidence/al180-spectrogram-hang/`。

## Deferred Design Concerns

- なし。cell 単位 `paint_quad` の O(n²) 描画コストは ruviform RF#461 /
  PR #462(`paint_layer` batch 化、Spectrogram 65,536 cells が headless
  実測 6.37s → 53.3ms)で解消済み。本 spec の pin `863a7a9d` はその
  merge commit を含む。
- 観測メモ: 実機検証中、window capture が断続的にほぼ単色
  `#2b5f9e` の frame になることがあった(プロセスは `Responding=True`
  で操作に応答し、次の present で実描画へ復帰)。GPUI の present /
  DirectComposition 経路の capture artifact の可能性があり、AL#179
  (canvas paint 空)の調査対象として別途記録する。

## Sources and Relations

- Issue #180(AL#180)、証拠 `F:/repositry/audio-products/.devin/evidence/al166-acceptance-verify/`。
- ruviform RF#453 / PR #459(canvas 0 サイズ是正)、RF#434/#442
  (Spectrogram kind)、ruviform#461(残課題)。
- `native-gui-workspace-tabs-selection.md`: workspace Tabs の
  `tab-select` 契約(本件の操作経路)。
