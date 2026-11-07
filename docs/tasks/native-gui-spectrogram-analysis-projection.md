---
id: "TASK-Lw4Ts8Bd2YqN6rXfJ3mKiP"
type: "task"
status: "in-progress"
title: "Issue #166 Spectrogram・Job/Run/Analysis 射影と drawn parity 実装タスク (第4 slice)"
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
    - "../plans/native-gui-spectrogram-analysis-projection.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/native-gui-spectrogram-analysis-projection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #166 Spectrogram・Job/Run/Analysis 射影と drawn parity 実装タスク

## T1: ruviform rev を 627e4bc2 へ更新

- [x] `node tools/update-ruviform-rev.mjs 627e4bc27784e745f8ec9af909b42d871b63f8aa`
- [x] `mise run check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps` / `check-ui`(`setup-ui` で node_modules を復元)

## T2: Spectrogram 射影

- [x] テストを先に作成(`tests/spectrogram.rs`、6 件)
- [x] `view::screen::spectrogram_screen`(Tile → SpectrogramSpec、列区間・count・gap)
- [x] 形状検証(values 長 ≠ rows × cols の拒否)

## T3: Job/Run/Analysis の Table・Label 射影

- [x] テストを先に作成(`tests/analysis_screen.rs`、5 件)
- [x] `view::run_screen`(状態 Label・stage 表・job 一覧)
- [x] `view::analysis_screen`(events 節・alignment 節・画面)
- [x] 共通 helper(`view::mod`)へ `tag` を集約し、`job.rs` の複製を除去

## T4: Tabs 画面と al-desktop mount

- [x] `view::workspace::workspace_screen`(Source/Spectrogram/Analysis の Tabs、節単位の fallback)
- [x] `al-desktop` を `workspace_screen` の mount へ置換
- [x] `tests/view.rs` の e2e で 3 tab の mount を検証(active の切替ごとに mount)
- [ ] 実 OS window での表示確認(NotRun: この作業環境では窓を開いて確認していない)

## T5: drawn parity

- [x] `tests/gpui_parity.rs` に Meter・TimeAxis・Waveform の Fake 期待値照合を追加
- [x] gap の Waveform bin が描かれないことの検証
- [ ] Spectrogram・Label・Table の drawn 照合(NotRun: `DrawnPlan` に無い。ruviform#442)

## T6: 上流ギャップの Issue 化

- [x] RF#442: GpuiObserver::drawn に Spectrogram・Label・Table の描画 plan を公開
- [x] RF#443: Tile → SpectrogramColumn の写し替え規則(列区間・count)の上流契約化

## T7: 未達項目

- [ ] Job/Run の画面 mount(al-desktop に job 選択・run 入力源が無い。AL-G4 へ送る)
- [ ] Spectrogram の theme token(ruviform#434 F3)
- [ ] 上流の列区間規則へ置換(ruviform#443 解消後に `spectrogram_columns` を差し替える)

## T8: 検証と完了

- [x] `mise run fmt` / `check` / `lint` / `test` / `build`(merge 前 HEAD で実行)
- [x] `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui`
- [x] `mise run check-cli-headless` / `check-ruviform-lock` / `check-ruviform-rev` / `sync-ui-deps` / `check-ui` / `test-ui`
- [x] review-protocol(独立サブエージェントによる diff レビュー)。blocker なし(IR の記録参照)
- [ ] PR 作成(日本語、`Refs #166`、Issue は close しない)と head SHA の報告
- [ ] merge 後 HEAD で最終検証(NotRun: マージは本ワーカーの範囲外。coordinator の merge 後に実施)
