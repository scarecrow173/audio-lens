---
id: "DESIGN-034yYLaOKRSfRJMRKiYgQV"
type: "design"
status: "approved"
title: "分析データセット設計"
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
    - "job-orchestration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/analysis-dataset.md"
    - "../adr/analysis-dataset-storage.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 分析データセット設計

## Context

すべての解析成果物を受け止める Analysis Dataset の構造を設計する。
spec `../specs/analysis-dataset.md` と ADR
`../adr/analysis-dataset-storage.md` に基づき、表形式・テンソル・
音声・メタデータを役割分割して保持する。

## Scope

### In Scope

- manifest.json を中心とするデータセットルート構造
- 特徴 / イベント / locate 結果の Parquet 表現
- STFT / PSD / CSD / コヒーレンス / 伝達関数のチャンクテンソル保存
- デコード済み / アライン済み / 残差音声の WAV・FLAC 保持
- validity・確度・サポート数・来歴を持つ共通スキーマ
- スキーマバージョンとアルゴリズムバージョンの分離

### Out of Scope

- 各特徴の具体的な算出式(解析設計側)
- 可視化向けフォーマット(visualization 側)
- 保存ライブラリの具体的選定

## Components and Boundaries

- `dataset-writer` コンポーネント: 各ステージからの成果物を正規化し、
  manifest + 各ロール別ストアへ保存する
- `dataset-reader` コンポーネント: manifest を介して範囲 / チャンク /
  来歴参照を提供する
- `provenance` コンポーネント: 入力 id・algorithm id・バージョン・
  パラメータ・alignment id・親子関係を記録する
- 境界: dataset は中間表現と最終成果物の双方の唯一の正本であり、
  解析ステージは直接他ステージの生データを読まない

## Data and Control Flow

- 各ステージの出力 → dataset-writer が role 別ストアに振り分け
- manifest.json に入力 / 設定 / スキーマバージョン / 各成果物 id を
  集約
- downstream(可視化・エクスポート・再解析)は dataset-reader 経由で
  参照
- テンソルは chunk / range 読み出しを前提に保持し、スカラー要約を
  代替として扱わない

## Non-Functional Constraints

- 参照透過性: すべての派生値が生成経緯へ遡れる
- 部分読み出し: 大規模テンソルは全体読み込み不要
- 作り直し前提: リリース前のため互換層・バージョン管理は持たず、
  形式変更時はデータセットを再生成する
- 確度保全: 「計測不能」と「異常なし」を分離する validity 体系

## Risks and Trade-offs

- 複数フォーマット管理の複雑化 → manifest を唯一の入口にして
  役割を明確化
- 形式変更時の影響範囲 → リリース前のため互換維持は行わず、
  変更時はデータセットを作り直す
- 中間データ肥大 → 保持が要求されるものと一時的なものを分離する

## Deferred Design Concerns

- Parquet / Zarr 相当実装の具体選定 | Rust エコシステム内での実装
  (外部 crate または自前)と性能計測に依存
  (`../adr/implementation-stack.md`) | 実装着手時 | 読み出し性能不足
- 巨大キャプチャでの chunk 粒度 | 実データでの計測が前提 |
  実装フェーズ | メモリ使用量増大

## Verification Notes

- [ ] manifest から各成果物の生成経緯を再構成できる
- [ ] テンソルを全体読み込みなしに部分範囲で読める
- [ ] スキーマ / アルゴリズムバージョンが別々に管理される

## キャプチャ間経路差の job スコープ

- `features/cross_capture/<reference_capture_id>__<target_capture_id>.json`
  に `CaptureComparison` を保存する。単一 input や source/capture ペアの
  観測ではないため job 軸を採用し、record と provenance の `input_id` は
  `job`、`alignment_id` は null とする。既存 `features/reference` /
  `features/expensive` のペア観測は変更しない。
- JSON は `reference_capture_id`、`target_capture_id`、
  `common_source_ids`、共通の `FeatureRecord` 配列 `records` を持つ。
  `capture_reference` を基準側に優先し、同一 role の場合は input id 順で
  全キャプチャの非順序組を1回ずつ出力する。比率の向きは常に target / reference。
- `cross_capture.path_gain` は両キャプチャに accepted かつ valid な locate が
  ある同一 source id の `transfer_h` の振幅比を使用する。位相は locate の
  時刻原点に依存するため集約せず、振幅の経路差に限定する。
- 大きな周波数列は `intermediate/cross_capture/<組>/path_gain` の
  チャンクテンソル `[3, fft_size/2+1]` に置き、record の tensor 参照から読む。
  行は `magnitude_ratio`、`gain_difference_db`、`source_support`。
  bin k の周波数は `k * sample_rate / fft_size` (DC/Nyquist を含む)。
- 各 bin の有限かつ正の振幅比をソース間で等重みの中央値に集約する。
  偶数個は中央2値の平均、dB 差は `20 * log10(median_ratio)`。
  ゼロ振幅・非有限値・両経路のいずれかの magnitude-squared coherence が
  0.5 未満の bin は除外する。欠損セルは NaN、support は 0 とし、
  record の `support_count` は支持された bin 数を示す。
- rate/FFT size が同じ grid だけを比較し、異なる grid は補間しない。
  H と coherence は locate の alignment に対応する成果物を参照する。
  共通ソースなしは `unsupported`、共通ソースがあっても H/coherence が欠落、
  grid 不一致、支持 bin なしは `insufficient_support` として保存する。
  比較未実施・計測不能を経路差ゼロと同一視しない。
- algorithm は `cross_capture.path_gain.v1`。median、行定義、rate、FFT size、
  使用可能ソース数を params に、locate artifact id と H/coherence の
  dataset-relative path を lineage / provenance.parents に記録する。
  confidence は寄与した locate の最弱値と Medium の最小値に制限する。
