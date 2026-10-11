---
id: "ADR-034DXb88VbwPFg3jZqRxkv"
type: "adr"
status: "accepted"
title: "al-math 公開 API からバックエンド型を排除し FFT 境界を split re/im 表現にする"
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
  depends-on:
    - "simd-arithmetic-pulp.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/al-math-api-fft.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# al-math 公開 API からバックエンド型を排除し FFT 境界を split re/im 表現にする

## Context and Problem Statement

[pulp 採用 ADR](simd-arithmetic-pulp.md) と
[SIMD 算術基盤設計](../designs/simd-dsp-foundation.md) により `al-math` の
カーネルは pulp dispatch で SIMD 化された。しかし公開 API には pulp 型が
露出している: `lib.rs` が `pulp::{Simd, WithSimd, c64}` を再公開し、
`F64s`/`C64s`/`dispatch` も公開、`Fft::transform` と `RealFft` 系の
シグネチャが `&mut [c64]` を取る。将来 pulp 以外のバックエンドへ移行する
可能性を残すため、公開面をバックエンド中立の型だけにする必要がある。
FFT 境界の複素数表現をどう定義するかが分岐点となる。

## Decision Drivers

* 公開 API は `f64` slice とクレート所有の結果表現のみとし、
  バックエンド固有型・トレイト境界を含めない(バックエンド移行を
  `al-math` 内部の変更に閉じ込める)
* 呼出し側の既存データ配置(`StftMatrix.re/.im`、永続テンソル
  `stft_re`/`stft_im`、`dsp::fft` の re/im シグネチャ)に余計な表現
  変換を挟まない。同一概念の表現は一箇所に集約する
* 現行 `dsp::fft` が呼出しごとに行う `Vec<c64>` 確保と re/im 往復
  コピー(STFT がフレームごとに通過)を除去する
* 単一ソース `WithSimd` カーネル + `pulp::Scalar` 強制経路との比較
  という検証モデルを維持する
* stable Rust / f64 共通表現を継続し、pre-release 方針として互換層を
  設けない

## Considered Options

* split re/im 配列(`&mut [f64]` ×2)を FFT 境界の複素数表現とする
* クレート所有の `Complex` 型(AoS、`#[repr(C)]` の re/im 構造体)を
  公開し、内部で `&mut [c64]` へゼロコストキャストする
* 不透明な `Spectrum` バッファ型を公開し、値へのアクセスを限定する
* 現状維持(pulp `c64` が公開シグネチャに残る)

## Decision Outcome

Chosen option: "split re/im 配列", because 呼出し側のデータ配置と一致し
pack/unpack を完全に消せる唯一の案であり、SoA の複素演算はレーン内
shuffle を要さず AoS と同等以上で、境界が純データのままバックエンド
中立を保てるため。

### Consequences

* Good, because `dsp::fft` の呼出しごとの `Vec<c64>` 確保と往復コピーが
  消え、STFT 等のフレーム単位呼出しで O(n×frames) の帯域を削減する
* Good, because `StftMatrix` と永続テンソルの re/im 分離表現と一致し、
  表現ドリフトを起こさない
* Good, because pulp 型の外部露出が消え、将来のバックエンド移行が
  `al-math` 内部に閉じる
* Bad, because `Fft`/`RealFft` カーネルの複素演算を実部演算へ分解する
  書き換え量が3案で最大であり、bit-reversal の要素交換が re/im の
  2ストアになる
* Bad, because `bench_dsp.rs` 等、FFT 境界を直接使う公開 API 消費者の
  追従が必要(`al-locate` の dsp.rs/bench_dsp.rs と `al-math` 内の
  tests/fft.rs・tests/correlation_filter.rs・
  examples/bench_correlation.rs、計5箇所に限定される)

### Confirmation

* `al-math` の公開 API(`pub`/`pub use` のシグネチャ)に `pulp` 由来の
  型・トレイトが出現しないこと
* `al-math` 以外のクレートが `pulp` に依存しないこと(Cargo.toml)
* `dsp::fft` が pack/unpack なしの split API を通ること
* scalar 強制経路と dispatch 経路の一致テストが維持されること

## Implementation Plan

* Affected paths: `crates/al-math/src/lib.rs`(`pub use`/`dispatch`/
  型エイリアスの crate 内限定化)、`crates/al-math/src/fft.rs`
  (公開シグネチャの split re/im 化とカーネルの SoA 化)、
  `crates/al-math/src/filter.rs`、`correlation.rs`(内部利用の追従)、
  `crates/al-locate/src/dsp.rs`、`crates/al-locate/examples/bench_dsp.rs`、
  `crates/al-math/tests/fft.rs`、`crates/al-math/tests/correlation_filter.rs`、
  `crates/al-math/examples/bench_correlation.rs`(公開 API 経由の
  `c64`/FFT 利用を split API へ追従。`bench_correlation.rs` の
  `previous_fft` 比較腕は c64 dispatch 版 FFT の再現であり、
  c64 除去後は公開 API で表現不能になるため、削除または split API
  での書き換えが必要)
* Patterns to follow: `dispatch` は演算の公開入口で1回だけ呼ぶ、
  カーネルは `Simd` にジェネリックな単一ソース、公開面は f64 slice と
  クレート所有の結果表現(既存設計の責務分割を維持)
* Patterns to avoid: pulp 型の再公開、公開シグネチャへのバックエンド型
  混入、AoS と SoA の二重表現、互換層や deprecation 期間(pre-release
  方針)
* Dependencies or configuration: 新規依存なし。`pulp` は `al-math` の
  内部依存として継続

## Verification

* [ ] `pulp`・`al_math::c64`・`al_math::Simd`・`al_math::WithSimd`・
  `al_math::dispatch` を参照するコードが `al-math` 内部に限定される
  ことを grep で確認する
* [ ] `al-math` の公開関数シグネチャにバックエンド固有型がないことを
  レビューで確認する
* [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が合格し、scalar/dispatch 一致テストが維持される
* [ ] `bench_dsp` で実使用経路(`dsp::fft` 経由の FFT、ncc、
  bandpass、gcc_phat、welch psd、stft-equivalent)が旧版と同等
  以上の性能であることを同一入力・条件で比較する。「旧版」は
  変更前コミットの c64 dispatch 版とし、旧コミットを別ビルドした
  `bench_dsp` と同一入力で比較する。直接 plan 利用の FFT
  マイクロベンチ(complex fft/rfft)に残る ~10% 劣後は SoA 構造
  要因の既知トレードオフとして記録する

## Pros and Cons of the Options

### split re/im 配列

* Good, because pack/unpack が完全に消え、SoA 複素演算は shuffle 不要で
  AoS 同等以上、純データゆえバックエンド中立
* Good, because `StftMatrix`・永続テンソル・`dsp::fft` の既存配置と
  一致し表現変換を新設しない
* Bad, because カーネルの複素演算を実部演算へ分解する書き換え量が
  最大で、bit-reversal swap が2要素分になる

### クレート所有 `Complex` 型

* Good, because `pulp::c64` は Pod なので `#[repr(C)]` 同型構造体から
  ゼロコストで相互キャストでき、カーネル変更不要
* Bad, because 呼出し側の詰め替えコストが残り、re/im 分離で保存される
  `StftMatrix`/永続テンソルとの間に新しい表現変換が発生する

### 不透明 `Spectrum` バッファ

* Good, because 内部表現を完全に隠蔽できる
* Bad, because 値アクセスがアクセサ/変換経由になり、外部利用が
  ベンチのみの現状では隠蔽の旨味が薄い

### 現状維持

* Good, because 変更が不要
* Bad, because バックエンド移行時に公開 API 利用箇所すべてが連鎖変更に
  なり、移行コストが利用側へ流出する

## More Information

* 調査の記録: `docs/discovery/al-math-api-fft.md`
* 上位の決定: `docs/adr/simd-arithmetic-pulp.md`(pulp 採用自体は
  維持。本 ADR はその公開面を制約する補強決定)
* `docs/designs/simd-dsp-foundation.md`(「pulp の型・関連型の公開は
  このクレートに集約」を本 ADR で「公開しない」に引き締める)
