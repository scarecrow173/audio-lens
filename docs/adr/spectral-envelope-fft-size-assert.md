---
id: "ADR-z2U7XfOoFCqimI9EpcPc8B"
type: "adr"
status: "accepted"
title: "spectral_envelope の fft_size を入口 assert で非ゼロ 2 冪必須とし底上げ規則を維持する"
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
  derives-from:
    - "../discovery/spectral-envelope-fft-size.md"
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

# spectral_envelope の fft_size を入口 assert で非ゼロ 2 冪必須とし底上げ規則を維持する

## Context and Problem Statement

`crates/al-math/src/filter.rs` の `spectral_envelope` は doc 上
「fft_size は 2 のべき乗」と要求しながら、実装の
`let n = self.fft_size.max(self.samples.len().next_power_of_two());`
(filter.rs:105)が `fft_size: 0` や非2冪(信号長より小さい場合)を
「小さい値」として黙って信号長ベースの n に置き換える。非2冪でも
信号長より大きければ `RealFft::new` 内部の assert で panic するため、
値の大小で検証の有無が変わる非一貫な契約になっている。サイレントな
誤結果(設定漏れが下流の類似度・指標に混じる)が最悪のケースであり、
入口で契約を強制する必要がある。

## Decision Drivers

* `fft_size` の doc 契約(非ゼロの2冪)を実行時にも強制し、
  0・非2冪の入力が黙って成功しないこと
* 既存呼出しが依存する `max()` 底上げ(fft_size < 信号長なら
  信号長まで拡大)は維持し、契約として doc に明記すること。
  `SpectralEnvelope`(既定 `fft_size: 1024`)は template 窓長が
  fft_size を超える利用があり、底上げを撤廃すると既存動作を壊す
* FFT 系公開 API の既存方針(非2冪・ゼロ長は panic、
  `rejects_*` 系テスト)と揃える。エラー返却型 API への変更は
  行わない
* 変更は `al-math` 内に閉じ、下流クレートへの波及を起こさないこと
* pre-release 方針: 互換層・deprecation 期間は設けない

## Considered Options

* 案A: `Envelope::with_simd` 先頭に
  `assert!(self.fft_size.is_power_of_two(), ...)` を追加し、
  `max()` 底上げは維持して doc に規則を明記する
* 案B: 底上げを撤廃し `fft_size < 信号長` も panic とする厳格契約
  (fft_size は常に実効 FFT 長)
* 案C: 公開シグネチャを Result/検証エラー返却に変更する
* 案D: 現状維持(内部で黙って置き換え続ける)

## Decision Outcome

Chosen option: "案A", because silent 置き換えという欠陥を最小差分で
除去でき、既存呼出しが依存する底上げ動作を契約として残せる唯一の案
であり、FFT 系 API の panic 契約方針とも整合するため。

`Envelope::with_simd` 先頭に
`assert!(self.fft_size.is_power_of_two(), "fft_size must be a non-zero power of two, got {}", self.fft_size);`
を追加する(`is_power_of_two()` は 0 で false を返すため非ゼロ検査を
兼ねる)。assert は `bins == 0` の早期 return より前に置き、
bins=0 でも fft_size 契約違反が隠れないようにする。`max()` 底上げは
維持し、doc に「fft_size が信号長より小さい場合は信号長まで拡大する」
規則を明記する。

### Consequences

* Good, because `fft_size: 0`・非2冪が入力値に依らず必ず panic し、
  値の大小で検証が変わる非一貫が解消する
* Good, because panic メッセージが `RealFft::new` 内部 assert より
  分かりやすく、設定漏れを原因箇所で検出できる
* Good, because `Envelope::with_simd` の1箇所で dispatch 経路と
  scalar 強制経路の両方を覆え、assert の重複が不要
* Good, because 既存呼出し(`SpectralEnvelope` 既定 1024、
  `reference.rs` の `cfg.stft_window` 既定 4096)は全て非ゼロ2冪で
  あり影響なし
* Bad, because 契約違反が実行時 panic であり、
  `cfg.stft_window` を JSON 設定で非2冪にした場合は解析実行時に
  panic する(ただし現状でも大きな非2冪は panic しており、
  検出位置が入口に前移しただけ)

### Confirmation

* `spectral_envelope` が `fft_size: 0` および非2冪で panic することを
  `#[should_panic]` テストで確認する
* 正常2冪・底上げ(fft_size < 信号長)・`bins == 0` の既存動作が
  維持されることを既存テストで確認する
* `mise exec -- cargo test -p al-math` が合格すること

## Implementation Plan

* Affected paths: `crates/al-math/src/filter.rs` のみ
  (assert 1行 + doc 1行 + モジュール内テスト数行)。
  他クレート変更なし
* Patterns to follow: FFT 系 API の panic 契約と `rejects_*` 系
  テストの方針。assert は演算の公開入口(`with_simd` 先頭)に
  集約する
* Patterns to avoid: 入力の黙った正規化(0 → 信号長への置き換え
  のような silent fix)、Result 型への公開シグネチャ変更、
  互換層・deprecation
* Dependencies or configuration: 新規依存なし

## Verification

* [ ] `spectral_envelope(&input, 0, bins)` が panic する
  (`#[should_panic]` テスト)
* [ ] `spectral_envelope(&input, 非2冪, bins)` が panic する
  (`#[should_panic]` テスト)
* [ ] `mise exec -- cargo test -p al-math` が合格する
* [ ] `mise exec -- cargo build --workspace` が成功する

## Pros and Cons of the Options

### 案A: 入口 assert + 底上げ維持

* Good, because 変更が assert 1行+doc+テストに限定され、
  原因(検証欠落)を入口で直接潰せる
* Good, because 既存呼出しが依存する底上げ動作を壊さず、
  doc 未記載だった規則を契約として可視化できる
* Bad, because 契約違反の検出が実行時 panic に留まる
  (型や静的検査ではない)

### 案B: 底上げ撤廃の厳格契約

* Good, because 「fft_size = 実効 FFT 長」という最も単純な契約に
  なる
* Bad, because template 窓長が fft_size を超える既存利用
  (`SpectralEnvelope` の sliding window 解析)を壊す。
  全呼出し経路で「fft_size ≥ 信号長」を保証するか、
  呼出し側へ底上げ責務を移す波及変更が必要

### 案C: Result/エラー返却

* Good, because 呼出し側がエラーを回復可能に処理できる
* Bad, because 公開シグネチャ変更が `al-locate`・`al-analysis` へ
  波及し、FFT 系 API 群の panic 契約方針とも不整合になる。
  1箇所の入力検証に対しコストが過大

### 案D: 現状維持

* Good, because 変更不要
* Bad, because 0・非2冪の silent 置き換えが残り、設定漏れが
  下流の類似度・指標へ混入し続ける。検証非一貫も解消しない

## More Information

* 調査の記録: `docs/discovery/spectral-envelope-fft-size.md`
  (問題分析・選択肢比較・既存呼出し監査)
* 起点: `docs/designs/al-math-api-boundary.md` の Deferred
  Design Concerns で残課題として記録された事項の独立修正
