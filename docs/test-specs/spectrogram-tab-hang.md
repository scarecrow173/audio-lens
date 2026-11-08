---
id: "TSPEC-034TX9JbnubNmwchpJ23Hs"
type: "test-spec"
status: "approved"
title: "Spectrogram tab hang 解消のテスト仕様"
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
    - "docs/plans/spectrogram-tab-hang.md"
  derived-by: []
  verifies:
    - "docs/specs/spectrogram-tab-hang.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

## 検証方針

hang の根因(canvas 0 サイズ → BoundsTree O(n²))は ruviform 側にあり、
AL 側の検証は「pin 追従後に Spectrogram tab 選択→paint が完了する」
ことの確認と回帰防止に集中する。

## テスト

### 自動(実装済み・実行済み)

- `crates/al-gui/tests/spectrogram_tab_paint.rs`
  (`test-ui-gpui` lane): 本番解像度 512×128=65,536 cell の
  selectable Tabs で `select_tab(1)` → `settle` → `drawn` を検証。
  pin 退行時は `settle` が戻らず hang する。実測 pass(0.13s、
  pin `863a7a9d` では RF#461 の `paint_layer` batch 化により
  65,536 cells の paint が数十 ms)。
- `crates/al-gui/tests/gpui_parity.rs`
  `workspace_tab_selection_intent_and_selected_tab_match_between_fake_and_gpui`:
  drawn components に `SPECTROGRAM_KEY` を含むことを assertion 追加。
  実測 pass(gpui_parity 全 12 テストで 0.45s)。

### 手動・実機

- debug build `target/debug/al-desktop.exe` を起動し、Dataset workspace
  の Spectrogram tab を pointer で選択。tab 切替完了と spectrogram
  描画を pixel/log で確認済み(pin `863a7a9d`、
  `F:/repositry/audio-products/.devin/evidence/al180-spectrogram-hang/`:
  `48-spectro2.png`/`53-spectro3.png` に heatmap 描画、
  `Responding=True`・CPU almost idle を複数回確認)。
- 補足: capture が断続的にほぼ単色 `#2b5f9e` の frame を返すことが
  あったが、プロセスは応答し次の操作で実描画へ復帰。GPUI present/
  DComp 経路の capture artifact の可能性として AL#179 調査へ記録。

## 判定

- 恒久的 hang(60 秒超の無応答)が再現しないこと。
- `settle`/paint が有限時間で完了し drawn plan に spectrogram が
  含まれること。
