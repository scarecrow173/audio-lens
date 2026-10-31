---
id: "TSPEC-034z99BMV8hBTVwYrwacBo"
type: "test-spec"
status: "superseded"
title: "可視化・エクスポート検証"
created: "2026-10-02"
updated: "2026-10-31"
owners: []
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
  superseded-by:
    - "../impl/ir/issue-106-viz-removal.md"
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/visualization-dataset.md"
    - "docs/specs/analysis-export.md"
  derived-by: []
  verifies:
    - "docs/specs/analysis-export.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

> **Superseded (Issue #106)**: 派生可視化・静的レポートの検証項目は
> 廃止対象とともに消えた。可搬 export の検証(可搬コピー・相対パス
> 化・manifest/hash・安全性)は `crates/al-dataset/tests/export.rs`
> へ引き継がれた。記録は
> [`../impl/ir/issue-106-viz-removal.md`](../impl/ir/issue-106-viz-removal.md)
> を参照。以下は歴史的記録として残す。

# 可視化・エクスポート検証

## Purpose

Analysis Dataset からの派生可視化データ生成・人間可読サマリ・
可搬パッケージの契約を固定する。派生層が新しい診断を発明せず、
決定的かつ再配置可能であることを保証する。

## Feature

Analysis Dataset からの派生可視化と成果物パッケージ

## Rules

- `visualization/` プロダクトは保存済み Analysis Dataset のみ
  から再生成できる(メディアデコード・特徴抽出なし)
- 全可視化系列は `visualization/timeline.json` の共通
  タイムベースに登録される(window/hop/オフセットを宣言)
- 波形 LOD は min/max/RMS の複数レベルを持ち、範囲読みで構築
  される(全 PCM をメモリに載せない)
- スペクトログラムは対象・アライン済みリファレンス・dB 差分・
  残差・コヒーレンスのタイルと validity マスクを持つ
- 帯域ヒートマップ・伝達ビュー・time-map ビュー・ステレオ
  ビュー・残差ビュー(波形/STFT/PSD/帯域エネルギー)・
  イベントマーカーが共通タイムラインにアラインされる
- 必須ロールが `not_produced` のプロダクトは
  `skipped_products` に理由付きで記録される
- confidence / validity / 元計測参照は派生データに保持される
- サマリは決定的で、根本原因を推定しない
- パッケージは相対パスのみ、入力同一性は sha256 で検証でき、
  元メディアが無くても数値/テンソルが読める
- export ツリー内に絶対パスを含む文字列は存在しない

## Examples

- Given populated dataset(features/events/residual/locate あり)、
  When visualization を実行、Then LOD/タイル/ビュー/マーカーが
  `visualization/` に生成される
- Given 同一 dataset、When viz を 2 回実行、Then ファイル内容が
  バイト一致する(job manifest の新規 id を除く)
- Given features 未実行の dataset、When visualization を実行、
  Then 対応プロダクトは `skipped_products` に記録され、
  ステージは失敗しない
- Given `al run --stages ingest,locate,dataset,visualization,export`、
  Then 完走し `export/` にパッケージが生成される
- Given 移設したパッケージ、When 別のパスで開く、Then 数値・
  テンソル・残差音声が読め、入力ハッシュが検証できる
- Given viz ステージの中途失敗、When resume する、Then 部分
  出力が置換され完走する

## Guarantees

- 派生層は診断語彙を追加しない(measurement-over-verdict)
- 長尺キャプチャでも全ズームレベルが生 PCM 非参照で描ける
- パッケージは移設可能で、入力同一性は hash で検証できる
- schema_version 不一致は明示的失敗になる

## Non-goals

- UI / ラスタ画像の生成
- コンシューマ側の実装

## Risk

- 決定性が崩れると fixture / golden 検証が機能しない
- 絶対パスの混入は可搬性を破壊する
- skipped の記録がないと「未実行」と「欠落」が区別できない
