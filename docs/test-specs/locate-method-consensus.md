---
id: "TSPEC-034z8k5ftvcMaUeWq1L4D4"
type: "test-spec"
status: "proposed"
title: "locate-method-consensus"
created: "2026-10-02"
updated: "2026-10-02"
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
  derives-from:
    - "docs/designs/localization.md"
  derived-by: []
  verifies:
    - "docs/specs/source-localization.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# locate-method-consensus

## Purpose

4 系統の独立定位手法と合議受理の契約を固定する。locate 実装の
根幹(偽ピーク排除・証拠保存)が回帰しないことを保証する。

## Feature

複数独立手法によるソース定位と合議受理

## Rules

- NCC・band-NCC・GCC-PHAT・スペクトル包絡の 4 手法は互いに独立に
  証拠(候補位置・スコア・ピーク対背景比・ピーク幅・第1/第2ピーク比・
  validity・棄却理由)を生成する
- 受理には 2 手法以上の位置一致(許容差は設定可能)とピーク/背景
  閾値が必要。単一手法の強いピークだけでは受理しない
- 受理/棄却どちらの場合も手法ごとの証拠と棄却理由が成果物に残る
- 無音・低情報区間はエラーではなく明示的な validity マークになる
- 受理結果はソース素材 id・ソース区間・キャプチャ区間・手法一致数・
  位置のばらつき・確度・アルゴリズム/バージョン/パラメータ・
  alignment id を持つ
- ソースとキャプチャのペアリングは input role で決まる
  (source_asset × capture_*)。サンプルレート不一致ペアは推定せず
  明示マークする

## Examples

- Given 既知オフセットでソースを埋め込んだ合成キャプチャ、
  When locate を実行、Then 真の位置が 2 手法以上で受理される
- Given 1 手法だけが強いピークを返す合成信号、When locate を実行、
  Then 受理されず棄却理由が記録される
- Given 無音区間だけのキャプチャ、When locate を実行、Then
  失敗ではなく silence/insufficient 系の validity が記録される
- Given 44.1kHz ソースと 48kHz キャプチャ、When locate を実行、
  Then そのペアは明示的に unmeasurable としてマークされる

## Guarantees

- 偽相関が受理をすり抜けない(≥2 独立手法の一致が必須)
- 全手法の証拠が永続化され、後から棄却理由を検査できる
- alignment id が発行され後段 provenance から参照可能

## Non-goals

- 定位精度の DSP 的限界評価
- 実ゲームキャプチャでの閾値調整(合成 fixture 基準)

## Risk

- 合議を弱めると偽定位が downstream 全解析を汚染する
- 証拠未保存だと棄却理由の検証が不可能になる
