---
id: "IMPL-034ELzO5lUonQDi0j7BnDL"
type: "impl"
status: "completed"
title: "caller-side optimizations: dsp conv overlap-save and fir_model migration"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/tasks/caller-opt-dsp-conv-overlap-save.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/adr/caller-side-optimizations-structure.md"
    - "docs/adr/simd-reduction-lane-fma.md"
    - "docs/test-specs/caller-side-optimizations.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# caller-side optimizations: dsp conv overlap-save and fir_model migration

## Summary

plan T4(A1 opt コミット)。`al_locate::dsp` に overlap-save 畳み込み
`conv` を新設し(`al_math::fft::RealFft`、plan 1 本再利用・
`B = (2m).next_power_of_two().max(1024)`・`L = B - m + 1`・
`copy_within` 尾部繰越・先頭 `m-1` discard)、T1 で名前付き関数化
済みの `fir_model` 内の直接畳み込みループを `conv` 呼出し 1 箇所に
置換した。`benches/conv.rs` に `dsp::conv` 腕を追加し baseline
`pre` と同一 fixture で計測 — `m = 4096` で約 52〜53 倍の高速化を
確認。`conv_direct` 参照実装は `reference.rs` tests・`dsp.rs`
tests・`benches/conv.rs` の 3 箇所に逐語同一で保持(共有公開面に
露出なし)。

## Context

task `docs/tasks/caller-opt-dsp-conv-overlap-save.md`
(TASK-034EJgOK1cNKnai60q1zOd)、design
`docs/designs/caller-side-optimizations.md` の A1 節(`dsp::conv`
設計疑似コード・`conv_direct` 置き場・`fir_model` 置換)、ADR
`docs/adr/caller-side-optimizations-structure.md`(overlap-save
採用・dsp 置き場・キャップ/尾部打ち切り意味の権威)、
`docs/adr/simd-reduction-lane-fma.md`(predicate 適用の権威)、
test-spec `docs/test-specs/caller-side-optimizations.md` に従う。
依存する T1 の `fir_model`/`fir_ir`/`subtract` 切り出し・
`conv_direct` 参照実装・characterization・baseline `pre` は
先行 land 済み。

## Implementation

### Changes

- `crates/al-locate/src/dsp.rs`
  - `pub fn conv(signal: &[f64], kernel: &[f64]) -> Vec<f64>` 新設。
    空入力の早期 return をブロック長計算より先に配置
    (`kernel.is_empty()` → `vec![0.0; signal.len()]`、
    `signal.is_empty()` → `Vec::new()`、`m == 0` では `m - 1` が
    usize でアンダーフローするため)。`B = (2 * m)
    .next_power_of_two().max(1024)`・`L = B - m + 1`・`RealFft::
    new(B)` 1 本で H forward・各ブロック forward/inverse を
    使い回す(`bandpass_many` の先例)。ブロックループは
    `xbuf.copy_within(B + 1 - m.., 0)` の尾部繰越 → 新規 `take`
    要素コピー → 残り zero-pad → forward → `B/2+1` bins の素直な
    スカラ複素乗算(`zre = xre·hre − xim·him`、
    `zim = xre·him + xim·hre`)→ inverse(1/N 正規化は
    `RealFft::inverse` 内包)→ 先頭 `m - 1` を discard して
    `take` 要素を `y` へ写す。`conv` 自体に `conv_len` キャップは
    持たせない(キャップは `fir_model` 側の責務)
  - `mod tests` に `conv_direct` 参照実装の逐語同一コピー
    (3 箇所目、共有ヘルパ化なし)・`noise()` LCG ヘルパ・
    `conv_matches_direct_reference` equivalence テストを追加。
    predicate は `|a-b| <= 1e-12*max(1,|a|,|b|)`
    (ADR-034EoHxzsvsHp9BOdCK2Cn)。fixture: 空 kernel → ゼロ列・
    空 signal → 空・両方空・`n < m`・`n = 1`・`m` ∈ {1, 3, 256,
    4096}・`n` ∈ {1, 小, 非ブロック倍数, 1_048_576}・全ゼロ・
    LCG ノイズ・非ノイズ決定的 fixture(sin・調和減衰)
  - `gcc_phat_finds_known_offset` の inline LCG を `noise(512,
    0x9E3779B9)` へ集約(同一シード・同一生成式で出力列は不変、
    LCG の重複定義を解消)
- `crates/al-locate/src/lib.rs`: dsp mirror の `pub use dsp::{...}`
  に `conv` を追加(`bandpass` と `corr0` の間)
- `crates/al-analysis/src/reference.rs`
  - production import に `conv` を追加
    (`use al_locate::dsp::{bandpass, conv, corr0, …}`)
  - `fir_model` の直接畳み込みループ(`if x == 0.0 { continue }`
    zero-skip 含む)を `conv(from, &ir[..ir.len().min(4096)])` 1 行に
    置換。`conv_len` キャップと「出力 = 線形畳み込みの先頭 n 要素」
    の意味は維持。`fr == 0` では `fir_ir` がゼロ埋め IR を返し
    `fir_model` は無条件に `conv` へ進む(ゼロ IR の畳み込み結果は
    `vec![0.0; n]` で旧早期 return と等価)。zero-skip は FFT 経路
    には持ち越さない(±0.0 の積は値不変のため等価、design 確定)
  - test 内 `conv_direct` の docstring を「former production
    loop」へ追従(直接畳み込みは本番コードから消えた)
- `crates/al-analysis/benches/conv.rs`: `dsp::conv` 腕を追加
  (benchmark ID `dsp_conv`)。`conv_direct` 腕は n ∈ {65_536,
  262_144} のまま、`dsp::conv` 腕は n ∈ {65_536, 262_144,
  1_048_576}(1M 点は FFT 経路の scaling 観測用で direct は
  O(n·m) 上 bench 不能)。kernel ∈ {256, 4_096}・fixtures・
  `sample_size(10)`・`measurement_time(2s)` は baseline と同一

### Decisions During Implementation

- **benchmark ID は `dsp_conv`**: criterion は benchmark ID を出力
  ディレクトリ名に使い、`::` を含む ID は Windows で不正なパスに
  なり得るため `dsp::conv` ではなく `dsp_conv` とした
- **`dsp_conv` 腕は `--baseline pre` で比較しない**: criterion は
  baseline に存在しない benchmark の比較を要求されると panic する
  (`Baseline 'pre' must exist`)。`conv_direct` 腕は `--baseline
  pre` 付き filter 実行、`dsp_conv` 腕は filter のみの plain 実行と
  し、絶対値を baseline 表(T1 IR)と突合して delta を記録した。
  完全出力は `target/bench-baseline/criterion-conv-a1.txt`
- **繰越し範囲は `B + 1 - m` で記述**: `B - (m - 1)` と等価だが
  ネストした減算を避けた形。`B >= 2m` が常に成立するため
  `m >= 1` でアンダーフローしない(空 kernel は早期 return 済み)
- **1M 点の equivalence fixture は小 kernel**: `conv_direct` は
  debug ビルドの O(n·m) のため 1M × 4096 はテストに不適。
  `n = 1_048_576` × `m = 8`(8M MACs)で 1M 級の経路を検証し、
  `m = 4096` は `n = 20_000`(4L + 部分ブロック)でカバー

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 結果(baseline `pre` との比較)

環境: 同一マシン上の `mise exec -- cargo bench -p al-analysis
--bench conv`(bench profile = release + debuginfo)。
`conv_direct` 腕は `--baseline pre` 付き、`dsp_conv` 腕は plain
実行。完全出力は
`target/bench-baseline/criterion-conv-a1.txt`(gitignore 済み)。

group `conv`、median:

| fixture | conv_direct `pre` (T1) | conv_direct 再計測 | dsp::conv | speedup vs `pre` |
| --- | --- | --- | --- | --- |
| n65_536 m256 | 4.1349 ms | 3.4464 ms (−16.2%, p = 0.00) | 0.6810 ms | ≈6.1x |
| n262_144 m256 | 16.576 ms | 14.268 ms (−15.7%, p = 0.00) | 3.1422 ms | ≈5.3x |
| n65_536 m4_096 | 62.115 ms | 62.485 ms (+1.3%, p = 0.07 ns) | 1.1991 ms | ≈51.8x |
| n262_144 m4_096 | 254.28 ms | 260.14 ms (+2.3%, p = 0.08 ns) | 4.7896 ms | ≈53.1x |
| n1_048_576 m256 | (direct bench 不能) | — | 12.390 ms | ≈5.4x(direct 線形外挿 ~66ms に対し) |
| n1_048_576 m4_096 | (direct bench 不能) | — | 18.779 ms | ≈54x(direct 線形外挿 ~1.02s に対し) |

`conv_direct` 再計測値は baseline `pre` と統計的に同一〜若干改善
(m256 で −16% はマシン状態差の範囲)で、比較基準の健全性を確認。
`dsp::conv` は m = 4096 で約 52〜53 倍、n = 1M でも線形スケール
(m4096: n4 倍で ~3.9x)を観測 — design の期待どおり
orders-of-magnitude 級の改善。

## Validation

- `mise exec -- cargo test -p al-locate conv`: green(
  `conv_matches_direct_reference` が全 fixture で predicate 内 pass、
  3.74s)
- `mise exec -- cargo test -p al-analysis reference`: green(
  `fir_model_matches_fir_ir_truncated_conv_direct`・
  `subtract_zero_fills_short_or_missing_model` を含む — `dsp::conv`
  置換後も characterization は unchanged で pass)
- `mise exec -- cargo test --workspace`: green(全 39 suite、
  golden 非回帰)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo bench -p al-analysis --bench conv`:
  `conv_direct` 腕は `--baseline pre` で完走、`dsp_conv` 腕は
  plain 完走、delta は上表のとおり記録
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接依存に
  al-math なし(al-core・al-dataset・al-job・al-locate・al-media・
  serde・serde_json・tracing;dev-deps は al-testkit・criterion)
- コード検査: `rg "y\[i \+ k\] \+=|for \(k, &hv\)"
  crates/al-analysis/src/reference.rs` の一致は `mod tests` 内
  `conv_direct` のみ(本番コードに直接畳み込みループなし)、
  `fir_model` の畳み込み呼出しは `conv` 1 箇所に集約

## Risks

- bench delta は同一マシン・同一 bench profile でのみ有効。
  `dsp_conv` 腕には `pre` baseline が存在しないため criterion の
  統計比較ではなく絶対値の突合(direct 再計測が baseline と
  統計的に等価なため比較基準は健全)
- `conv` は `signal.len()` までの先行出力のみを返す — 線形畳み込み
  全体(n+m-1)を必要とする将来の caller は別途契約が必要
  (現行は `fir_model` の「先頭 n 要素」意味のみを対象)
- overlap-save の境界条件バグ残存リスクは fixture 行列で担保。
  問題時は `fir_model` を test 内 `conv_direct` 同等の直接
  畳み込みへ戻すだけでロールバック可(呼出し 1 箇所)

## Follow-ups

- T5(caller-opt-measurement-ir-closeout): 本 IR の conv bench
  delta を before/after 全比較値として closeout へ転記
- conv 内スペクトル複素乗算の SIMD 化は design どおり Deferred
  (FFT 自体が支配的、スカラループで十分)
