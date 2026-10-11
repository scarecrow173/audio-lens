---
id: "SPEC-24roIb4kfGhLAuK7OKg933"
type: "spec"
status: "approved"
title: "spectral_envelope の fft_size 契約: 非ゼロ 2 冪必須と底上げ規則"
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
    - "../adr/spectral-envelope-fft-size-assert.md"
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
---

# spectral_envelope の fft_size 契約: 非ゼロ 2 冪必須と底上げ規則

## What Should Be Built

`al-math` の `spectral_envelope(samples, fft_size, bins)` において、
`fft_size` が非ゼロの2冪であることを入口の assert で強制する。
`fft_size: 0` および非2冪の入力は panic とする。fft_size が信号長
より小さい場合に信号長まで拡大する `max()` 底上げは既存契約として
維持し、doc に明記する。

## Why Now

`Envelope::with_simd` の
`let n = self.fft_size.max(self.samples.len().next_power_of_two());`
(filter.rs:105)は、`fft_size: 0` や非2冪(信号長より小さい場合)を
黙って信号長ベースの n に置き換える。非2冪でも信号長より大きければ
`RealFft::new` 内部で panic するため検証が非一貫であり、最悪の
ケースは silent な誤結果(設定漏れが下流の類似度・指標に混じる)。
修正方針(案A)がユーザー承認済みであり、pre-release のため互換層
なしで契約を引き締められる。

## Users and Value

- `al-locate` / `al-analysis` の実装者: fft_size の設定漏れや
  非2冪指定を、silent な誤結果ではなく原因箇所の panic として
  検出できる
- API 利用者: doc に明記された底上げ規則により、fft_size < 信号長
  のとき実効 FFT 長がどう決まるかを読み取れる

## Scope

### In Scope

- `Envelope::with_simd` 先頭への
  `assert!(self.fft_size.is_power_of_two(), ...)` 追加
- `spectral_envelope` の doc への底上げ規則(fft_size < 信号長なら
  信号長まで拡大)の明記
- `#[should_panic]` テストの追加(`fft_size: 0` と非2冪)
- `filter.rs` モジュール内テストの範囲での変更

### Out of Scope

- `max()` 底上げ自体の撤廃・厳格化(fft_size < 信号長の panic 化)
  — 既存呼出しが依存するため維持
- Result/エラー返却への公開シグネチャ変更 — FFT 系 API の panic
  契約方針と整合しない
- `bandpass`/`bandpass_many` 等他関数への検証追加 — 本件は
  `spectral_envelope` の fft_size のみ
- `bins` の契約変更(`bins == 0` は空ベクトルを返す既存契約を維持)
- 他クレート(`al-locate`/`al-analysis`)の変更 — 既存呼出しは
  全て非ゼロ2冪であり影響なし

## Requirements

- `fft_size` は非ゼロの2冪でなければならず、`0` および非2冪は
  panic すること(契約違反は assert で検出する)
- `fft_size` が信号長より小さい場合、実効 FFT 長 n は
  `fft_size.max(samples.len().next_power_of_two())` で決まり、
  この規則が doc に明記されていること
- `bins == 0` は空ベクトルを返す既存契約が維持されること
- 既存の分析意味・出力形式を変更しないこと
- scalar 強制経路と dispatch 経路が同一入力で一致し続けること

## Acceptance Criteria

- [ ] `spectral_envelope(&input, 0, bins)` が panic する
  (`#[should_panic]` テスト)
- [ ] `spectral_envelope(&input, 非2冪, bins)` が panic する
  (`#[should_panic]` テスト)
- [ ] doc に「fft_size が信号長より小さい場合は信号長まで拡大する」
  規則が明記されている
- [ ] `mise exec -- cargo test -p al-math` が合格する
  (既存テスト全合格を含む)
- [ ] `mise exec -- cargo build --workspace` が成功する

## Deferred Design Concerns

- `bandpass`/`bandpass_many` の周波数引数・ゼロ長入力に対する
  検証方針の棚卸し — 本 spec では `spectral_envelope` のみを対象と
  し、他関数の契約監査は別途行う
- `cfg.stft_window`(JSON 設定)の非2冪値に対する起動時検証 —
  現状は実行時 panic で検出される。設定読み込み時の早期検証は
  別課題

## Sources and Relations

- 調査: `docs/discovery/spectral-envelope-fft-size.md`(問題分析・
  選択肢比較・既存呼出し監査)
- 決定: `docs/adr/spectral-envelope-fft-size-assert.md`(案A:
  入口 assert + 底上げ維持の契約)
