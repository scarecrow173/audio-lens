---
id: "DESIGN-034AasB7gEcddWOpUHrKX8"
type: "design"
status: "approved"
title: "ゴールデン fixture マトリクス設計"
created: "2026-10-04"
updated: "2026-10-04"
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
  superseded-by: []
  related:
    - "mp4-ingest.md"
    - "video-features.md"
    - "expensive-families.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/validation-strategy.md"
    - "../discovery/validation-fixture-gaps.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# ゴールデン fixture マトリクス設計

## Context

spec `../specs/validation-strategy.md` は locate / reference /
video-A/V の既知ケース行列と「既知パラメータに対する数値許容の
回帰アサーション」を要求する。現状テストは各クレートに点在し、
要求行列を網羅していない。fixture 生成と期待メトリクス検証を
横断で共有できる基盤を設計する。

## Scope

### In Scope

- 共有の合成信号/チャンネル操作ビルダ(ゲイン・EQ・遅延・
  jitter・極性・ch swap/mix・SRC・クリップ・drop/repeat/
  stuck ブロック・ノイズ)
- コンテナ fixture の再利用可能化。**現行 `write_avi`/`AviSpec`
  は CFR かつ `dwStart=0` 固定・movi パケット列を制御不能で、
  spec が要求する VFR・dwStart≠0 オフセット・0 長 dup チャンク・
  drop ギャップを生成できない**。testkit 側にパラメタ化 AVI
  writer(movi パケット列・per-stream dwStart・チャンクサイズを
  指定可能)と MP4 writer(stts/ctts/stss 等を指定可能)を提供
  する。`write_avi`/`AviSpec` は本番 API かつ `al-cli` の
  `gen_samples` 例が使うが、fixture 知識を本番コードから外す
  方針に従い `al-testkit` へ移し、`al-cli` は dev-dependency で
  参照する(既存呼び出しは re-export/移動で対応)
- ケース名・既知パラメータ・期待メトリクス・許容誤差を
  対にした回帰検証パターン
- spec が列挙する locate/reference/video-A/V ケースの網羅
- spec `validation-strategy.md` の「固定すべき過去の失敗モード」
  3 件を名前付きケースとして明示する:
  1. 無音/強周期信号での偽相関ピーク受理
  2. 非コヒーレントな PSD 比による高域リフト偽装
  3. 低コヒーレンス時の optimal-lag 過大推定

### Out of Scope

- 期待出力のゴールデンファイル永続化(本設計では
  コード内の期待値+許容で検証する)
- 新規解析機能そのもの(各 WS の fixture は各 WS が追加)

## Components and Boundaries

- `al-testkit`(dev-dependency のみの補助クレート): 信号生成・
  変換(ゲイン/EQ/遅延/極性/mix/SRC/クリップ/drop/repeat/
  stuck)・コンテナ/メディア fixture・期待値検証ヘルパ。
  本番クレートからは参照しない。`write_avi`/`AviSpec` は
  `al-media` から本クレートへ移し、`al-cli` の `gen_samples`
  例は dev-dependency で参照する。MP4 writer は WS-C
  (`mp4-ingest`)が所有する合成 writer を共有/移管する形で
  集約する(兄弟 design 参照: `related`)
- 各クレートの tests は `al-testkit` で fixture を生成し、
  `expect` 系ヘルパで既知メトリクスと許容誤差を対にして検証
- 境界: fixture/期待値はテスト領域にのみ存在し、本番コードに
  fixture 知識を持ち込まない。なお各クレートに点在する既存の
  `noise`/`wav_bytes` ヘルパ重複は本 WS では移行せず、
  既存テストは維持する(plan どおり)。新規 fixture は
  testkit に集約する

## Data and Control Flow

- testkit の builder が既知パラメータで信号/コンテナを生成
- 解析を実行 → 期待メトリクス(回復オフセット・ゲイン・
  遅延・トーン位置・validity 等)を許容誤差付きで照合
- ケース行列は spec のリストに対応づけられる命名で整理する

## Non-Functional Constraints

- fixture はすべてプログラム生成で決定的(シード固定)
- 期待値はテストコード内の数値(浮動小数は許容誤差必須)
- 外部アセットに依存しない

## Risks and Trade-offs

- 浮動小数の安定性 → 許容誤差を必須にし、値ではなく
  「回復されるべき関係」をアサーション
- fixture builder の重複実装 → `al-testkit` に集約

## Deferred Design Concerns

- ゴールデンファイル出力への移行 | 期待値の管理コスト |
  期待値が増えて管理しきれなくなった時 | レビュー差分が見にくい

## Verification Notes

- [ ] spec 列挙ケースがテストとして存在する
- [ ] 既知パラメータが許容誤差内で回復される
- [ ] 偽受理/低相関が期待どおり invalid/low confidence になる
