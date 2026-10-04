---
id: "DESIGN-034AaszqnyPGmacvhSzO83"
type: "design"
status: "approved"
title: "Expensive 追加解析ファミリ設計"
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
  depends-on:
    - "analysis-stages.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/expensive-analysis.md"
    - "../discovery/expensive-family-gaps.md"
    - "analysis-stages.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Expensive 追加解析ファミリ設計

## Context

spec `../specs/expensive-analysis.md` の要求ファミリのうち、
`expensive.rs` に未実装のものを追加する。既存の
「トリガ理由の永続化」「下位測定を上書きしない」
「非 Valid トリガは invalid 観測を記録」の契約は維持する。
`analysis-stages.md` のステージ/イベント設計を前提とする。

## Scope

### In Scope

- impulse-response 候補推定と early/late energy・decay 記述子
- click/splice 定位と stuck/repeated buffer 候補の深掘り
- pre-echo 候補と transient smear
- compressor/limiter transfer-curve 候補と pumping/ducking 変調記述子
- level-dependent transfer(入力レベル→伝達の依存性)
- intermodulation 証拠(強トーン周辺の実用的な範囲)
- ノイズ分解の stationary / tonal / modulated 分類
- 周波数依存の混合行列(サポート可能な帯域で)

### Out of Scope

- 新しいトリガ種別の追加(既存 trigger 機構のまま)
- 下位(cheap/reference)特徴の再設計
- ゲーム固有のバグ分類

## Components and Boundaries

- `al-analysis::expensive` を `expensive.rs` 単体から
  `expensive/` サブモジュール群に分割する。ルート側は
  config・trigger・`plan_regions`・`analyze_region` の
  オーケストレーションを保持し、各ファミリは
  `expensive/{ir,codec,dynamics,noise,channel,timing}.rs` に
  置く。既存ファミリの振る舞いは変えない
- 各ファミリは `analyze_region` と同じ push 規約
  (特徴 id・単位・confidence・validity・support・resolution
  メタデータ)でレコードを emit する
- 境界: 入力は region の PCM/STFT/既存スペクトル中間物。
  dataset 永続化形状は既存と同じ

## Data and Control Flow

- 既存どおり trigger → region → `analyze_region` → family 計算
  → FeatureRecord 永続化
- IR 系: リージョンの H(f) または deconvolution で IR 候補を得て、
  early(主ピーク近傍)/late エネルギー比・decay 傾きを記録
- codec 系: オンセット前後の高域エネルギー分布から pre-echo /
  smear を観測
- dynamics 系: 短時間ゲイン軌跡の入出力散布と低周波変調から
  transfer-curve / pumping を観測
- noise 系: バンドエネルギー系列の時間変動・トーン性・AM を
  stationary/tonal/modulated に分類
- channel 系: 帯域ごとの最小二乗混合係数を推定
- 非 Valid トリガ選択時は従来どおり推定せず invalid 観測を記録

## Non-Functional Constraints

- すべて純粋 DSP・外部依存なし・決定的
- 各レコードはトリガの信頼度を超えない
- 下位測定値は変更・削除しない

## Risks and Trade-offs

- deconvolution / 帯域混合推定は数値的に不安定になりうる
  → 正規化・条件数の記録・support 不足時の invalid 化で防御
- ファミリ追加で `expensive.rs` が大きくなりすぎる
  → サブモジュール分割で吸収する

## Deferred Design Concerns

- IR 推定の窓長・正規化パラメータ | fixture 精度と相談 |
  fixture 実装時 | 過大/過小な decay 推定
- intermodulation の対象トーン選択 | 実材料依存 | 実運用時 |
  過剰な候補出力

## Verification Notes

- [ ] 各ファミリの既知合成ケースで期待値が記録される
- [ ] 非 Valid トリガの領域では invalid 観測になる
- [ ] 既存ファミリの出力が変わらない
