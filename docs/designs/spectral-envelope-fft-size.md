---
id: "DESIGN-UuNFjPBJ3weQkbJMWWpkh0"
type: "design"
status: "approved"
title: "spectral_envelope の fft_size 入口 assert と底上げ契約の明文化"
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
  implements:
    - "../specs/spectral-envelope-fft-size.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/spectral-envelope-fft-size.md"
    - "../adr/spectral-envelope-fft-size-assert.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# spectral_envelope の fft_size 入口 assert と底上げ契約の明文化

## Context

`spectral_envelope` の `fft_size` は doc 上「2 のべき乗」を要求し
ながら、`Envelope::with_simd` 内の
`let n = self.fft_size.max(self.samples.len().next_power_of_two());`
(filter.rs:105)が0や非2冪を黙って信号長ベースの n に置き換え、
検証が非一貫(非2冪でも信号長より大きければ `RealFft::new` 内部で
panic)。ADR で「入口 assert + 底上げ維持」(案A)が決定された。
本設計はその具体化であり、変更は `crates/al-math/src/filter.rs`
のみで完結する。

## Scope

### In Scope

- `Envelope::with_simd` 先頭への assert 追加
- `spectral_envelope` doc への底上げ規則明記
- `filter.rs` モジュール内テストへの `#[should_panic]` 追加

### Out of Scope

- `spectral_envelope` 関数本体・`dispatch` 呼出し構造の変更
- `bins` 契約・`bandpass` 系関数の検証
- 他クレートの変更(既存呼出しは全て非ゼロ2冪)

## Components and Boundaries

- `crates/al-math/src/filter.rs`:
  - `Envelope::with_simd` の先頭(`if self.bins == 0` の早期
    return より前)に次を追加する:

    ```rust
    assert!(
        self.fft_size.is_power_of_two(),
        "fft_size must be a non-zero power of two, got {}",
        self.fft_size
    );
    ```

    `usize::is_power_of_two()` は 0 で false を返すため、非ゼロ
    検査と2冪検査を1式で兼ねる
  - `spectral_envelope` の doc コメントを更新し、
    「fft_size は 2 のべき乗(0 は不可)。fft_size が信号長より
    小さい場合は信号長まで拡大する。bins=0 は空結果を返す」
    という契約を明記する

## Data and Control Flow

- assert の配置を `with_simd` 先頭とする理由:
  `spectral_envelope` は `dispatch(Envelope { ... })` を通り、
  `with_simd` が scalar 強制経路(`pulp::Scalar.vectorize`)と
  dispatch 経路の双方で唯一の実行本体である。ここに置けば
  assert が1箇所で全経路を覆い、公開関数側との二重検査が不要
- `bins == 0` の早期 return より前に置く理由: bins=0 のとき
  fft_size 契約違反が早期 return に隠れて panic しない経路が
  残るのを防ぐ。契約は引数の妥当性であり、処理スキップと独立に
  検査する
- assert 後の `n` 決定は現行の `max()` 式をそのまま維持する。
  assert により fft_size は非ゼロ2冪が保証され、`max()` は
  「2冪だが信号長より小さい値を信号長まで拡大する」という
  本来の底上げ責務のみを担う

## Non-Functional Constraints

- 数値結果の同一性: assert の追加は早期失敗のみで、正常入力の
  計算経路・結果を一切変更しない
- 既存呼出し影響: `al-locate` の `SpectralEnvelope::default()`
  (`fft_size: 1024`)、`al-analysis` の `cfg.stft_window`
  (既定 4096、JSON 設定値)はいずれも非ゼロ2冪であり、
  振る舞い変化なし
- panic 契約: FFT 系 API 群の既存方針(非2冪・ゼロ長は panic)
  と整合する

## Risks and Trade-offs

- `cfg.stft_window` を JSON 設定で非2冪にした場合、従来は
  信号長との大小で panic 有無が変わったが、今後は必ず
  `spectral_envelope` 入口で panic する → 検出位置が入口に
  前移しただけで、新しい失敗モードではない。設定読み込み時の
  早期検証は別課題(spec の Deferred Design Concerns)
- assert メッセージに実際の値を含めるため、デバッグ時に
  入力値を即座に特定できる

## Deferred Design Concerns

- `bandpass`/`bandpass_many` の引数検証の棚卸し — 別課題
- `cfg.stft_window` の非2冪値に対する設定読み込み時の早期検証 —
  別課題

## Verification Notes

- [ ] `fft_size: 0` で panic する `#[should_panic]` テスト
- [ ] 非2冪の `fft_size`(例: 1000)で panic する
  `#[should_panic]` テスト
- [ ] 既存テスト `scalar_and_dispatch_match_for_filters_and_
  envelopes`(fft_size: 256)と `tests/correlation_filter.rs`
  (fft_size: 256)が合格し続ける
- [ ] `mise exec -- cargo test -p al-math` 合格
- [ ] `mise exec -- cargo build --workspace` 成功
