---
id: "TASK-034EJgOK1cNKnai60q1zOd"
type: "task"
status: "todo"
title: "caller 側最適化 A1: dsp::conv overlap-save 化と fir_model 置換"
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
  implemented-by: []
  depends-on:
    - "docs/plans/caller-side-optimizations.md"
    - "docs/tasks/caller-opt-least-squares-column-api.md"
  blocks:
    - "docs/tasks/caller-opt-measurement-ir-closeout.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/adr/caller-side-optimizations-structure.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/caller-side-optimizations.md"
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化 A1: dsp::conv overlap-save 化と fir_model 置換

## Work

plan T4(A1 opt コミット)。`al_locate::dsp` に overlap-save 畳み込み
`conv` を新設し、T1 で名前付き関数化済みの `fir_model` 内の
直接畳み込みを置き換える。`conv_direct` は T1 で既に
`reference.rs` tests に存在し、本タスクで `dsp.rs` tests と
`benches/conv.rs` にも逐語同一(verbatim)コピーを追加する
(3箇所、共有ヘルパ化・公開面への露出はしない)。

**`crates/al-locate/src/dsp.rs`** — `pub fn conv(signal: &[f64],
kernel: &[f64]) -> Vec<f64>` 新設(design の `dsp::conv` 設計
どおり、内部で `al_math::fft::RealFft` を使う):

- 線形畳み込みの先頭 `signal.len()` 要素を返す。空入力の早期
  return はブロック長計算より必ず先に置く:
  `kernel.is_empty()` → `vec![0.0; signal.len()]`、
  `signal.is_empty()` → `Vec::new()`(`m == 0` では `m - 1` が
  usize でアンダーフローするため)
- `m = kernel.len()`、ブロック長 `B = (2 *
  m).next_power_of_two().max(1024)`(2のべき乗・`RealFft::new`
  契約。`m` ≤ 4096 で B ≤ 8192 に有界、`.max(1024)` は小
  kernel での per-block オーバーヘッドを潰す下限)。segment 長
  `L = B - m + 1`
- `RealFft::new(B)` を1本構築し H の forward・各ブロックの
  forward/inverse で使い回す(`bandpass_many` の先例)
- `h` を `B` に zero-pad → `plan.forward` で `H = (hre, him)`
  (長さ `B/2+1`)を1回だけ取得
- ブロックループ: `xbuf` 先頭へ前ブロック末尾 `m-1` を繰越し
  (`copy_within`)→ 新規 `take` 要素をコピー → 残り zero-pad
  → forward → `B/2+1` bins で複素乗算(`zre = xre·hre −
  xim·him`、`zim = xre·him + xim·hre`、dsp 内の素直なスカラ
  ループ)→ inverse(1/N 正規化は `RealFft::inverse` 内包、
  追加 scaling 不要)→ 先頭 `m-1` を discard して `take` 要素を
  `y` へ写す
- `conv` 自体に `conv_len` キャップは持たせない(純粋な畳み込み
  関数のまま — キャップは `fir_model` 側)
- `mod tests`: `conv_direct` コピー + equivalence テスト。
  `conv(x,h)` と `conv_direct` を要素毎 `|a-b| <= 1e-12 *
  max(1.0,|a|,|b|)` で照合。fixture: 空 kernel → ゼロ列・空
  signal → 空・`n < m`・`m` ∈ {1, 3, 256, 4096}・`n` ∈ {1,
  小, 非ブロック倍数, 1M級}・全ゼロ・LCG ノイズ
- `lib.rs` の dsp mirror に `conv` を追加

**`crates/al-analysis/src/reference.rs`** — `fir_model` の
畳み込みを `dsp::conv(from, &ir[..ir.len().min(4096)])` 呼出し
1箇所に置き換える(畳み込み経路の切替点を1箇所に集約)。
`zero-skip`(`if x == 0.0 { continue }`)は FFT 経路に
持ち越さない(±0.0 の積は値不変のため等価、sparse 最適化は
FFT 化で不要)。`fr == 0` のとき `fir_ir` がゼロ埋め IR を返し
`fir_model` は無条件に `dsp::conv` へ進む(ゼロ IR の畳み込み
結果は `vec![0.0; n]` で現行早期 return と等価)

**`crates/al-analysis/benches/conv.rs`** — `dsp::conv` 腕を追加:
n ∈ {65_536, 262_144, 1_048_576} × kernel ∈ {256, 4096}。
T1 で記録した `conv_direct` baseline と同一 harness で
`--baseline` 比較し、1M 点は FFT 経路の scaling 観測用。
delta を IR に追記

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/caller-side-optimizations.md`(T4)
- `docs/designs/caller-side-optimizations.md`(`dsp::conv` 設計:
  B・L・M-1 discard・正規化・疑似コード、`conv_direct` 置き場、
  `fir_model` 置換)
- `docs/adr/caller-side-optimizations-structure.md`(overlap-save
  採用・dsp 置き場・キャップ/尾部打ち切り意味の権威)
- `docs/adr/simd-reduction-lane-fma.md`(predicate 適用の権威)

## Dependencies

- `docs/tasks/caller-opt-least-squares-column-api.md`(直列化は
  plan Implementation Sequence の決定)
- 実質的には T1 の `fir_model`/`fir_ir`/`subtract` 切り出し・
  `conv_direct` 参照実装・conv/fir_model characterization・
  conv_direct baseline に依存
- T5 は本タスクに依存する

## Affected Files

- `crates/al-locate/src/dsp.rs`(`conv` 新設・test 内
  `conv_direct` + equivalence テスト)
- `crates/al-locate/src/lib.rs`(dsp mirror に `conv` 追加)
- `crates/al-analysis/src/reference.rs`(`fir_model` の畳み込みを
  `dsp::conv` へ置換)
- `crates/al-analysis/benches/conv.rs`(`dsp::conv` 腕追加)
- `docs/impl/ir/<record>.md`(conv bench delta の追記)

## Done When

- [ ] `dsp::conv` が存在し overlap-save(B 有界・M-1 discard・
  inverse 1/N 正規化・`RealFft` plan 1 本再利用・空入力早期
  return)で実装されている
- [ ] `dsp.rs` tests の equivalence テストが `conv_direct` と
  predicate 内で pass し、エッジケース(空・n<m・非倍数・
  4096・全ゼロ)を網羅する
- [ ] `conv_direct` が `reference.rs` tests・`dsp.rs` tests・
  `benches/conv.rs` の3箇所に逐語同一で存在し、共有公開面に
  露出していない
- [ ] `fir_model` が名前付き関数で直接畳み込みループを持たず、
  `dsp::conv` 呼出しが1箇所に集約されている(コード検査)
- [ ] `conv_len = ir.len().min(4096)` キャップと「出力 = 線形
  畳み込みの先頭 n 要素」の意味が維持されている
- [ ] T1 の `fir_model` characterization が `dsp::conv` 置換後も
  green
- [ ] `lib.rs` mirror に `conv` が現れ、`mise exec -- cargo tree
  -p al-analysis --depth 1` に al-math 直接依存がない
- [ ] `benches/conv.rs` に `dsp::conv` 腕があり baseline 比較
  delta が IR に追記済み

## Verification

- [ ] `mise exec -- cargo test -p al-locate conv` で dsp
  equivalence テストが pass
- [ ] `mise exec -- cargo test -p al-analysis reference` で
  `fir_model`/`subtract` characterization が opt 後も pass
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p
  al-analysis` が green
- [ ] `mise exec -- cargo bench -p al-analysis --bench conv --
  --baseline <baseline-name>` が実行でき delta を IR に記録
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `rg "y\[i \+ k\] \+=|for \(k, &hv\)" crates/al-analysis/src/
  reference.rs` で直接畳み込みループが本番コードに残っていない
  ことを検査

## Out of Scope

- `fir_ir` の STFT・irfft 内部の変更
- conv 内スペクトル複素乗算の SIMD 化(別チェーン、スカラ
  ループで十分 — design Deferred)
- `conv` への `conv_len` キャップ内蔵(fir_model 側の責務)
