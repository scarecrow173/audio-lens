---
id: "SPEC-034yYltIfR72sgyFIMvsdh"
type: "spec"
status: "proposed"
title: "分析データセットスキーマ"
created: "2026-10-02"
updated: "2026-10-11"
owners: ["scarecrow173"]
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
  depends-on:
    - "analysis-job-model.md"
  blocks:
    - "cheap-analysis-features.md"
    - "reference-analysis.md"
    - "expensive-analysis.md"
    - "candidate-event-model.md"
    - "visualization-dataset.md"
    - "analysis-export.md"
  supersedes: []
  superseded-by: []
  related:
    - "../adr/analysis-dataset-storage.md"
  refines:
    - "audiolens-overview.md"
  refined-by: []
  derives-from:
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 分析データセットスキーマ

## What Should Be Built

将来の UI や AI コンシューマに依存しない、安定かつ情報保存型の
Analysis Dataset を定義する。

すべての派生計測が持ちうる結果形状:

- 特徴名 / スキーマ id
- 値またはテンソル位置
- 単位
- 時間範囲
- 周波数範囲(該当する場合)
- チャンネル / チャンネルペア
- 分解能 / window / hop / sample_rate (Hz)。`Resolution.sample_rate` は
  必須の `u32` で、window/hop を計算した PCM のレートを表す。
  正規化後の解析では原入力レートではなく正規化レートを保存する。
  窓を持たない segment 要約等は resolution 自体を省略できる。
  周波数軸と窓の時間換算はこのレートを使用し、原入力レート等の来歴は
  manifest/params に保持する。旧成果物は再生成し、欠落値の fallback は設けない。
- 絶対的な対象値
- リファレンス値(該当する場合)
- 差分および / または比率(意味がある場合)
- 確度
- validity / ステータス
- 無効 / 低確度の理由
- サポート数 / 有効窓数
- アルゴリズム id / バージョン
- パラメータ
- 入力 id
- alignment id
- 親 / 中間物の系譜

validity セマンティクス(最低限の区別):

- valid
- silence / insufficient energy
- insufficient support
- low correlation
- low coherence
- ambiguous alignment
- out of supported frequency range
- decode/input unavailable
- not applicable
- computation failed

「異常なし」は「計測不能」と同じ状態で表現してはならない。

保存ロール:

- JSON: manifest、スキーマ、設定、来歴
- Parquet: フレーム / セグメント特徴、locate マッチ、イベント
- Zarr 等のチャンクテンソル形式: STFT、PSD/CSD/コヒーレンス /
  伝達関数テンソル
- WAV / FLAC: デコード済み / アライン済み / 残差音声。
  現行 audio store は WAV を実装し、FLAC 保存は #33 の承認済み defer。
  Float64 WAV の残差は再取得時に解析サンプルを保持し、音声 artifact と
  親 source/alignment・混合行列の provenance を辿れる。
  RIFF サイズ上限を超える保存はエラーとする。FLAC や大容量コンテナの
  新依存・方式選定は別途承認する ([判断記録](../tasks/spec-residual-decisions.md))。

推奨レイアウト:

```text
analysis/
  manifest.json
  inputs/
  locate/
  intermediate/
  features/
    cheap/
    reference/
    expensive/
  residual/
  events/
  visualization/
```

バージョニング:

- 生成したアルゴリズム id / バージョン / パラメータは来歴として記録する
- リリース前のため後方互換は保証しない。形式変更時は互換層や
  バージョン判定を設けず、データセットを作り直す

## Why Now

後段の全仕様(特徴・イベント・可視化・エクスポート)が共通して
読み書きする中核契約であり、他領域より先に形を決める必要がある。

## Users and Value

- データセットのコンシューマ(可視化、エクスポート、将来のツール):
  特徴がどう生成されたかを正確に把握できる
- 解析者: 大規模テンソルを全体読み込みなしに範囲 / チャンクで参照できる

## Scope

### In Scope

- 結果形状、validity セマンティクス、保存ロール、推奨レイアウト
- アルゴリズム id / バージョン / パラメータの来歴記録
- 形式変更時の作り直し方針(互換層・マイグレーションは持たない)

### Out of Scope

- 個別特徴の具体的な計算式(各解析 spec を参照)
- UI や AI コンシューマ固有のスキーマ

## Requirements

- コンシューマが特徴の生成経緯を正確に判断できる
- 大規模テンソルは全体を読み込まずチャンク / 範囲で読める
- スカラー要約が、保持が要求される中間データの代替にならない

## Acceptance Criteria

- [ ] 任意の特徴について、生成に使われた入力・アルゴリズム・
  パラメータ・alignment を一意に辿れる
- [ ] 大規模テンソルを全体読み込みなしにチャンク / 範囲で読める
- [ ] 保持が要求される中間データがスカラー要約に置き換えられていない

## Deferred Design Concerns

- Parquet / Zarr 相当の具体的実装とエンコード詳細 | Rust 実装での
  選定に依存(`../adr/implementation-stack.md`) | 実装フェーズ |
  形式変更コスト(互換層は設けない)

## Sources and Relations

全体仕様の Analysis Dataset 要件を细化。保存ロールとレイアウトの
決定は ADR `../adr/analysis-dataset-storage.md` を参照。
