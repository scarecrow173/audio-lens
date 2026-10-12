---
id: "IMPL-034EKRlMleadCH45JoiJfj"
type: "impl"
status: "completed"
title: "caller-side optimizations: stft Fft plan hoist"
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
    - "docs/tasks/caller-opt-stft-fft-plan-hoist.md"
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

# caller-side optimizations: stft Fft plan hoist

## Summary

plan T2(A3 opt)。`stft` のフレームループが毎フレーム `dsp::fft`
経由で `Fft::new` を構築していたのを、plan をガード内・ループ外に
巻き上げて全フレームで再利用する内部リファクタに改めた。
`al_locate::dsp` に `Fft` を re-export(新設キャリアではなく既存
`pub use al_math::correlation::{...}` と同 pattern)し、crate root
mirror にも追加。`stft` の公開シグネチャ・`StftMatrix` 出力・
フレームバッファ確保は不変で、T1 の `stft_is_bit_exact_*`
characterization が opt 後もそのまま green。

## Context

task `docs/tasks/caller-opt-stft-fft-plan-hoist.md`
(TASK-034EJgxFS2IJznrYHnuNJy)、design
`docs/designs/caller-side-optimizations.md` の A3 節(plan
キャリア・stft 変更コード・ビット一致根拠)、ADR
`docs/adr/caller-side-optimizations-structure.md`(plan 所有境界:
dsp 公開・`dsp::fft` 維持・フレームバッファ巻き上げなし)、
test-spec `docs/test-specs/caller-side-optimizations.md`(Rule B
ビット一致ゲート)に従う。依存タスク T1
(`caller-opt-bench-baseline-characterization`)の
characterization と criterion baseline `pre` が先行 land 済み。

## Implementation

### Changes

- `crates/al-locate/src/dsp.rs`: `pub use al_math::fft::Fft;` を
  `al_math` re-export ブロックに追加(correlation と filter の間、
  module 名アルファベット順)
- `crates/al-locate/src/lib.rs`: dsp mirror 規約に倣い既存
  `pub use dsp::{...}` リストへ `Fft` を追加
- `crates/al-analysis/src/stft.rs`:
  - production import を `use al_locate::dsp::{Fft, mag_sq, scale};`
    に変更(design 指定形)
  - `samples.len() >= window && window > 0 && hop > 0` ガード内・
    フレームループ外で `let plan = Fft::new(fft_size);` を1回構築
    (`window == 0` では `next_power_of_two() == 1` になるため、
    「フレームが走る場合のみ plan を構築する」現行意味に揃えて
    ガード内に配置)
  - フレームループ内を `plan.transform(&mut fre, &mut fim, false)`
    呼出しに変更
  - フレームバッファ `vec![0.0; fft_size]` ×2 は `re.push(fre)` /
    `im.push(fim)` で出力 `StftMatrix` 行へ move されるため現行の
    まま(巻き上げ対象外、ADR 確定事項)
  - module doc を `dsp::fft` single-shot から `Fft` plan 再利用の
    記述へ更新
  - `mod tests` に `use al_locate::dsp::fft;` を追加 — ビット一致
    oracle `reference_frames` が意図的に single-shot `dsp::fft`
    経路を使うため test 側で import(lib target で unused import
    warning にならないよう分離)

### Decisions During Implementation

- **`fft` import の配置**: design の指定コードは production import
  から `fft` を外す形だったが、T1 で追加した oracle
  `reference_frames` が single-shot `dsp::fft` を経由する仕様上
  必要。`mod tests` 内へ `use al_locate::dsp::fft;` を移し、
  production import は design 指定形 `{Fft, mag_sq, scale}` に揃えた
- **`Fft::new` をガード外へ出さない**: `window == 0` でも
  `Fft::new(1)` は構築可能だが、現行の「フレームが走る場合のみ
  plan を構築する」意味を保つため design どおりガード内に固定
- **`dsp::fft`・`stft` のシグネチャ不変**: single-shot caller
  (`codec.rs`・`dynamics.rs`・`ir.rs`・`noise.rs`・
  `reference.rs` の irfft・`expensive/tests.rs`)と全 stft caller
  は無修正のまま

## Related Experiments

### Adopted

<!-- No experiment logs: mechanical task per task-doc. -->

### Rejected

<!-- No rejected experiments. -->

## Bench 結果(baseline `pre` との delta)

環境: 同一マシン上の `mise exec -- cargo bench -p al-analysis
--bench stft -- --baseline pre`(bench profile = release +
debuginfo)。完全出力は
`target/bench-baseline/criterion-stft-a3.txt`(gitignore 済み)。

group `stft`、window/hop 4096/2048、median:

| 信号長 | baseline `pre` | A3 適用後 | delta |
| --- | --- | --- | --- |
| 524_288 | 13.840 ms | 11.934 ms | −13.8%(CI [−14.5%, −13.0%], p = 0.00, improved) |
| 1_048_576 | 26.973 ms | 24.520 ms | −9.1%(CI [−10.5%, −7.7%], p = 0.00, improved) |

フレーム数に比例して消えた `Fft::new`(twiddle 生成 + workspace
確保)分が改善として観測された。524_288(255 フレーム)・
1_048_576(511 フレーム)ともに統計的に有意な改善。

## Validation

- `mise exec -- cargo test -p al-analysis stft`: green(
  `stft_is_bit_exact_*` ×5 を含む 11 件、opt 後も unchanged で
  pass — `f64::to_bits` ビット一致)
- `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`:
  green
- `mise exec -- cargo test --workspace`: green(全 suite、
  golden 非回帰)
- `mise exec -- cargo build --workspace`: green
- `mise exec -- cargo fmt --all -- --check`: green
- `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`: green
- `mise exec -- cargo bench -p al-analysis --bench stft --
  --baseline pre`: 完走、delta は上表のとおり IR に記録
- `mise exec -- cargo tree -p al-analysis --depth 1`: 直接依存に
  al-math なし(`Fft` は `al_locate::dsp` 経由で到達)
- コード検査: `Fft::new` は `stft` ガード内・ループ外に1箇所のみ、
  `dsp::fft` 公開シグネチャ不変、フレームバッファ
  `vec![0.0; fft_size]` ×2 はループ内に残存

## Risks

- bench delta は同一マシン・同一 bench profile でのみ有効。
  criterion の `change` は `pre` baseline との統計比較で、別環境
  では絶対値・改善率ともに変わり得る
- 将来 `Fft` が `transform` で内部状態を持つ実装に変わると plan
  再利用のビット一致仮定が崩れ得る — 現行は `&self` で不変係数
  のみ参照し、`stft_is_bit_exact_*` が検出する。ロールバックは
  `dsp::fft` 呼出しへ戻す1行

## Follow-ups

- T3(A4-opt): `from_columns` + rows API 置換 + caller 移行(
  `docs/tasks/caller-opt-least-squares-column-api.md`、本タスクに
  depends-on)
- T4(A1-opt): `dsp::conv`(overlap-save)+ `fir_model` 置換
- 最終測定タスク(caller-opt-measurement-ir-closeout): 本 IR の
  bench delta を before/after 全比較値として closeout へ転記
