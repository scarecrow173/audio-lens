---
id: "TSPEC-brMJfBQXA6vJ3f4Vm4UKjR"
type: "test-spec"
status: "proposed"
title: "spectral_envelope の fft_size 契約検証"
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
    - "docs/specs/spectral-envelope-fft-size.md"
    - "docs/designs/spectral-envelope-fft-size.md"
    - "docs/adr/spectral-envelope-fft-size-assert.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# spectral_envelope の fft_size 契約検証

## Purpose

`spectral_envelope` の `fft_size` に対する入口 assert(案A)が、
契約違反(0・非2冪)を確実に panic させ、かつ既存の正常動作
(2冪・底上げ・bins=0・scalar/dispatch 一致)を損なわないことを
保証する検証意図を記録する。assert 追加は早期失敗のみの変更であり、
「panic するようになった」ことと「正常経路が変わっていない」ことを
分けて確認する必要がある。

廃止条件: `fft_size` の契約が再度変更され本 spec の Rules が新しい
test-spec に置き換わる場合に `deprecated` / `superseded` とする。

## Feature

`al-math` の `spectral_envelope` が非ゼロ2冪の `fft_size` を assert
で強制し、信号長より小さい fft_size を信号長まで拡大する底上げ規則を
維持する。

## Rules

- R1(fft_size 契約): `fft_size: 0` および非2冪(信号長との
  大小に依らず)で panic する。panic は `Envelope::with_simd`
  先頭の assert によるもので、`RealFft::new` 内部 assert より先に
  分かりやすいメッセージで発生する
- R2(底上げ規則): `fft_size` が2冪でも信号長より小さい場合、
  実効 FFT 長は信号長の `next_power_of_two()` まで拡大され、
  正常に結果を返す。この規則が `spectral_envelope` の doc に
  明記されている
- R3(正常経路の不変): 正常2冪の `fft_size` での出力値が変更前と
  同一である。scalar 強制経路と dispatch 経路が同一入力で
  一致し続ける(既存 `scalar_and_dispatch_match_for_filters_and_
  envelopes` を維持)
- R4(bins 契約): `bins == 0` は空ベクトルを返す既存契約を維持
  する。assert は bins 早期 return より前にあり、fft_size 契約
  違反は bins=0 でも隠れない

## Examples

- Given `fft_size: 0`、When `spectral_envelope` を呼んだとき、
  Then panic する(`#[should_panic]`)
- Given 非2冪の `fft_size`(例: 1000)、When `spectral_envelope`
  を呼んだとき、Then panic する(`#[should_panic]`)
- Given 信号長より小さい2冪の `fft_size`(例: 信号長 3000・
  fft_size 1024)、When `spectral_envelope` を呼んだとき、
  Then panic せず信号長ベースの n で結果を返す(底上げ維持)
- Given `bins == 0`、When `spectral_envelope` を正常な fft_size で
  呼んだとき、Then 空ベクトルを返す
- Given 正常2冪の `fft_size`、When scalar 強制経路と dispatch
  経路で同一入力を処理したとき、Then 出力が `1e-10` 絶対誤差内で
  一致する(既存テスト維持)

## Guarantees

- 契約違反の入力が `RealFft::new` の内部 assert ではなく入口の
  明示的メッセージで panic し、設定漏れを原因箇所で検出できる
- 値の大小で検証の有無が変わる非一貫が解消され、0・非2冪は
  常に panic する
- 正常入力の計算経路・出力値は変更前と同一であり、下流の
  類似度・指標・golden 出力に影響しない
- scalar/dispatch 一致テストが維持され、assert が ISA 選択に
  依存しないことが保証される

## Non-goals

- 底上げ規則の撤廃・厳格化(fft_size < 信号長の panic 化)は
  検証しない(ADR で不採用)
- `bandpass`/`bandpass_many` の引数検証は対象外
- panic メッセージの文言そのものは検証しない(「panic する」
  ことのみを検証する)
- `cfg.stft_window` の設定読み込み時検証は対象外(実行時 panic
  での検出は本 Rules の範囲)

## Risk

- `#[should_panic]` テストを落とすと、assert の削除や条件緩和が
  silent 置き換えの復活として気づかれず、設定漏れが下流指標へ
  混入し続ける
- 底上げ維持の確認を落とすと、assert の過剰な厳格化(fft_size <
  信号長まで panic にする)が既存呼出しを壊す変更として混入しうる
- scalar/dispatch 一致の維持確認を落とすと、assert 配置ミス
  (dispatch 経路のみ等)が見逃され経路間で契約がずれる
