---
id: "SPEC-034yYlsZHRt4b5fTzZS7Qr"
type: "spec"
status: "proposed"
title: "マルチメソッドソース定位"
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
    - "media-ingest.md"
  blocks:
    - "locate-tracking.md"
    - "reference-analysis.md"
  supersedes: []
  superseded-by: []
  related:
    - "../adr/multi-method-locate-consensus.md"
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

# マルチメソッドソース定位

## What Should Be Built

キャプチャ音声内のソース素材を、比較区間を確立できる程度に堅牢に
定位する。これはアラインメント基盤であり、類似度の評決ではない。

必須手法(独立した定位証拠を実行する):

- フルバンド正規化相互相関(NCC)
- 選択バンドパス信号上の NCC
- GCC-PHAT / PHAT 系の遅延証拠
- スペクトル包絡マッチング

候補 / 受理データ(すべての手法・候補について保存):

- キャプチャ内候補位置
- スコア / ピーク値
- ピーク対中央値(または同等の背景)比
- ピーク幅
- 第 1 / 第 2 ピーク比(有用な場合)
- validity フラグと棄却理由

合議による受理(既存原則に従う):

- ピーク / 背景証拠が設定可能な閾値を超えること
- 少なくとも 2 つの独立手法が設定可能な位置許容差内で一致すること

各手法の出力を最終受理位置だけに縮約せず、不一致を保存する。

明示的に扱うべき既知の失敗モード:

- 無音境界
- 強い周期性を持つ信号
- 広い / 曖昧なピーク
- 情報量の少ない窓
- 偽相関ピーク

出力(locate 結果に含めるもの):

- ソース素材 id
- ソース区間
- キャプチャ区間
- 手法ごとの証拠
- 手法一致数
- 位置のばらつき
- 確度 / validity
- アルゴリズム / バージョン / パラメータ

## Why Now

比較解析(reference / expensive)は正しいアラインメントが前提であり、
単一手法の偽ピークをそのまま受理すると下流の全計測が誤った区間で
行われる。定位証拠の多重度と棄却理由の保存が必要。

## Users and Value

- 解析者: 受理された位置だけでなく、各手法の不一致や棄却理由まで
  検証できる
- 下流ステージ: alignment id と確度に基づき比較区間を選択できる

## Scope

### In Scope

- 上記 4 系統の独立定位手法
- 候補ごとの証拠・棄却理由の永続化
- 合議受理ルールと閾値
- 既知失敗モードの明示的フラグ

### Out of Scope

- ループ / 長尺 / 残差探索などの追跡的拡張(`locate-tracking.md` に分離)
- 定位結果の正誤を最終判定するヒューリスティック

## Requirements

- 1 手法の偽ピークだけが暗黙に受理マッチにならない
- 低情報 / 無音区間は通常の失敗ではなく明示的にマークされる
- 手法ごとの位置は後のデバッグ・解析のために保持される

## Acceptance Criteria

- [ ] 1 手法のみの偽ピークが受理マッチとして黙って採用されない
- [ ] 低情報 / 無音区間が明示的にマークされる
- [ ] 手法ごとの位置が後段のデバッグ・解析で参照できる

## Deferred Design Concerns

- 各手法の具体的パラメータデフォルトと帯域分割 | 検証 fixture との
  対応で調整 | `validation-strategy.md` の fixture 実装時 |
  閾値未調整による偽受理

## Sources and Relations

全体仕様の locate 段(基礎)を细化。合議受理ルールの決定は ADR
`../adr/multi-method-locate-consensus.md` を参照。追跡的拡張は
`locate-tracking.md` に記録する。
