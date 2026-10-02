---
id: "DESIGN-034yYLaqu0owt08BLocFDQ"
type: "design"
status: "approved"
title: "ソース定位・トラッキング設計"
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
    - "ingest-av.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/source-localization.md"
    - "../specs/locate-tracking.md"
    - "../adr/multi-method-locate-consensus.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ソース定位・トラッキング設計

## Context

ソース素材をキャプチャ内に定位し、ループ・長尺・ドリフト・
マルチチャンネル・残差探索を扱うアラインメント層を設計する。
spec `../specs/source-localization.md` と `../specs/locate-tracking.md`、
ADR `../adr/multi-method-locate-consensus.md` に基づく。

## Scope

### In Scope

- NCC / band-NCC / GCC-PHAT / スペクトル包絡による独立証拠収集
- 候補位置・スコア・ピーク特性・棄却理由の永続化
- 合議受理(少なくとも 2 手法一致 + 閾値)
- ループ反復の個別識別
- 長尺ソースの区分的 time map(anchor + segment)
- 多チャンネルソースの最小二乗混合推定と保存
- 支配的ソースの減算を挟む 2 段階残差探索

### Out of Scope

- 定位結果の意味的評価(バグ判定はしない)
- 定位精度を改善するための新規 DSP 研究
- UI 上のマッピング表示(可視化側の責務)

## Components and Boundaries

- `locate-methods` コンポーネント: 各手法の候補と証拠を生成する
  独立実行器群
- `consensus` コンポーネント: 手法ごとの証拠を集約し、閾値と
  位置一致で受理 / 棄却を決める
- `tracking` コンポーネント: ループ・長尺・ドリフト・スキップ・
  再開を表す anchor / segment / time map を生成
- `mixer-fit` コンポーネント: 多チャンネルソースの混合行列を推定
- `residual-search` コンポーネント: 支配的ソースを減算した残差上で
  二次定位を行う
- 境界: 入力は ingest の PCM / メタデータ、出力は locate 結果と
  alignment id・time map・残差情報を dataset に渡す

## Data and Control Flow

- ingest → decoded PCM + metadata
- 各 locate-methods が独立に候補と証拠を生成
- consensus が受理 / 棄却と確度を決定
- tracking が長尺ソース・ループ・ドリフトに対して anchor / segment /
  time map を生成
- mixer-fit が混合行列を推定して永続化
- residual-search が支配的ソースを減算し、残差を対象に再探索
- すべての結果に alignment id・手法・バージョン・確度を付与

## Non-Functional Constraints

- 偽受理を構造的に防ぐこと(単一手法の一発決定をしない)
- 失敗・棄却・低情報領域を明示的に記録すること
- 位置ずれ・ドリフト・再開を後段解析に伝搬可能な形で保持すること

## Risks and Trade-offs

- 多手法実行の計算コスト増 → 候補生成と受理を分離し、高コストな
  補助解析は後段へ回す
- 合議ルールの閾値調整難 → fixture で検証可能な形に保つ
- 長尺 / ループの複雑性 → 単一グローバルオフセットに潰さず
  anchor / segment で表現する設計に留める

## Deferred Design Concerns

- 各手法の実装詳細・パラメータデフォルト | 検証 fixture との
  対応で調整 | fixture 実装時 | 過少 / 過大な受理率
- 残差探索の繰り返し回数・順序戦略 | 実測での計算コストに依存 |
  実装フェーズ | 無駄な再探索

## Verification Notes

- [ ] 偽ピーク単独では受理されない
- [ ] ループ / 長尺で anchor・segment・time map が保持される
- [ ] 残差探索の減算来歴が追跡可能
