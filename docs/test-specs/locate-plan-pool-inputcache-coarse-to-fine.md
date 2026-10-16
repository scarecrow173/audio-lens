---
id: "TSPEC-034GMJZTDO67z7TO6TNSzl"
type: "test-spec"
status: "draft"
title: "locate plan pool・InputCache 監査・coarse-to-fine 境界"
created: "2026-10-16"
updated: "2026-10-16"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/locate-plan-cache.md"
    - "docs/designs/locate-plan.md"
    - "docs/adr/locate-plan-run-pool.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# locate plan pool・InputCache 監査・coarse-to-fine 境界

## Purpose

#40 の残件実装が (a) 検出結果を変えずに相関 plan/workspace を
再利用すること、(b) `InputCache` の設計上の再利用が実測で
成立すること、(c) coarse-to-fine の座標変換が境界でも正しい
ことを固定する。pool 導入の価値は「結果不変・構築回数削減」
であり、本 spec がその回帰境界を定義する。

## Feature

locate メソッド経路の相関実行を、ワーカー所有の形状キー pool
経由に切り替える。観測可能な出力(MethodEvidence、
CoarseSearch、Anchor、合議結果)はすべて不変。

## Rules

- Rule P1: 同一 `(template_len, signal_len)` の連続呼出で
  `CorrelationPlan`/`RealFftPlan` の新規構築は 1 回のみ
- Rule P2: pool 経由の `ncc`/`gcc_phat` 出力はフリー関数
  `dsp::ncc`/`dsp::gcc_phat` と全要素一致
- Rule P3: pool のエントリ数は `MAX_SHAPES` を超えない
  (超過時は全 evict し再構築する)
- Rule C1: `InputCache::prepare` 系の呼出に対し、同一
  (id, channel, rate, version, filter) の 2 回目以降は hit で
  フィルタ再計算しない
- Rule C2: `release(id, version)` 後、当該入力のエントリは
  `len()` から消える(Arc の外部生存参照がなければメモリ解放)
- Rule F1: coarse 位置→原レート座標換算は `(pos * rate +
  coarse_rate/2) / coarse_rate` の丸めで、真値から
  ±`coarse_rate` 以内に収まる
- Rule F2: `neighborhood_radius` は `2 * resampler_radius +
  ceil(rate/coarse_rate)` で、間引き丸め誤差をカバーする
- Rule F3: coarse 候補が `search_range` の radius 以内に位置する
  場合 `fallback_reason` は `"boundary-candidate"` を返す

## Examples

- Example P1: pool に対し同じ長さの (template, signal) で ncc を
  2 回呼ぶ → plan 構築は 1 回、2 回目の出力は 1 回目と一致
- Example P2: 異なる長さの呼出を `MAX_SHAPES + 1` 種類行う →
  pool の生存エントリは `MAX_SHAPES` 以下
- Example C1: 1 capture × 複数 source の prepare シーケンスで
  `stats()` が `hits >= 1` を記録し、miss はユニーク入力数に一致
- Example C2: `release` 呼出後 `len()` が対象分だけ減る
- Example F1: 既知 offset で間引き信号を用意し coarse NCC ピーク
  を原レート換算した位置が真値 ±coarse_rate に収まる
- Example F3: 真のオフセットが 0 または `signal.len() -
  template.len()` の端にある fixture で `boundary-candidate` が
  記録される

## Guarantees

- 全既存 locate テスト・golden・合議テストが不変で pass する
  (数値経路は `Correlation::run` に集約され plan は値に影響しない)
- `CorrelationWorkspace` の借用契約(曲線保持中の再使用禁止)は
  pool API の借用セマンティクスが静的に強制する
- pool はワーカー内に閉じ、`Send`/`Sync` を新たに要求しない

## Non-goals

- 性能改善量の閾値テスト(計測は IR に記録、CI で時間を強制しない)
- `best_window_match` の shape 多様性に対する hit 率保証
- `dsp::ncc` フリー関数自体の挙動変更

## Risk

- pool 化で借用順序が破られると compile error になる設計のため、
  実行時リスクは eviction thrash と workspace メモリ同時保持のみ。
  Rule P3 と設計の bound 規約がこれをカバーする
