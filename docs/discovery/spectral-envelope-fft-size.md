---
id: "DISC-8QyfdU1NnGhLDa1c1oqzf2"
type: "discovery"
status: "resolved"
title: "spectral_envelope の fft_size 契約と silent 置き換え問題"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source:
    - "../../crates/al-math/src/filter.rs"
    - "../../crates/al-locate/src/methods.rs"
    - "../../crates/al-analysis/src/reference.rs"
    - "../../crates/al-math/tests/correlation_filter.rs"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# spectral_envelope の fft_size 契約と silent 置き換え問題

## Exploration Goal

`docs/designs/al-math-api-boundary.md` の Deferred Design Concerns で
残課題として記録された、`spectral_envelope` の `fft_size` 検証不備を
調査する。`filter.rs` の
`let n = self.fft_size.max(self.samples.len().next_power_of_two());`
(filter.rs:105)は、`fft_size: 0` や非2冪(信号長より小さい場合)を
「小さい値」として黙って信号長ベースの n に置き換える。doc には
「fft_size は 2 のべき乗」とあるが強制されておらず、検証が
非一貫である。修正方針の選択肢を比較し確定する。

## Key Issues and Assumptions

- 確認済み: `fft_size: 0` は `max()` により信号長ベースの
  `next_power_of_two()` へ黙って置き換わり、計算は「成功」するが
  呼出し側の設定漏れが検出されない。サイレントな誤結果(設定漏れが
  下流の類似度・指標に混じる)が最悪のケース
- 確認済み: 非2冪でも信号長より小さい値は同様に黙って置き換わる。
  一方、非2冪で信号長より大きい値は `RealFft::new` 内部の assert で
  panic する。つまり「値の大小で検証の有無が変わる」非一貫な振る舞い
- 確認済み: `max()` の本来意図は「fft_size < 信号長」の底上げであり、
  FFT は信号長以上の長さを必要とするため信号長へ拡大する規則。
  入力検証の代替として残ったもので、検証を目的とした設計ではない
- 確認済み: 既存呼出しは `al-locate` の `SpectralEnvelope::default()`
  (`fft_size: 1024`, `bins: 32`)と `al-analysis` の
  `reference.rs`(`cfg.stft_window`、既定 4096、JSON 設定で
  上書き可能)。いずれも2冪の正常値であり、assert 追加の影響はない
- 前提: pre-release 方針により互換層・deprecation 期間は設けない。
  契約変更は assert で即時適用する
- 前提: `bins == 0` は早期 return で空ベクトルを返す既存契約であり、
  本件では変更しない

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: 入口 assert(非ゼロ2冪必須)+ 底上げ維持 + doc 明記 | 変更が assert 1行+doc+テストで最小。silent 置き換えを排除しつつ、既存呼出しが依存する底上げ規則を契約として残せる。`RealFft::new` 内部 panic より先に、分かりやすいメッセージで失敗させられる | panic 契約であり実行時エラー返却ではない(FFT 系 API 群と同じ方針だが) | yes |
| B: fft_size < 信号長も panic(底上げ撤廃、厳格化) | 契約が最も単純(fft_size は常に実効 n) | 既存呼出しが依存する底上げ動作を壊す。`SpectralEnvelope` の win が fft_size を超えるケースの全呼出し経路検証が必要で変更が広がる | no |
| C: Result/検証エラー返却に変更 | 呼出し側がエラーを処理できる | 公開シグネチャ変更が下流2クレートに波及。FFT 系の panic 契約(`rejects_*` 系)と方針がずれる。小さな欠陥に対しコストが過大 | no |
| D: 現状維持(内部で黙って置き換え) | 変更不要 | 設定漏れ・非2冪指定が下流指標へ混入し続ける。検証非一貫が残る | no |

## Tentative Conclusions and Hypotheses

- confirmed: 問題は filter.rs:105 の `max()` が入力検証を兼ねて
  しまっていること。`fft_size` が0または非2冪のとき「小さい値」と
  誤解釈され、意図しない FFT 長で成功してしまう
- confirmed: 修正境界は `Envelope::with_simd` 先頭(assert 1箇所)
  で十分。公開関数 `spectral_envelope` は `dispatch` 経由で
  `with_simd` に到達するため、入口1箇所で全経路を覆える
- confirmed: 案A(入口 assert + 底上げ維持)を採用。ユーザー承認済み
- confirmed: 既存呼出し(methods.rs 既定 1024、reference.rs の
  `cfg.stft_window`)は全て非ゼロ2冪であり影響なし
- hypothesis: `#[should_panic]` テストで `fft_size: 0` と非2冪の
  2ケースを追加すれば契約が網羅できる(実装後に確認)

## Open Questions

- assert の配置を `Envelope::with_simd` 先頭とするか
  `spectral_envelope` 関数本体とするか → design で確定
  (`with_simd` 先頭が dispatch/scalar 両経路を1箇所で覆う)
- 底上げ規則の doc 表現(「fft_size が信号長より小さい場合は
  信号長まで拡大する」)の文言 → design で確定

## Promotion Candidates

- [x] adr-doc needed: fft_size を非ゼロ2冪必須とし入口 assert で
  強制、底上げ規則は契約として維持する決定
- [x] spec-doc needed: `spectral_envelope` の fft_size 契約と
  受入基準

## Sources and Relations

- `crates/al-math/src/filter.rs`(`spectral_envelope`/
  `Envelope::with_simd`、filter.rs:105 の `max()` 行、doc コメント)
- `crates/al-math/src/fft.rs`(`RealFft::new` の非2冪 panic)
- `crates/al-locate/src/methods.rs`(`SpectralEnvelope` 既定値
  `fft_size: 1024`、呼出し箇所)
- `crates/al-analysis/src/reference.rs`(`cfg.stft_window` 経由の
  呼出し、既定 4096)
- `docs/designs/al-math-api-boundary.md` の Deferred Design
  Concerns(本件を残課題として記録した起点。本チェーンは独立した
  修正チェーンであり、旧チェーンの derives-from には連結しない)
