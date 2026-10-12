---
id: "TASK-034EJgxFS2IJznrYHnuNJy"
type: "task"
status: "done"
title: "caller 側最適化 A3: stft の Fft plan 巻き上げ"
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
    - "docs/plans/caller-side-optimizations.md"
  implemented-by:
    - "docs/impl/ir/caller-side-optimizations-stft-fft-plan-hoist.md"
  depends-on:
    - "docs/plans/caller-side-optimizations.md"
    - "docs/tasks/caller-opt-bench-baseline-characterization.md"
  blocks:
    - "docs/tasks/caller-opt-least-squares-column-api.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/adr/caller-side-optimizations-structure.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/caller-side-optimizations.md"
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化 A3: stft の Fft plan 巻き上げ

## Work

plan T2(A3 opt コミット)。`stft` のフレームループが毎フレーム
`dsp::fft` 経由で `Fft::new` を構築しているのを、plan をループ外に
巻き上げて再利用する内部リファクタに改める。出力はビット一致必須
(T1 の characterization テストがゲート)。

- `crates/al-locate/src/dsp.rs`: `pub use al_math::fft::Fft;` を
  追加(薄いキャリア新設ではなく re-export — 既存 `pub use
  al_math::correlation::{...}` と同 pattern。`Fft` は変換・
  スレッド間で再利用可能な不変係数の型としてそのまま露出)
- `crates/al-locate/src/lib.rs`: dsp mirror 規約に倣い、既存
  `pub use dsp::{...}` 行に `Fft` を追加
- `crates/al-analysis/src/stft.rs`: `use al_locate::dsp::{Fft,
  mag_sq, scale};` 等に `Fft` を追加し、`samples.len() >=
  window && window > 0 && hop > 0` ガードの内側・ループ外で
  `let plan = Fft::new(fft_size);` を1回構築、フレームループは
  `plan.transform(&mut fre, &mut fim, false)` を呼ぶ
  - `Fft::new` をガード内に置く理由: `window == 0` のとき
    `next_power_of_two() == 1` でも「フレームが走る場合のみ
    plan を構築する」現行意味に揃える
  - フレームバッファ `vec![0.0; fft_size]` ×2 は `re.push(fre)` /
    `im.push(fim)` で出力 `StftMatrix` の行に move されるため
    現行のまま残す(巻き上げ対象外、バッファ自体が出力)
  - `stft` の公開シグネチャは不変 → 全 caller(`reference.rs`
    ×~6・`cheap.rs:609,940`・`expensive/mod.rs`・`codec.rs`・
    `channel.rs`・`band_gain.rs`・テスト群)無修正
- `dsp::fft` の公開シグネチャは変更しない → single-shot caller
  (`codec.rs`・`dynamics.rs`・`ir.rs`・`noise.rs`・
  `reference.rs` の irfft・`expensive/tests.rs`)無修正
- ビット一致の根拠: `Fft` の twiddle は `Fft::new` で決定的に
  生成され `transform` はバッファを全上書きする。同一カーネル・
  同一入力の再利用で出力はビット一致
- opt 適用後に stft bench を `--baseline <T1 で保存した名>` で
  再実行し、delta を Implementation Record ドラフトに追記

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/caller-side-optimizations.md`(T2)
- `docs/designs/caller-side-optimizations.md`(A3 plan キャリア・
  stft 変更コード・ビット一致根拠)
- `docs/adr/caller-side-optimizations-structure.md`(plan 所有境界:
  dsp 公開・`dsp::fft` 維持・フレームバッファ巻き上げなし)

## Dependencies

- `docs/tasks/caller-opt-bench-baseline-characterization.md`:
  stft ビット一致 characterization と stft criterion baseline が
  先に land していること
- T3 は本タスクに依存する(直列化は plan Implementation
  Sequence の決定)

## Affected Files

- `crates/al-locate/src/dsp.rs`(`pub use al_math::fft::Fft`)
- `crates/al-locate/src/lib.rs`(dsp mirror に `Fft` 追加)
- `crates/al-analysis/src/stft.rs`(`Fft` import + plan 巻き上げ)
- `docs/impl/ir/<record>.md`(stft bench delta の追記)

## Done When

- [ ] `al_locate::dsp::Fft` が re-export され crate root mirror にも
  `Fft` が現れる
- [ ] `stft` のフレームループ外(ガード内)で `Fft::new` が1回だけ
  構築され、ループ内に `Fft::new` がない(コード検査)
- [ ] `dsp::fft` の公開シグネチャが不変で single-shot caller が
  無変更
- [ ] T1 の stft ビット一致 characterization が opt 後も green
- [ ] フレームバッファ `vec![0.0; fft_size]` ×2 がループ内に
  残っている(出力行への move を維持)
- [ ] stft criterion bench の baseline 比較 delta が IR に追記済み

## Verification

- [ ] `mise exec -- cargo test -p al-analysis stft` でビット一致
  characterization を含む stft テストが pass
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が green
- [ ] `mise exec -- cargo bench -p al-analysis --bench stft --
  --baseline <baseline-name>` が実行でき delta を IR に記録
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れない

## Out of Scope

- stft フレームバッファ・出力行バッファの巻き上げ/再利用
- `dsp::fft` シグネチャ変更・single-shot caller の変更
- `cheap.rs:940` 等の stft caller 側変更(シグネチャ不変のため不要)
