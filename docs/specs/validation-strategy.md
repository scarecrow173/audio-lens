---
id: "SPEC-034yYlGk2ZJyVWRZif4yP3"
type: "spec"
status: "proposed"
title: "検証戦略とゴールデンFixture"
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
  depends-on: []
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

# 検証戦略とゴールデンFixture

## What Should Be Built

DSP 変更を計測可能にし、既に遭遇した失敗モードへの回帰を防ぐ
検証戦略。

必須の合成 / ゴールデンケース:

### Locate

- 厳密な挿入
- ゲイン変更された挿入
- EQ フィルタ済み挿入
- 無音境界
- 偽相関ピークを持つ強周期信号
- ループ素材
- 複数の正当な出現
- 一定オフセットの長尺ソース
- 線形ドリフトの長尺ソース
- スキップ + 再開
- マルチチャンネルダウンミックス
- 静かな素材を覆い隠す支配的 BGM
- BGM 減算後の残差探索

### Reference analysis

- 既知のスカラーゲイン
- 既知の帯域別 EQ
- 既知の位相 / オールパス変化
- 固定遅延
- ジッタ付き遅延
- サンプルレート不一致 / 時間スケールドリフト
- 極性反転
- チャンネルスワップ / 行列ミックス
- 低コヒーレンス付加ノイズ
- クリッピング / リミッティング
- 既知遅延のコムフィルタ
- 既知遅延のエコー
- 意図的な SRC エイリアス / イメージアーティファクト
- ドロップブロック
- 反復オーディオバッファ
- スタックサンプル

### Video / A-V

- CFR / VFR タイムスタンプケース
- 既知の A/V オフセット
- A/V ドリフト
- 重複 / ドロップフレームメタデータケース

回帰アサーション(「特徴が存在する」以上を検証する):

- 既知の合成パラメータに対する数値許容差
- 期待される validity / 確度の挙動
- 曖昧な locate ケースでの偽受理の不存在
- 低相関 / 低コヒーレンス結果の適切なマーキング
- 保持される中間テンソルの次元 / メタデータ
- 適切な場合の決定的出力ハッシュ

固定すべき過去の失敗モード:

- 無音境界や周期信号での偽相関ピーク
- 非コヒーレントな PSD 比による高域の持ち上げ
- 低相関帯域での最適ラグ探索による値の水増し

## Why Now

アルゴリズム変更の影響を事前に可視化しないと、解析品質の回帰が
静かに混入する。失敗モードを fixture として固定することが、
変更の安全性の基盤になる。

## Users and Value

- 開発者: 変更が既知の失敗モードを再発させないかを確認できる
- レビュアー: 数値・validity・確度の期待値を客観的に照合できる

## Scope

### In Scope

- 上記のゴールデン / 合成ケースと回帰アサーション
- 既知失敗モードの固定

### Out of Scope

- 実環境キャプチャの大規模ベンチマーク運用

## Requirements

- すべてのコアアルゴリズムに少なくとも 1 つの正常系と 1 つの
  敵対的 fixture がある
- 閾値 / アルゴリズム変更を保存済みの期待メトリクスで評価できる

## Acceptance Criteria

- [ ] すべてのコアアルゴリズムに正常系・敵対的 fixture が各 1 つ以上
  存在する
- [ ] 閾値 / アルゴリズム変更を保存済み期待メトリクスで評価できる

## Deferred Design Concerns

- fixture の生成・保存形式と CI 組み込み | 実装技術の選定に依存 |
  開発環境決定時 | fixture なしでの実装開始による回帰リスク

## Sources and Relations

全体仕様に対する横断的な検証方針。各領域 spec の受け入れ基準を
数値的に裏付ける。
