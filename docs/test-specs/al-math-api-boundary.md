---
id: "TSPEC-034DYifVBA3Fc2Vs952TLz"
type: "test-spec"
status: "proposed"
title: "al-math 公開 API 中立化と split re/im FFT 境界"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/al-math-api-boundary.md"
    - "docs/designs/al-math-api-boundary.md"
    - "docs/adr/al-math-api-fft-split-re-im.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# al-math 公開 API 中立化と split re/im FFT 境界

## Purpose

`al-math` 公開 API のバックエンド中立化と `Fft`/`RealFft` の split
re/im 化(S1〜S6 スライス)が、task 分割・逐次実装の過程でも受入基準を
落とさないことを保証する検証意図を記録する。公開面の型中立性・
FFT の数値契約・性能不低下は、将来のバックエンド置き換えや
`StftMatrix` の `RealFft` 直結設計の前提となる不変条件であり、
「実装が終わった」という主張ではなく契約として残す。

廃止条件: FFT 境界の表現が再度変更され本 spec の Rules が新しい
test-spec に置き換わる場合、または pulp を除去して公開面の中立性が
構造的に自明になる場合に `deprecated` / `superseded` とする。

## Feature

`al-math` の公開 API がバックエンド中立の型(f64 slice とクレート
所有構造体)のみで構成され、split re/im 化された FFT/RealFft
カーネルが、変更前と同等の数値結果・境界契約・性能を維持する。

## Rules

- R1(公開面の中立性): `al-math` の公開シグネチャ(`pub`/`pub use`)
  に pulp 由来の `Simd`/`WithSimd`/`c64`/`F64s`/`C64s`/`dispatch`
  が出現しない。`crates/` 全体で `al_math::c64` 等の外部参照と
  `al-math` 以外の `pulp` 参照が 0 件であることを grep 監査し、
  `cargo build --workspace` が成功する(grep が取りこぼす
  brace import 等はコンパイルで検出する)
- R2(split re/im API 契約): `Fft::transform(&self, re: &mut [f64],
  im: &mut [f64], inverse: bool)` と `RealFft::forward(input,
  out_re, out_im)` / `inverse(in_re, in_im, output)` が、re/im
  長さ一致・plan 長一致を assert し、非2冪長・ゼロ長では panic
  する。`RealFft::inverse` の `in_re`/`in_im` は同一スライスを
  共有してはならない(debug assert)。DC/Nyquist bin の虚部は
  forward が 0 を書き、inverse が無視する
- R3(scalar/dispatch 一致): scalar 強制経路と dispatch 経路が
  同一入力で一致する。誤差閾値は現行 `1e-10 * (1 + |b|)` 相対を
  維持し、緩和しない
- R4(独立参照比較の保持): 複素 FFT/RealFft の出力が独立参照実装
  (radix-2 `reference()`・naive DFT)との直接比較で一致する
  既存テストを split API 形で保持する。`al_math::c64` を使わず
  re/im 演算で記述し、共有カーネルの系統誤りを検出可能にする
- R5(振る舞い不変): 既存の分析意味・golden 出力・結果形式が
  不変であり、`al-math`/`al-locate`/`al-analysis` の全テストが
  合格する
- R6(性能不低下): `bench_dsp` の FFT 系カーネル(`fft 4M`、
  `complex fft 4M`、`rfft 4M`、`stft-equivalent`)と
  `bandpass`/`ncc`/`gcc_phat` が、変更前コミットを別ビルドした
  旧版と同一入力で同等以上である

## Examples

- Given n = 1..1024 の全 2 冪長と forward/inverse の各方向、
  When `Fft::transform` を scalar/dispatch 両経路で実行したとき、
  Then re/im 各要素が `1e-10 * (1 + |b|)` 相対内で一致し、
  forward→inverse の round trip が入力を復元する
- Given DC bin と Nyquist bin を含む実数入力、When `RealFft` の
  forward/inverse を実行したとき、Then 非負周波数 N/2+1 bins が
  複素 FFT の対応 bin と一致し、端点虚部は forward で 0・
  inverse で無視される
- Given 非2冪長・ゼロ長の入力、When 公開 API を呼んだとき、
  Then panic する(既存 `rejects_*` 系テストを split API で維持)
- Given `Fft` を共有して複数スレッドから呼ぶ利用形態、
  When 並列で transform を実行したとき、Then 決定的かつ
  正しい結果を返す(`immutable_complex_plan_can_be_shared` 維持)
- Given `crates/` 全体のソース、When `al_math::c64`/`Simd`/
  `WithSimd`/`F64s`/`C64s`/`dispatch`/`pulp` を監査したとき、
  Then `al-math` 外からの参照が 0 件である
- Given 変更前コミットを別ビルドした `bench_dsp`/
  `bench_correlation`、When 同一入力で FFT 系・bandpass/ncc/
  gcc_phat 腕を比較したとき、Then 新版が旧版と同等以上である

## Guarantees

- 公開 API の型中立性は grep 監査と workspace ビルドの二重で保証
  され、将来 pulp 以外のバックエンドへの移行が `al-math` 内部に
  閉じる
- split re/im 化後も、forward/inverse round trip、DC/Nyquist、
  短尺・任意べき乗長、panic 契約が既存テストと同等の網羅性で
  保証される
- 独立参照(radix-2/naive DFT)比較が split API で保持され、
  scalar/dispatch 一致テストでは検出不能な系統誤りを検出できる
- `dsp::fft` 経路の `Vec<c64>` 確保と往復コピーが除去されても、
  分析出力(golden)と API 利用側の意味は変わらない
- `bench_correlation` の `previous_fft` 腕削除後も、残る腕で
  カーネル性能の旧版比較が可能である

## Non-goals

- 内部表現の bit 完全一致は要求しない。複素演算の分解形は
  数学的同値と 1e-10 相対閾値内の一致を保証するに留める
- release ビルドでの `in_re`/`in_im` 同一スライス検査は行わない
  (debug assert とドキュメント契約)
- `al-math` 内部での `pulp`/`c64` 利用の完全除去は検証しない
  (公開面の除去が目的であり内部利用は許容)
- ベンチの絶対性能値や新規最適化目標は検証しない(旧版比較での
  不低下のみ)
- `StftMatrix` の `RealFft` 直結・ベクトルプリミティブ公開など
  後続設計の振る舞いは対象外

## Risk

- 独立参照比較を落とすと、`stage`/`recombine_pair`/`recombine` の
  書き換えミスが scalar/dispatch 両経路に共通する系統誤りとして
  残留し、下流の解析結果を静かに劣化させる
- 公開面の中立性監査を落とすと、将来の変更で `pub use` 経由の
  pulp 露出が再発し、バックエンド置き換え時に連鎖修正が復活する
- panic 契約・長さ assert の検証を落とすと、非2冪長やゼロ長入力が
  未定義動作・誤結果になり、利用側で検出不能な破損を生む
- 性能比較を落とすと、bit-reversal の 2 ストア化や SoA 化による
  回帰を検出できず、本変更の副次的目的(配置変換コスト除去)が
  相殺されても気づかない
