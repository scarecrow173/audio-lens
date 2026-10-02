---
id: "SPEC-034yYltmjZLj2DYX34Y56t"
type: "spec"
status: "proposed"
title: "ループ・長尺・ドリフト対応のトラッキング"
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
    - "source-localization.md"
  blocks: []
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

# ループ・長尺・ドリフト対応のトラッキング

## What Should Be Built

locate を一発マッチから現実のゲーム音声素材へ拡張する: ループ、
長尺のセリフ / 音楽、マルチチャンネルソース、ドリフト、スキップ、
より大きな持続音によるマスキング。

ループ素材:

- 必要に応じて 2〜3 個連結したループ反復を用いて探索する
- 保存するもの: ループ index / 出現位置、ソース位相 / 時間、
  キャプチャ時間
- 複数の正当な出現を 1 つの曖昧なマッチに潰さない

長尺素材のトラッキング(複数区間にわたるソース→キャプチャ対応):

```text
source_time -> capture_time
```

アンカーと推定セグメントを永続化し、マッピングが以下を表現できる
ようにする:

- 連続区間
- クロックドリフト / 速度不一致
- 局所ジッタ
- スキップされたソース区間(例: セリフ送り)
- キャプチャ側ギャップ
- 別ソース位置での再開 / 再アタッチ
- 未マッチ領域

不連続は観測であり、自動的にバグとはみなさない。

マルチチャンネルソースの扱い:

- 4ch 素材等について、最小二乗でキャプチャチャンネルの混合を推定する
- 符号付き係数を保持する(極性反転が観測可能であること)
- 適合品質と行列の条件数を記録する
- チャンネルが強く共線な場合にリッジ正則化をサポートする
- 推定した source→capture 混合行列を永続化する

2 段階の残差探索:

1. BGM のような支配的な持続ソースを定位する
2. アライン / 適合 / 減算する
3. アンビエンス等のより静かな素材を残差音声から探索する

減算の来歴を永続化する:

- 親ソース
- alignment id
- 推定ゲイン / チャンネル変換
- 減算前後の残差エネルギー
- 減算確度
- 子 locate 結果

## Why Now

ゲーム音声はループ BGM、長いセリフ、多チャンネル素材、
ドリフト / スキップが常態であり、一発マッチ前提の locate では
比較区間を張れない。アンカーとセグメントの分離が必要。

## Users and Value

- 解析者: ループ出現の個別識別、長尺マッピングの区間表現、
  マスクされた素材の残差探索ができる

## Scope

### In Scope

- ループ反復の個別識別
- 区分的な source→capture time map(アンカー + セグメント)
- 最小二乗による多チャンネル混合推定とその保存
- 2 段階残差探索と減算来歴

### Out of Scope

- 残差に対する意味的な原因推定
- 定位そのものの単発手法(`source-localization.md` を参照)

## Requirements

- 長尺ソースは 1 つのグローバルオフセットではなく区分的 time map を
  生成できる
- ループ出現は個別に識別可能
- 残差探索結果は支配的ソースがどう除去されたかを保持する

## Acceptance Criteria

- [ ] 長尺ソースが 1 つのグローバルオフセットではなく区分的
  time map を生成する
- [ ] ループ出現が個別に識別できる
- [ ] 残差探索結果が支配的ソースの除去方法を保持している

## Deferred Design Concerns

- ループ連結数や残差探索の順序付けポリシー | fixture での挙動確認が
  前提 | `validation-strategy.md` の fixture 実装時 | 過剰な残差探索が
  計算コストを圧迫するリスク

## Sources and Relations

`source-localization.md` の拡張。基礎手法と合議受理は
`source-localization.md`、混同行列の解析利用は
`reference-analysis.md` を参照。
