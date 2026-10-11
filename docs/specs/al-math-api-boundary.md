---
id: "SPEC-034DXcFaOvk0LFMoPMjkaS"
type: "spec"
status: "approved"
title: "al-math 公開 API のバックエンド中立化と split re/im FFT 境界"
created: "2026-10-11"
updated: "2026-10-11"
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
    - "../adr/al-math-api-fft-split-re-im.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/al-math-api-fft.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers:
    - "../discovery/al-math-simd-reduction.md"
  deferred-by: []
---

# al-math 公開 API のバックエンド中立化と split re/im FFT 境界

## What Should Be Built

`al-math` の公開 API をバックエンド中立の型だけで構成し、pulp 固有の
型・トレイトを公開面から除去する。あわせて FFT の複素数境界を
split re/im 表現(`&mut [f64]` ペア)に変更し、`dsp::fft` ラッパが
呼出しごとに行う `Vec<c64>` 確保と往復コピーを除去する。

## Why Now

将来 pulp 以外のバックエンドへ移行する可能性を残す方針が決まった。
現状は `lib.rs` の再公開と `Fft`/`RealFft` の `c64` シグネチャにより
バックエンド型が外部へ露出しており、移行コストが利用側へ流出する。
また STFT がフレームごとに `dsp::fft` を通る経路で pack/unpack が
残っており、表現変更が副次的な性能改善になる。pre-release のため
互換層なしで素直に API を変更できる。

## Users and Value

- `al-analysis` / `al-locate` の実装者: `al_math` の利用側が
  バックエンド型を知る必要がなくなり、`StftMatrix` の re/im 配置と
  一致する API をそのまま呼べる
- 将来のバックエンド置き換え担当者: 変更が `al-math` 内部に閉じ、
  下流クレートの連鎖修正が不要になる

## Scope

### In Scope

- `al-math` 公開面からの pulp 型・トレイト(`Simd`, `WithSimd`,
  `c64`, `F64s`, `C64s`, `dispatch`)の除去または `pub(crate)` 化
- `Fft`/`RealFft` の公開シグネチャの split re/im 化と、内部カーネル
  の SoA 複素演算への書き換え
- 公開シグネチャ上のカーネル呼出し点に dispatch 集約を維持
- `dsp::fft` / `bench_dsp.rs` 等、FFT 境界の直接利用者の追従
- scalar 強制経路と dispatch 経路の一致検証モデルの維持

### Out of Scope

- pulp の置き換え自体(引き続き内部依存として利用)
- 固定幅ベクトル型(`f32x4`/`f32x8` 相当)の導入 — 調査の結果不要と
  判断済み
- 汎用ベクトル/行列プリミティブ層の公開 API 拡張 — 本 spec では公開面の
  型中立化のみを対象とし、プリミティブ追加は別 spec で扱う
- `dsp::fft` ラッパ自体の設計見直し(その後の STFT/RealFft 化は別設計)
- AVX-512、GPU、ベクトル超越関数(既存設計の deferred を継承)

## Requirements

- `al-math` の公開 API(`pub`/`pub use` のシグネチャ)に `pulp`
  由来の型・トレイトが出現しないこと
- `al-math` 以外のワークスペースクレートが `pulp` に依存しないこと
- FFT 境界は split re/im(`&mut [f64]` ペア)とし、AoS 複素バッファと
  SoA の二重表現を公開面に持ち込まないこと
- `dispatch` の呼出しは演算の公開入口で1回とし、カーネルは
  `Simd` ジェネリックな単一ソースを維持すること
- scalar 強制経路(`pulp::Scalar`)と dispatch 経路が同一入力で
  一致し続けること
- 既存の分析意味・golden 出力・結果形式を変更しないこと

## Acceptance Criteria

- [ ] `pulp`・`al_math::c64`・`al_math::Simd`・`al_math::WithSimd`・
  `al_math::dispatch` への参照が `al-math` 内部に限定される
- [ ] `al-math` の公開関数シグネチャにバックエンド固有型がない
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が合格する
- [ ] FFT の forward/inverse round trip、DC/Nyquist が scalar/dispatch
  両経路で一致し、非2冪・ゼロ長は panic(既存
  `rejects_empty_transform` 等を維持)、n=1 等の短尺・任意べき乗長で
  一致する
- [ ] 複素 FFT/RealFft の出力が独立参照実装(radix-2 reference /
  naive DFT)と直接比較で一致する既存テストが split API でも
  保持される
- [ ] `bench_dsp` で実使用経路(`dsp::fft` 経由の FFT、ncc、
  bandpass、gcc_phat、welch psd、stft-equivalent)が旧版と同等
  以上の性能である。直接 plan 利用の FFT マイクロベンチ
  (complex fft/rfft)は SoA 構造要因による ~10% 劣後を既知
  トレードオフとして記録する

## Deferred Design Concerns

- ベクトル演算プリミティブ(dot/sum_sq/scale/axpy 等)の公開 — 各
  分析 crate に重複する `corr0`/`rms`/`energy` 系ヘルパの集約を
  次の spec で扱う。その際に reduction 順序ポリシー(lane 単位
  reduce の許容)を決定する必要がある
- `StftMatrix` を `RealFft` 直結にする設計 — 本変更の split re/im
  境界がその前提となる
- 直接 FIR 畳み込みの FFT 畳み込み化・lag 走査の `ncc` 置換 —
  アルゴリズム置き換えであり別 spec
- 直接 plan 利用の FFT マイクロベンチに残る ~10% 回帰の追跡 —
  SoA 固有のメモリ uops 増が構造的要因であり、将来 AoS
  ハイブリッド等で回収する余地を残す

## Sources and Relations

- 調査: `docs/discovery/al-math-api-fft.md`(SIMD 残一覧、pulp 露出
  監査、複素表現の代替案比較)
- 決定: `docs/adr/al-math-api-fft-split-re-im.md`(split re/im 採用、
  公開面のバックエンド中立化)
- 上位決定: `docs/adr/simd-arithmetic-pulp.md`(pulp は内部依存と
  して継続)
- 設計方針: `docs/designs/simd-dsp-foundation.md`
