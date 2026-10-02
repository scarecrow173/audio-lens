---
id: "DISC-034yYFlTHXyXnjrEm19mGq"
type: "discovery"
status: "resolved"
title: "AudioLens 要件取りまとめ"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source:
    - "../../AGENTS.md"
    - "../../README.md"
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
  derives-from: []
  derived-by:
    - "../specs/audiolens-overview.md"
    - "../specs/analysis-job-model.md"
    - "../specs/media-ingest.md"
    - "../specs/source-localization.md"
    - "../specs/locate-tracking.md"
    - "../specs/analysis-dataset.md"
    - "../specs/cheap-analysis-features.md"
    - "../specs/reference-analysis.md"
    - "../specs/expensive-analysis.md"
    - "../specs/candidate-event-model.md"
    - "../specs/video-av-timing.md"
    - "../specs/visualization-dataset.md"
    - "../specs/validation-strategy.md"
    - "../specs/analysis-export.md"
    - "../adr/measurement-over-verdict.md"
    - "../adr/analysis-dataset-storage.md"
    - "../adr/multi-method-locate-consensus.md"
    - "../adr/immutable-resumable-job.md"
    - "../adr/derived-visualization-dataset.md"
    - "../adr/implementation-stack.md"
    - "../designs/overview.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AudioLens 要件取りまとめ

## Exploration Goal

ゲーム音声デバッグのために「キャプチャ映像と比較用入力をアラインし、
音声に何が起きたかを計測して構造化データを出力するツール」の要件
全体を正本ドキュメント群へ昇格させるための取りまとめ。プロジェクト
初期に収集された要件を、領域別 spec と横断 ADR に分配する。

## Key Issues and Assumptions

- 対象は単一の類似度スコアでは切り分けられない多層の音声劣化であり、
  観測の保持が解析の主目的であること
- ソース定位(alignment)が比較解析の前提であり、定位の不確かさを
  下流へ伝播させる必要があること
- 入力は対象キャプチャ 1 つ + 任意個数のリファレンス / 素材であり、
  ロールで一般化されること
- 保存形式は表 / テンソル / 音声 / メタデータで役割が異なり、
  単一形式への統一は適さないこと
- 可視化・エクスポートは解析データの派生物であり、新しい意味を
  発明しないこと

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| 評決型解析器(合否 / severity を出力) | 即答性 | 未知の失敗を再解釈不能 | no |
| 観測型解析器 + 下流コンシューマ | 再解釈可能・来歴保全 | 評決は別レイヤが必要 | yes |
| 単一ストレージ形式 | 実装単純 | テンソル / 音声の特性を犠牲 | no |
| 役割別ストレージ分割 | クエリ性・可搬性 | フォーマット管理の複雑さ | yes |

## Tentative Conclusions and Hypotheses

- confirmed: コア解析器は観測のみを出力し、評決・原因推定・
  総合スコアを持たない
- confirmed: 解析パイプラインは Ingest → Locate → Cheap → Reference →
  Candidate → Expensive → Dataset → Visualization に分解される
- confirmed: データセットは JSON / Parquet / チャンクテンソル /
  WAV・FLAC の役割分割で保持する
- hypothesis: 実装言語・ランタイム・DSP ライブラリは未決定であり、
  開発環境選定フェーズで別途決める

## Open Questions

- 実装言語 / ランタイム / DSP ライブラリ — ユーザーが開発環境選定
  フェーズで決定
- 対応コンテナ / コーデック辞書の初期内容 — インジェスト実装時

## Promotion Candidates

- [x] spec-doc needed: 全体仕様 + 13 領域の要件を `docs/specs/` に作成済み
- [x] adr-doc needed: 観測主義・ストレージ分割・locate 合議・
  不変ジョブ・可視化派生の 5 件を `docs/adr/` に作成済み

## Sources and Relations

プロジェクト初期の要件取りまとめを正本化した記録。派生先は
`derived-by` に列挙する各 spec / ADR。
