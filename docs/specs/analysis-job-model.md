---
id: "SPEC-034yYlrZw66KVKNEHErIn4"
type: "spec"
status: "proposed"
title: "分析ジョブモデルとオーケストレーション"
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
  blocks:
    - "media-ingest.md"
    - "analysis-dataset.md"
  supersedes: []
  superseded-by: []
  related:
    - "../adr/immutable-resumable-job.md"
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

# 分析ジョブモデルとオーケストレーション

## What Should Be Built

ingest・locate・3 ステージ解析・データセット保存・可搬エクスポートを
束ねる実行モデル。再現可能かつ再開可能(resumable)であることが必須。

- 1 回の解析実行を、安定した id を持つ不変のジョブとしてモデル化する
- 対象 1 つ + 0 個以上のリファレンス / 素材を受理する
- リファレンスは「正常動画」等のハードコード分岐ではなく、ロール
  メタデータを持つ一般化入力とする。推奨ロール:
  `capture_bug`、`capture_reference`、`source_asset`、ユーザー定義の
  拡張ロール
- ペアワイズ解析を許可する(例: ソース素材 ↔ バグキャプチャ、
  ソース素材 ↔ 正常キャプチャ、正常キャプチャ ↔ バグキャプチャ)
- 入力・デコード済みストリーム・locate 結果・アライン区間・特徴結果に
  それぞれ明示的な id を持たせる
- 設定はシリアライズ可能かつバージョン管理される
- パラメータと閾値は各結果とともに永続化する
- ステージ選択をサポートする:
  - cheap のみ
  - cheap + reference
  - 自動 expensive
  - 手動 expensive 区間
  - always-run expensive
- 解析の再実行なしに永続化済み結果を再参照できる(GUI 範囲読み・
  export は dataset 成果物のみを消費する)
- 部分失敗後にパイプラインを再開できる
- 基盤アルゴリズムが許す範囲で決定的実行

CLI / API 形状は進化しうるが、以下と同等の操作をアーキテクチャが
サポートすること:

```text
audiolens analyze <job-config>
audiolens locate <job-config>
audiolens analyze --stage cheap
audiolens analyze --stage expensive --range 12.0:18.0
audiolens visualize <existing-analysis>
```

## Why Now

ジョブモデルは全ステージが共有する実行・再現性の基盤であり、
データセットスキーマと並んで最初に固める必要がある。後付けでは
id 体系と来歴追跡が破綻する。

## Users and Value

- 解析実行者: 失敗時の再開・部分再実行・設定の再実行比較が可能
- データセットのコンシューマ: すべての出力の生成系譜を
  マニフェストから再構成できる

## Scope

### In Scope

- ジョブ / ステージ / 成果物の id 体系とライフサイクル
- 入力ロールモデルとペアワイズ解析の構成
- 設定のシリアライズ・バージョニング・パラメータ永続化
- ステージ選択、キャッシュ再利用、再開、決定的実行の方針
- 上記操作を表現できる CLI / API 形状

### Out of Scope

- 各ステージ内部のアルゴリズム詳細(各領域 spec を参照)
- ジョブスケジューラや分散実行基盤

## Requirements

- ジョブは不変で、再実行は新しいジョブ id を持つ
- 設定・パラメータ・閾値は結果と不可分に記録される
- 失敗したステージが、完了済みの下流 / 上流成果物を無効化しない
- 同一の保存済みジョブ設定を異なるアナライザバージョンで再実行して
  比較できる

## Acceptance Criteria

- [ ] ラン manifest から、すべての出力を生成した入力・アルゴリズム・
  バージョン・パラメータを再構成できる
- [ ] 表示・検査・export にデコードや音声解析の再実行が不要
- [ ] expensive ステージの失敗が完了済みの cheap / reference 出力を
  無効化しない
- [ ] 同一の保存済みジョブ設定を再実行し、アナライザバージョン間で
  結果を比較できる

## Deferred Design Concerns

- 具体的な CLI 文法・設定ファイル形式 | 実装技術の選定に依存 |
  開発環境決定時 | なし(抽象操作のみ規定)

## Sources and Relations

全体仕様 `audiolens-overview.md` を領域別に细化した spec。
実行モデルの不変性・再開性に関する決定は ADR
`../adr/immutable-resumable-job.md` を参照。
