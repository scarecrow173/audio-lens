---
id: "DISC-034AajDKh03yeDOK42xdhe"
type: "discovery"
status: "resolved"
title: "検証 fixture マトリクスのギャップ"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "../specs/validation-strategy.md"
    - "../../crates/al-cli/examples/gen_samples.rs"

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
    - "../designs/golden-validation.md"
    - "../plans/golden-validation.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 検証 fixture マトリクスのギャップ

## Exploration Goal

issue #13(検証戦略・ゴールデン fixture)は幅広い合成/ゴールデン
ケースと数値許容の回帰アサーションを要求する。現状テストとの
差分を整理し、fixture マトリクスの構成を確定する。

## Key Issues and Assumptions

- 実装済み: 各クレートの単体/結合テスト(合成信号の既知パラメータ
  検証あり)、決定的 WAV フィクスチャ 7 件 + AVI フィクスチャ
  (samples/)、export の決定的ハッシュスナップショット
  (`al-viz/golden.rs`)
- 未整備(issue #13 要求):
  - locate 要求ケースの網羅(gain 変更/EQ フィルタ挿入、
    強周期信号の偽ピーク、skip+resume、multichannel downmix、
    BGM マスキング後の残差探索など)
  - reference 要求ケース(既知 EQ・位相/all-pass・固定/jitter
    遅延・極性・ch swap・低コヒーレンスノイズ・クリップ・
    comb/echo・SRC alias・drop/repeat/stuck バッファ)
  - Video/A-V ケース(CFR/VFR、既知 A/V offset・drift、
    dup/drop メタデータ)
  - 「既知パラメータに対する数値許容の期待メトリクス」を
    保存し閾値/アルゴリズム変更を評価する機構
- 前提: fixture はすべてプログラム生成で決定的(外部アセットに
  依存しない)

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| クレート別 fixture ビルダ + 期待メトリクス JSON による回帰テスト | 決定的・追加依存なし・CI で回帰検出できる | fixture マトリクスの整備コスト | yes |
| ゴールデンファイル(期待出力をファイル化) | 差分が見やすい | 浮動小数の安定性管理が必要・肥大化しやすい | maybe |
| 現状テストのまま | 変更不要 | issue #13 未達 | no |

## Tentative Conclusions and Hypotheses

- confirmed: 既存のテスト fixture ヘルパを拡張し、ケース名と
  既知パラメータ・期待メトリクス許容を対にしたマトリクスで
  issue の大半を満たせる
- hypothesis: 新規機能(映像特徴/MP4/expensive 追加ファミリ)の
  fixture は各ワークストリームの task で追加し、本 WS は
  既存機能のマトリクス + ハーネスを担うのが綺麗

## Open Questions

- 期待メトリクスの記録形式(コード埋め込み vs JSON) → design で決定

## Promotion Candidates

- [ ] spec-doc needed: 既存 `docs/specs/validation-strategy.md` が
  要求を記述済み。新規 spec 不要
- [ ] adr-doc needed: 不要

## Sources and Relations

- GitHub issue #13 (validation strategy / golden fixtures)
- `docs/specs/validation-strategy.md`
- `crates/al-cli/examples/gen_samples.rs`, `samples/`
- `crates/*/tests/`(既存 fixture パターン)
