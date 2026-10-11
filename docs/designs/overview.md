---
id: "DESIGN-XhBzf2DS7KALGUBXv2xcuH"
type: "design"
status: "draft"
title: "AudioLens 設計概要"
created: "2026-10-02"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by:
    - "job-orchestration.md"
    - "ingest-av.md"
    - "localization.md"
    - "dataset-schema.md"
    - "analysis-stages.md"
    - "visualization-export.md"
    - "simd-dsp-foundation.md"
  derives-from:
    - "../specs/audiolens-overview.md"
    - "../discovery/initial-requirements-consolidation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AudioLens 設計概要

設計文書の入口。全体仕様 `../specs/audiolens-overview.md` を
レイヤー別の詳細設計に分解し、各設計間の依存関係を示す。

## 設計ドキュメント構成

| 設計 | 概要 | 主な上流 |
| --- | --- | --- |
| [job-orchestration.md](./job-orchestration.md) | 不変ジョブ・ステージ実行・部分再開 | ジョブモデル spec、不変ジョブ ADR |
| [ingest-av.md](./ingest-av.md) | デコード・タイミング完全性・AV 同期 | ingest spec、AV タイミング spec |
| [localization.md](./localization.md) | 多手法定位・ループ・残差探索 | 定位 spec、トラッキング spec、合議 ADR |
| [simd-dsp-foundation.md](./simd-dsp-foundation.md) | SIMD 算術基盤・DSP 移行・定位入力準備 | 定位 spec、pulp ADR |
| [dataset-schema.md](./dataset-schema.md) | manifest・表・テンソル・音声の構造 | データセット spec、ストレージ ADR |
| [analysis-stages.md](./analysis-stages.md) | cheap / reference / expensive・イベント | 解析系 spec、観測主義 ADR |
| [visualization-export.md](./visualization-export.md) | 可視化派生データ・サマリ・パッケージ | 可視化・エクスポート・検証 spec、派生 ADR |

| [residual-fixes.md](./residual-fixes.md) | #28〜#33 の残存契約修正・仕様判断(draft) | 残件 spec、責務境界 ADR |

## 層構造と依存

```text
job-orchestration (不変ジョブ / ステージ計画)
  ├─ ingest-av            … 入力 → PCM + メタデータ
  ├─ localization         … PCM → アラインメント (depends: ingest-av)
  ├─ dataset-schema       … 全成果物の正本 (depends: job-orchestration)
  ├─ analysis-stages      … 3層解析 + イベント (depends: localization, dataset-schema)
  └─ visualization-export … dataset → 表示用・配布用成果物
                              (depends: dataset-schema, analysis-stages)
```

## 横断の設計原則

- 観測主義: コアは計測のみ出力し、判定語彙は持たない
  (`../adr/measurement-over-verdict.md`)
- 不変ジョブと安定 id による再現性・部分再開
  (`../adr/immutable-resumable-job.md`)
- 多手法合議による定位の偽受理防止
  (`../adr/multi-method-locate-consensus.md`)
- 役割分割されたストレージと来歴の完全保持
  (`../adr/analysis-dataset-storage.md`)
- 可視化は解析意味を持たない派生データ
  (`../adr/derived-visualization-dataset.md`)

## 未決定事項

- 実装言語は Rust に決定済み(`../adr/implementation-stack.md`)。
  toolchain / crate 構成の具体化は `mise.toml` と開発環境整備で行う
- 各詳細設計の status は `draft`。少なくとも 1 件の詳細設計が
  レビュアー承認(`approved`)されるまでは planning に進まない
