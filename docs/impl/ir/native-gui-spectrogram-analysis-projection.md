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
    added:
      - type: "doc"
        path: "docs/specs/native-gui-spectrogram-analysis-projection.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Native GUI Spectrogram・Job/Run/Analysis の ViewSpec 射影と drawn parity 実装記録(IR)

## Summary

ruviform-viz の ViewSpec へ Spectrogram・Job/Run/Analysis 画面を射影し、
Tabs 画面を al-desktop へ mount した Native GUI slice の実装記録。
Meter・TimeAxis・Waveform の drawn parity まで検証した。Issue #166 の一部。

## Context

Issue #166(https://github.com/scarecrow173/audio-lens/issues/166)の slice
として、task `docs/tasks/native-gui-spectrogram-analysis-projection.md` と
spec `docs/specs/native-gui-spectrogram-analysis-projection.md`(implements)
に従って実施した。上流 ruviform へは scarecrow173/ruviform#442
(drawn plan に Spectrogram・Label・Table を追加)と #443(Tile →
SpectrogramColumn の列区間・count の規則を上流契約に)を起票した。

## Implementation

### Changes

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

### Decisions During Implementation

- **列の `count` の意味**: `SpectrogramColumn::count` の契約は「有限 sample 数」だが、`Tile` から得られない
  ため「列の有限セル数」とした。`count == 0` ⇔ 全セル NaN の gap 判定は両解釈で一致する。上流の判断は
  ruviform#443。
- **列区間の複製**: `Spectrogram::tile` の等分割規則を `view::screen::spectrogram_columns` に複製した。
  テストは上流 `tile()` の実出力に対して固定する。上流契約化(ruviform#443)後に差し替える。

## Related Experiments

本 slice に関連する実験ログは存在しない。`metadata.experiments` の
adopted / rejected はいずれも空で、`docs/impl/exp/` の既存 jsonl
(issue-139 / 143 / 148 / 149 / 153 の CI 関連記録)は本件と無関係。

### Adopted

- なし

### Rejected

- なし

## Validation

merge 前 HEAD で実行した検証結果:

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

### 独立レビュー(review-protocol)

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

## Risks

- `run_screen` の `zip` は `RunOutcome` の 3 配列が等長であることに依存する。
  `from_json` は常に等長にするため現状は到達しないが、フィールドは `pub` のため
  将来の直接構築に注意が必要(独立レビューの非ブロッキング指摘)。
- 列区間の `span * (c + 1)` は非常に長い frame 範囲で u64 オーバーフローしうる。
  上流 `Spectrogram::tile` も同じ式のため、複製の既知の性質として
  ruviform#443 に記録済み。
- `alignments_of` が呼ぶ `DatasetSession::locate()` は `locate/summary` 全行を
  読む既存の service メソッドで、境界を持たない読み出し(本 slice の新規コード
  ではないが残課題として記録)。
- Spectrogram の表示は先頭 4096 STFT frame・128×512 セルに制限される。
  表示は Label で明示する。全量表示は上流の LOD 相当の API が必要で本 slice の
  対象外。
- drawn parity の照合対象は Meter・TimeAxis・Waveform のみ。
  Spectrogram・Label・Table は `DrawnPlan` に該当 variant が無く観測できない
  (ruviform#442)。

## Follow-ups

- Job/Run/Analysis の al-desktop mount は未実施。al-desktop は job の選択と
  run 結果の入力源を持たないため、射影関数とテストまでで、画面への mount は
  AL-G4 へ送る。
- Spectrogram・Label・Table の描画 parity は ruviform#442 で `DrawnPlan` に
  variant が追加された後に拡張する。
- 列区間の複製と `count` の意味(上記 Decisions)は、ruviform#443 の上流契約化・
  上流判断の確定後に差し替える。
- Spectrogram の theme token 適用は未実施。viz の既存色を使い、
  ruviform#434 F3 で扱う。
- 実 OS window の表示確認は NotRun。この作業環境では窓を開いて確認していない。
- merge 後 HEAD での最終検証は NotRun。マージは本ワーカーの範囲外で、
  coordinator の merge 後に実施する。
- AL-G4 / AL-G5(実機の Save/Reload・性能・visual regression)は別 slice。
