---
id: "SPEC-034yYlEMw8LnaoJcOc4v9z"
type: "spec"
status: "proposed"
title: "高コスト詳細解析"
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
    - "candidate-event-model.md"
    - "reference-analysis.md"
    - "analysis-dataset.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# 高コスト詳細解析

## What Should Be Built

正当な理由がある区間にのみ高コスト解析を実行しつつ、未知のバグが
自動ヒューリスティックに完全に門前払いされないよう、手動 /
always-run 実行も許可する。

トリガモード(いずれも実行理由を永続化する):

- cheap / reference 特徴からの自動候補
- 高残差
- 低リファレンス類似度
- 異常なタイミング
- 手動の時間範囲選択
- 明示的な always-run 設定

必須の解析ファミリ:

### 精密伝達 / カットオフ

- より高分解能の H(f)
- 急峻なカットオフ推定
- 狭帯域ノッチ / ピークの特性化
- カットオフ候補とインジェスト辞書の照合

### コム / エコー / 反射

- 精密スペクトルの自己相関
- ケプストラムピーク解析
- 推定コム遅延
- 二次エコー遅延 / レベル
- インパルス応答候補推定
- 初期 / 後期エネルギーと減衰記述子

### サンプルレート変換

候補ソースレート / 変換経路について:

- エイリアス / 折り返し相関
- スペクトルミラー / イメージ相関
- ナイキスト領域の構造
- 候補 fs スコア

### 非線形挙動

- 高調波残差
- 高調波エネルギー比
- レベル依存伝達
- 正負の非対称性
- クリッピング / リミッティング形状
- 実用的な範囲での相互変調証拠

### ダイナミクスモデル精緻化

- コンプレッサ / リミッタ伝達曲線候補
- アタック / リリース挙動
- ポンピング / ダッキング変調記述子

### タイミング / 反復 / ドロップ

- 高分解能の局所 time map
- 広い自己相似 / 反復ブロック探索
- ドロップアウト / ギャップの精緻化
- クリック / スプライスの位置特定
- スタック / 反復バッファ候補

### コーデックアーティファクト解析

- プリエコー候補
- ブロック境界周期性
- バーディ / スペクトルホール候補
- 過渡のにじみ

### ノイズ分解

- 定常 / トーン / 変調残差ノイズ
- 精緻化されたハム / 高調波系列

### 高度なチャンネル解析

- サポートが許す場合の周波数依存混合行列
- 詳細な位相 / 極性ルーティング証拠

## Why Now

全領域への常時高コスト解析は計算量的に非現実だが、候補駆動だけでは
未知の失敗モードを取りこぼす。トリガの記録と手動 / always-run の
両立が必要。

## Users and Value

- 解析者: なぜその区間に高コスト解析が走ったかを追跡できる
- 上級ユーザー: 任意区間に手動で expensive を実行できる

## Scope

### In Scope

- 上記トリガモードと解析ファミリ
- トリガ理由の永続化

### Out of Scope

- 結果から根本原因を断定する分類器

## Requirements

- 各 expensive 結果がトリガを記録する
- expensive 解析は任意の選択区間に手動実行できる
- expensive 検出器が、派生元の低レベル計測を上書き・隠蔽しない

## Acceptance Criteria

- [ ] 各 expensive 結果がトリガを記録している
- [ ] expensive 解析が任意の選択区間に手動実行できる
- [ ] expensive 検出器が派生元の低レベル計測を上書き / 隠蔽しない

## Deferred Design Concerns

### 独立記述子の保留と実装の受け皿 (#30)

以下は [expensive-family-gaps discovery](../discovery/expensive-family-gaps.md)
で明示した defer を維持する。#30 は未実装の追跡を補い、要求を実装済みに
変更しない。2026-10-11 に coordinator が defer 継続を承認した。

| 未実装項目 | 保留理由と既存観測 | 再検討条件 | 実装の受け皿 |
| --- | --- | --- | --- |
| 精密スペクトル自己相関の独立指標化 | cepstrum の `exp.echo_delay_s` がコム遅延を担う。独立指標の意味が曖昧 | cepstrum と区別する計測定義・単位・支持条件と既知コム fixture の期待値を承認する | al-analysis expensive の codec family (`codec.rs`)。新しい `exp.*` 指標として既存 echo 観測を保持する |
| エイリアス/折り返しの独立相関記述子 | `exp.src_image_score` が折り返し強度を担う。独立相関の定義は未確定 | image score との差、対応する周波数ペア・正規化・候補 fs の範囲と SRC fixture を承認する | al-analysis expensive の SRC family (`timing.rs`)。image 観測を置換しない独立指標 |
| ナイキスト領域の構造 | 新しい特徴の観測記述が未確定 | 対象 fs、領域、構造の数値表現・有効性と既知スペクトル fixture を承認する | al-analysis expensive の SRC family (`timing.rs`)。領域・resolution を明示する追加観測 |
| ドロップアウト/ギャップ形状の精緻化 | repeat block/gap が既存範囲を担うが、形状記述は未定義 | 長さ・境界・回復形状の定義、無音と欠落の区別、既知 dropout fixture を承認する | al-analysis expensive の timing family (`timing.rs`)。既存 repeat/gap と併存する追加記述子 |

この表を follow-up の backlog とする。再検討条件が成立した項目ごとに
別 Issue と実装 task を作成し、本 spec の計測契約・feature 辞書・test-spec
を承認してから着手する。予算と trigger 理由、未支持時の validity、下位
測定を上書きしない契約を維持する。現時点で四項目の実装 task は runnable
ではない。対応結果は [fixture-review-fidelity task](../tasks/fixture-review-fidelity.md)
から追跡する。

- 各ファミリの実行順とコスト予算 | fixture / 実測での計算時間計測が
  前提 | `validation-strategy.md` の fixture 実装時 | 実行時間の
  爆発リスク

## Sources and Relations

`candidate-event-model.md` の候補 / イベントと
`reference-analysis.md` の観測をトリガとして利用する。
