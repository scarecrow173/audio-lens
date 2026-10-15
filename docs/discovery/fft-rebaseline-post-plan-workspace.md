---
id: "DISC-034GsCb9zcTq5N9ewsGvON"
type: "discovery"
status: "resolved"
title: "FFT re-baseline after plan/workspace split (issue #42)"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source:
    - "remaining-issues-roadmap.md"
    - "https://github.com/scarecrow173/audio-lens/issues/42"
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
    - "docs/discovery/remaining-issues-roadmap.md"
    - "docs/discovery/al-math-api-fft.md"
    - "docs/adr/al-math-api-fft-split-re-im.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/remaining-issues-roadmap.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# FFT re-baseline after plan/workspace split (issue #42)

## Exploration Goal

Issue #42 は旧スカラー複素 FFT が NumPy 比 ~2.7 倍劣後していた時代に起票され、
rustfft(または同等の混合基数・実数 FFT・SIMD 対応ライブラリ)への換装を提案
していた。しかし前提は PR #44/#45/#52 で変化した: `RealFft` は pulp SIMD 化
済みで split re/im 表現を持ち、plan/workspace が分離されている。本 discovery
は **再ベースライン計測を行い、換装提案の存否を判定する**ことを目的とする。

## Key Issues and Assumptions

- 計測対象は `al-math` の `RealFft` カーネル(plan 生成・forward・inverse)。
- 比較基準は issue 起票時の動機であった NumPy `rfft` との相対性能。
- pow2 非適合入力での padding waste は混基数ライブラリの残存優位点として
  別途観測する。
- 実呼び出し側(`al-locate` overlap-save、`al-analysis` NCC 等)はブロック
  サイズを自前で選ぶため、pad waste は入力非適合時のみ発生する限定要因。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| rustfft / pocketfft へ換装 | 混基数で pad 効率改善の余地 | 依存追加・API 再設計・検証やり直し。SIMD 実装の優位性は未証明 | no |
| 現行 pulp `RealFft` 維持(計測で判定) | 依存なし・既検証 | pow2 限定の pad waste が残る | yes |

## Measurement (2026-10-15, Windows x86_64, release)

`crates/al-math/benches/fft_kernel.rs`(Criterion 0.8.2)。plan / forward /
inverse を分離計測し、pad 非適合サイズは realized pow2 へ zero-pad して
forward を計測した。

### `RealFft` カーネル(median)

| n | plan | forward | inverse |
| --- | --- | --- | --- |
| 1024 | 3.49 µs | 3.51 µs | 3.49 µs |
| 4096 | 12.5 µs | 15.4 µs | 15.4 µs |
| 16384 | 45.0 µs | 79.2 µs | 81.2 µs |
| 65536 | 267 µs | 380 µs | 383 µs |
| 262144 | 1.81 ms | 1.70 ms | 1.75 ms |
| 1048576 | 8.62 ms | 10.7 ms | 11.3 ms |
| 4194304 | 35.0 ms | 57.7 ms | 63.1 ms |

### NumPy `rfft` との比較(numpy 2.5.4 / Python 3.12, 同一環境, median)

| n | `RealFft::forward` | NumPy rfft | 比較(al-math 側) |
| --- | --- | --- | --- |
| 1024 | 3.5 µs | 11.3 µs | 約 3.2x 高速 |
| 16384 | 79 µs | 117 µs | 約 1.5x 高速 |
| 65536 | 380 µs | 851 µs | 約 2.2x 高速 |
| 262144 | 1.70 ms | 3.69 ms | 約 2.2x 高速 |
| 1048576 | 10.7 ms | 19.7 ms | 約 1.8x 高速 |
| 4194304 | 57.7 ms | 106.5 ms | 約 1.8x 高速 |

### pow2 padding waste(pad 非適合入力)

| 入力 n + kernel | realized pow2 | waste | forward |
| --- | --- | --- | --- |
| 65536 + 256 | 131072 | 1.99x | 781 µs |
| 262144 + 4096 | 524288 | 1.97x | 4.21 ms |
| 800000 + 4096 | 1048576 | 1.30x | 10.8 ms |
| 1500000 + 8192 | 2097152 | 1.39x | 26.0 ms |

## Tentative Conclusions and Hypotheses

- **confirmed**: 起票時の動機であった「NumPy 比 ~2.7x 劣後」は反転しており、
  現行 pulp SIMD `RealFft` は全計測サイズで NumPy rfft を約 1.5-3.2x
  上回る。換装提案の前提は消滅した。
- **confirmed**: plan 生成コストは forward 1 回分と同程度(1M で 8.6ms vs
  10.7ms)。#52 の plan/workspace 分離による plan 再利用は引き続き有効。
- **confirmed**: 残存差異は混基数による pad 効率のみ。観測した waste は
  1.30-1.99x だが、実呼び出し側はブロックサイズを pow2 で自前選択する
  overlap-save 構造のため、悪条件は入力長が直接 FFT サイズを決める経路に
  限定される。
- **hypothesis**: rustfft 換装が pad waste を改善し得るサイズ帯でも、
  カーネル単位で現行実装を 1.5x 以上上回る保証はなく、依存追加・API
  再設計・数値等価検証のコストに見合わない。

## Decision

**Issue #42 は close(継続実装なし)が妥当。** rustfft 換装は棚上げとし、
将来プロファイルで FFT 支配かつ pow2 非適合入力が律速と判明した場合にのみ
prototype 比較を再検討する(依存追加はその時点で再承認)。

## Open Questions

- なし(判定に必要な計測は完了)。

## Promotion Candidates

- [ ] spec-doc needed: なし(計測・判定のみ)
- [ ] adr-doc needed: なし(既存 ADR `al-math-api-fft-split-re-im` の維持判断)

## Sources and Relations

- Issue #42(post-#52 FFT re-baseline measurement / close-or-continue)
- `docs/discovery/remaining-issues-roadmap.md` — #42 確定決定
  「rustfft 棚上げ・まず再計測して判定」
- `docs/adr/al-math-api-fft-split-re-im.md` — 現行 FFT 境界の正規決定
- 計測コード: `crates/al-math/benches/fft_kernel.rs`
- 計測環境: Windows x86_64, criterion =0.8.2, numpy 2.5.4
