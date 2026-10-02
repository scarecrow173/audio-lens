---
id: "TSPEC-034z99BlV8EIVYk7jxmzM9"
type: "test-spec"
status: "proposed"
title: "解析ステージ検証"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/specs/cheap-analysis-features.md"
    - "docs/specs/reference-analysis.md"
    - "docs/specs/expensive-analysis.md"
    - "docs/specs/candidate-event-model.md"
  derived-by: []
  verifies:
    - "docs/specs/cheap-analysis-features.md"
    - "docs/specs/reference-analysis.md"
    - "docs/specs/expensive-analysis.md"
    - "docs/specs/candidate-event-model.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 解析ステージ検証

## Purpose

cheap / reference / expensive 3 ステージと dataset 追記プロトコル・
candidate/event モデル・確度伝播の契約を固定する。

## Feature

常時特徴・リファレンス比較・候補駆動高コスト解析と dataset への
永続化

## Rules

- cheap ステージはリファレンス素材なしで完走する
  (`--stages ingest,cheap_analysis,dataset` が受理される)
- 中間スペクトル(複素 STFT / PSD / 帯域)は `intermediate`
  ロールの tensor に保存され、後段は DatasetReader から
  再計算なしに再利用する
- 全特徴は resolution_class + 実 window/hop + validity +
  algorithm id/version/params を持つ
- 複数ステージが `analysis/` に追記でき、同じ (role,path) の
  再書き込みは置換される(manifest 重複なし)
- locate 結果は dataset の `locate` ロールに物化される
- R0〜R3 各残差に波形・STFT・PSD・帯域エネルギー・
  RMS/リファレンス比・ソース相関が保持される
- 低コヒーレンス帯域の伝達推定は `LowCoherence` で invalid、
  無音区間のゲインは `SilenceInsufficientEnergy` で invalid と
  なり、誤解を招く数値を出さない
- イベントは `{"artifact","rows"}` リンクで裏付け観測を指し、
  安定 id を持つ
- expensive 結果はトリガ理由を記録する。manual_ranges /
  always_run で起動できる
- `LocateStage` は source/capture 入力ゼロで空結果を返す
  (ステージ失敗ではない)

## Examples

- Given キャプチャのみの入力、When
  `ingest,cheap_analysis,dataset` を実行、Then cheap 特徴が
  `features/cheap` に保存されジョブが完了する
- Given 既知ゲインを掛けてソースを埋め込んだキャプチャ、
  When `ingest,locate,cheap_analysis,reference_analysis,dataset`
  を実行、Then 400Hz / 帯域ゲインが既知値を回復する
- Given EQ(ローパス)を掛けたキャプチャ、When reference を実行、
  Then 帯域別ゲインが高域で低下し低コヒーレンス帯域は
  `LowCoherence` で invalid になる
- Given ステージ書き込み中の失敗を注入、When resume する、
  Then 同じ (role,path) が重複登録されず完走する
- Given 無音区間を含むキャプチャ、When reference を実行、
  Then その区間のゲインは数値ではなく
  `SilenceInsufficientEnergy` で invalid になる
- Given `config.analysis.expensive.manual_ranges` を指定、
  When expensive を実行、Then 指定区間に解析が走り
  トリガ理由が記録される
- Given リファレンス類似度の低い区間、When expensive を実行、
  Then candidate トリガが記録され候補区間に解析が走る

## Guarantees

- dataset 追記は冪等で、manifest の (role,path) 重複が生じない
- manifest が破損した dataset を Completed ステージ成果物ごと
  消去しない(明示失敗)
- 低確度の入力が高確度の下流主張にならない(確度 min +
  validity 依存元マッピング)
- expensive は派生元の低レベル計測を上書きしない
  (別ロールに書く)

## Non-goals

- 各推定器の精度限界・パラメータ最適値
- 実ゲームキャプチャでの性能評価

## Risk

- 追記プロトコルの破綻は resume を永久失敗にする
- 確度伝播が漏れると「高確度だが根拠のないイベント」が量産される
- 中間スペクトル未保存だと後段の再計算なし再利用が成立しない
