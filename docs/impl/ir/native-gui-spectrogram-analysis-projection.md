---
id: "IMPL-Mx9Vu5Ce3ZrO7sYgK4nLjQ"
type: "impl"
status: "completed"
title: "Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity 実装記録(IR)"
created: "2026-11-08"
updated: "2026-11-08"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/166"
  changes:
    - "../../specs/native-gui-spectrogram-analysis-projection.md"
  implements:
    - "../../specs/native-gui-spectrogram-analysis-projection.md"
  refines: []
  refined-by: []
  derives-from:
    - "../../tasks/native-gui-spectrogram-analysis-projection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity 実装記録(IR)

## 実装したこと

- **Ruviform rev 更新**: `node tools/update-ruviform-rev.mjs 627e4bc2…` で `Cargo.lock` の 20 package を
  `c31dc67c` から `627e4bc2` へ。差分は source SHA のみ(20 行)。
- **Spectrogram 射影**(`view::screen::spectrogram_screen`): `ruviform-viz` の `Tile` を列ごとに
  `SpectrogramColumn` へ 1 対 1 で写す。値と gap(NaN セル・`count == 0` 列)は再計算しない。
  列の frame 区間は `Spectrogram::tile` の等分割と同じ式、`count` は列の有限セル数。
  表示範囲 Label(`frames a..b of N`)を併せて置く。
- **Job/Run/Analysis 射影**: `view::run_screen`(状態 Label・stage 表・job 一覧)と
  `view::analysis_screen`(events 節・alignment 節)。stage の失敗理由と実行時間を保持し、
  非有限の数値は欠損セル、行数は `MAX_TABLE_ROWS` で拒否。
- **Tabs 画面と al-desktop mount**(`view::workspace::workspace_screen`): dataset 引数を指定した場合、
  Source・Spectrogram・Analysis の 3 tab を持つ Tabs を mount する。Spectrogram と Analysis は
  節・tab 単位で読み、読めない部分は理由の Label に置き換える。Source の失敗のみエラー。
- **共通 helper**: `view::mod` に `tag`・`text_cell`・`num_cell`・`label`・`table`・`panel` を集約し、
  `job.rs` の private `tag` を除去。
- **drawn parity**: `tests/gpui_parity.rs` で `GpuiObserver::drawn` の Meter bar・TimeAxis mark・
  Waveform bin を、mount した spec の payload から導いた Fake 期待値と照合。gap bin の除外も検証。
- **上流 Issue**: scarecrow173/ruviform#442(drawn plan に Spectrogram・Label・Table を追加)、
  #443(Tile → SpectrogramColumn の列区間・count の規則を上流契約に)を起票。

## 検証結果(merge 前 HEAD で実行)

| コマンド | 結果 |
| --- | --- |
| `mise run fmt` | pass |
| `mise run check` | pass |
| `mise run lint`(biome + clippy `-D warnings`) | pass(biome の warning は既存の tools/ 配下のみ、本変更外) |
| `mise run test`(workspace) | pass(al-gui: spectrogram 6・analysis_screen 5・view 12(e2e の tab 検証を含む)・screen 12 ほか全件 failed 0) |
| `mise run build` | pass |
| `mise run check-ui-gpui` / `lint-ui-gpui` | pass |
| `mise run test-ui-gpui` | pass(gpui_parity 5 件、うち drawn 照合 2 件、analysis 5、spectrogram 6、screen 12 ほか) |
| `mise run check-cli-headless` | pass |
| `mise run check-ruviform-lock` | pass(`627e4bc2`、20 packages) |
| `mise run check-ruviform-rev` | pass(REV match) |
| `mise run sync-ui-deps` | pass(ui/vendor を 627e4bc2 で同期) |
| `mise run check-ui` / `test-ui` | pass(typecheck pass、node:test 112 件 pass・fail 0) |
| GPUI 境界の確認 | `ruviform_ui_gpui` は `bin/al-desktop.rs` と gpui-test 限定の `tests/gpui_parity.rs` のみ |

- `mise run setup-ui` を一度実行して `ui/node_modules` を復元した(`check-ui` の前提)。
- 数値 oracle(Waveform bit 同一・gap の NaN・count 0・Meter peak)は既存テストが非回帰で通る。

## 独立レビュー(review-protocol)の結果

- 独立した read-only サブエージェントが diff と upstream 627e4bc2 の契約(`check_spectrogram`・
  `table` 検証・`collect_drawn`・`viewport_fraction`・`spectrogram.rs::tile`)を照合。
- 判定: **blocker なし**。列区間の式は上流 `Spectrogram::tile` と一致、`count` の意味と mount 検証の通過、
  tab 間のキー衝突なし、drawn 期待値は contract の描画規則の写しで空の期待値を通さない、
  GPUI 型の漏えいなし、数値 oracle の変更なし。
- 指摘(非ブロッキング):
  - `run_screen` の `zip` は `RunOutcome` の 3 配列が等長であることに依存する。`from_json` は常に
    等長にするため現状は到達しない。フィールドは `pub` のため将来の直接構築に注意する。
  - 列区間の `span * (c + 1)` は非常に長い frame 範囲で u64 オーバーフローしうる。上流も同じ式のため
    複製の既知の性質として ruviform#443 に記録する。
  - `alignments_of` が呼ぶ `DatasetSession::locate()` は `locate/summary` 全行を読む既存の service メソッド。
    本 slice の新規コードではないが、境界を持たない読み出しとして残課題に記録する。

## 逸脱・残課題

- **列の `count` の意味**: `SpectrogramColumn::count` の契約は「有限 sample 数」だが、`Tile` から得られない
  ため「列の有限セル数」とした。`count == 0` ⇔ 全セル NaN の gap 判定は両解釈で一致する。上流の判断は
  ruviform#443。
- **列区間の複製**: `Spectrogram::tile` の等分割規則を `view::screen::spectrogram_columns` に複製した。
  テストは上流 `tile()` の実出力に対して固定する。上流契約化(ruviform#443)後に差し替える。
- **Job/Run の mount は未実施**: al-desktop は job の選択と run 結果の入力源を持たない。射影関数と
  テストまでで、画面への mount は AL-G4 へ送る。
- **Spectrogram・Label・Table の描画 parity は NotRun**: `DrawnPlan` に該当 variant が無く観測できない
  (ruviform#442)。Meter・TimeAxis・Waveform のみ照合した。
- **Spectrogram の theme token は未実施**: viz の既存色を使う(ruviform#434 F3)。
- **Spectrogram の読み出し上限**: 先頭 4096 STFT frame と 128×512 セル。表示は Label で明示する。全量表示は
  上流の LOD 相当の API が必要で本 slice の対象外。
- **実 OS window の表示確認は NotRun**: この作業環境では窓を開いて確認していない。
- **merge 後 HEAD での最終検証は NotRun**: マージは本ワーカーの範囲外。coordinator の merge 後に実施する。
- **AL-G4 / AL-G5 は対象外**: 実機の Save/Reload・性能・visual regression は別 slice。
