---
id: "DESIGN-034yYLaOKRSfRJMRKiYgQV"
type: "design"
status: "approved"
title: "分析データセット設計"
created: "2026-10-02"
updated: "2026-10-02"
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
