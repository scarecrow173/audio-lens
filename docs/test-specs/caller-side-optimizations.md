---
id: "TSPEC-034EJgd4MF3zQZ1FN9K4WV"
type: "test-spec"
status: "approved"
title: "caller 側最適化チェーン検証仕様"
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
    - "docs/designs/caller-side-optimizations.md"
  derived-by: []
  verifies:
    - "docs/specs/caller-side-optimizations.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化チェーン検証仕様

## Purpose

`docs/specs/caller-side-optimizations.md` の Acceptance Criteria を
検証可能な形に分解した検証仕様。caller 側最適化チェーン(A1
`fir_model` 畳み込み FFT 化・A3 STFT plan 再利用・A4 LeastSquares
列指向)が「characterization テスト先行 + bench baseline/事後計測 +
Implementation Record 記録」のプロセス契約の下で、数値等価
(A3 ビット一致 / A1・A4 は recomposition tolerance 述語)と公開
契約(`dsp::fft`・`stft`・`subtract_fit`/`prepare_residual` の
シグネチャ不変、dsp ファサード一元化)を維持しながら適用された
ことを保証する。等価性 predicate の権威は
`docs/adr/simd-reduction-lane-fma.md`(ADR-034EoHxzsvsHp9BOdCK2Cn)、
API・構造面の権威は `docs/adr/caller-side-optimizations-structure.md`
(ADR-034EHHl9H4TTHHr0OQEa39)とし、本文の再掲と乖離した場合は
ADR を正とする。

各 characterization テストは対応する最適化コミットの前後両時点で
green であることを意図し、最適化適用後も継続して残る。廃止して
よいのは、該当する最適化が revert され、かつ characterization が
捕捉していた現行意味そのものが仕様変更により消滅した場合のみ。

## Feature

al-analysis / al-locate の caller 側3ホットスポットの最適化が、
変更前後で捕捉済みの振る舞い契約を守ること — 畳み込み・
STFT・最小二乗の出力等価性、公開シグネチャと依存境界の不変、
計測基盤の存在と baseline/事後比較の記録。

## Rules

### Rule A: characterization-first のコミット順序

- Rule: 各最適化の適用コミットに先行して、(1) 現行出力を捕捉する
  characterization テストが存在し pass すること、(2) 該当する
  criterion / Instant ベンチが変更前に baseline 計測されていること。
  この順序は git 履歴(char コミットが対応する opt コミットより
  先に land していること)で確認可能でなければならない
- Rule: `subtract` の zero-fill 修正(`m[i.min(m.len())]` →
  `m.get(i).copied().unwrap_or(0.0)`)は characterization
  コミットに同梱し、A1 最適化の適用(等価性ゲート実行)より先に
  land させる — 変更順序を記録上明確にするための確定事項
- Rule: `fir_ir`/`fir_model`/`subtract` の名前付き関数切り出しは
  挙動不変の純粋リファクタとして characterization コミットに
  含まれ、characterization は関数レベルで行う(畳み込み切り出し
  と同じ方針)

### Rule B: A3 — STFT 出力のビット一致

- Rule: 変更前後で `StftMatrix` の `re`/`im` 全要素と `frames`/
  `fft_size`/`window`/`hop` メタが `f64::to_bits` 比較で一致する
  こと。参照は test 内で各フレームを独立に window + `dsp::fft`
  (single-shot 経路)して得る
- Rule: fixture は決定的(シード固定パターンまたは synthetic
  fixture)で、通常複数フレーム・`samples.len() < window`(0
  フレーム経路)・単一フレーム(`window <= n < window + hop`
  または `hop >= n`)・非2冪 window・全ゼロを含む
- Rule: コード検査の不変条件として「stft ループ内に `Fft::new`
  がない」「`dsp::fft` の公開シグネチャが不変」が保持される。
  フレームバッファ(`vec![0.0; fft_size]` ×2)は出力行へ move
  されるため巻き上げ対象外

### Rule C: A1 — 畳み込みと fir_model の predicate 等価

- Rule: `dsp::conv` の出力は、test/bench 内に自己完結で保持する
  直接畳み込み参照実装 `conv_direct` と、固定 fixture 上で要素毎
  `|a - b| <= 1e-12 * max(1.0, |a|, |b|)` を満たす。`conv_direct`
  は `reference.rs` tests・`dsp.rs` tests・`benches/conv.rs` の
  3箇所に逐語同一(verbatim)のコピーとして保持し、共有公開面に
  出さない
- Rule: conv のエッジケースは、空 kernel → `signal.len()` 長の
  ゼロ列、空 signal → 空ベクトル、`n < m`、kernel = 1、
  `m` ∈ {1, 3, 256, 4096}、`n` ∈ {1, 小, 非ブロック倍数, ~1M}、
  全ゼロ、決定的ノイズ(LCG)を含む
- Rule: `fir_model` characterization は、切り出した `fir_ir` の IR
  を `conv_len = ir.len().min(4096)` に切詰めて `conv_direct` に
  通した参照と `fir_model` 出力を predicate 照合する。エッジ:
  空・`n < conv_len`・`conv_len = 4096` 境界・全ゼロ・代表
  ノイズ。切り出し直後・`dsp::conv` 置換後の両時点で同一テストが
  green であること
- Rule: `subtract` zero-fill は ragged なチャネル長(model が
  residual より短い)fixture で panic せず短い側が zero-fill される
  ことで検証する。本番経路は Aligned shared-length 契約
  (`stage.rs:283-295`)により到達不能の構造的防御であり、挙動
  変更ではない — clamp(`m.len() - 1` 上限)は採用しない

### Rule D: A4 — LeastSquares 列指向と fit_mix の predicate 等価

- Rule: `LeastSquares::from_columns(&[&[f64]], λ)` は `new` 相当
  契約を列向きに持つ: 列が1本以上・全列等長・全要素有限・
  観測数(列長) ≥ 列数・λ が非負有限、のいずれかを欠けば
  `None`。`solve(rhs)` は `rhs.len() == 観測数`(ridge
  augmentation 後の列長ではない)を要求し、違反で `None`
- Rule: rows API(`LeastSquares::new`・`least_squares` の行
  シグネチャ)は置換であり両建てしない(pre-release 方針)。
  `mod legacy`(行ベース参照)は oracle として残し、テスト内で
  列→行転置して対比する
- Rule: `fit_mix` characterization は `MixerEstimate.matrix` と
  `fit_quality` が test 内の行ベース normal-equations oracle と
  predicate 内一致。`condition_number` は述語対象外とし、
  INFINITY 整合(非有限の一致)と位数一致の粗い sanity bound の
  みを検証する
- Rule: `Apply` の残差 RSS は ridge 適用前の観測列
  (`obs_columns` 1 コピー)上の `sub_scaled`/`sum_sq` で計算し、
  ridge 項を含めない現行契約を維持する。QR 分解の尾部から RSS を
  取る実装は禁止

### Rule E: ベンチ基盤と回帰 bounds

- Rule: `crates/al-analysis` に criterion `=0.8.2` dev-dep と
  `[[bench]] name = "stft" / "conv" harness = false` stanza が
  あり、`benches/stft.rs`・`benches/conv.rs` が存在する。
  `benches/conv.rs` は `dsp::conv` 導入前は `conv_direct`
  baseline 腕のみ(direct conv は n ≤ 262_144 に有界化、
  `sample_size(10)` + 短い `measurement_time`)、`dsp::conv` 腕は
  最適化コミットで追加される。`crates/al-math` に
  `benches/lstsq.rs` + stanza があり rows API baseline を計測
  できる
- Rule: `crates/al-analysis/examples/bench_reference.rs` が
  `stft`・有界 synthetic `analyze_pair`(fir_model 経路)・
  `prepare_residual`(fit_mix 内部経由)を Instant 計測し、A4
  前後で同一 harness となる(`prepare_residual` の公開シグネチャ
  `&[Vec<f64>]` は不変)
- Rule: 各最適化について criterion delta(`--save-baseline`/
  `--baseline` 管理)と Instant ベンチ・manifest `duration_ms`
  の before/after 実測が Implementation Record(`docs/impl/ir/`)
  に記録され、ノイズ幅(目安 ±5%)を超える回帰がないこと

### Rule F: 公開契約・依存境界の不変

- Rule: `dsp::fft`・`stft`・`subtract_fit`/`prepare_residual` の
  公開シグネチャは不変。`fit_mix`・`least_squares` は列
  シグネチャへの置換(互換層なし)
- Rule: al-analysis に al-math 直接依存を追加しない —
  `cargo tree -p al-analysis --depth 1`(または `Cargo.toml`
  直接確認)で判定する。`dsp::conv`・`dsp::Fft` は dsp
  ファサード + `lib.rs` mirror 経由で到達可能
- Rule: golden テスト(`al-analysis/tests/golden.rs`・
  `al-locate/tests/golden.rs` 等)が pass を維持する。永続化値
  (`ResidualEnergy`・`MixerEstimate`・feature record・gain
  params)は tolerance 内で変動し得るが判定意味・閾値は変更
  しない

## Examples

- Example(commit 順序): `git log` で A1-char コミット
  (`fir_model`/`fir_ir`/`subtract` 切り出し + zero-fill + テスト)
  が A1-opt コミット(`dsp::conv` + 置換)より先にある。stft ・
  mixer の char テストも対応する opt コミットに先行する
- Example(stft ビット一致): 決定的入力(sin 混合 + LCG ノイズ、
  window 4096 / hop 2048)で `stft` を実行し、各フレームを test
  内で `vec![0.0; fft_size]` に window 適用 + `dsp::fft` した
  参照と `re`/`im` を `to_bits` 比較して全要素一致。
  `samples.len() < window` では 0 フレームの `StftMatrix` が
  返り、全ゼロ入力では全要素 0.0 のビット一致
- Example(conv edge): `conv(&x, &[])` → `vec![0.0; x.len()]`、
  `conv(&[], &h)` → `vec![]`、`conv(&x, &[1.0])` → `x` に
  等しい(predicate 内)、`x.len() < h.len()` でも先頭 n 要素の
  線形畳み込みを返す
- Example(subtract zero-fill): `subtract(&[vec![1.0; 4]], &[vec![0.5; 2]])`
  のような model 側が短い ragged fixture で panic せず、後半
  要素が `residual - 0.0` として残る。本番は長さ n 共有のため
  到達不能
- Example(from_columns 契約): `from_columns(&[], 0.0)` → `None`、
  列長が観測数 < 列数 → `None`、非有限要素・負 λ・非有限 λ →
  `None`、等長でない列 → `None`。`solve(&rhs)` で
  `rhs.len() != 観測数` → `None`
- Example(fit_mix oracle): `linalg.rs` `mod legacy` と同型の
  test 内行ベース normal-equations 参照で得た解・`fit_quality`
  と `fit_mix` 出力が predicate 内一致。`condition_number` は
  ill-conditioned fixture でも INFINITY 整合と位数一致のみ要求
- Example(bench): `cargo bench -p al-analysis -- --save-baseline
  pre` で baseline 保存、opt 後に `--baseline pre` で比較。
  `cargo run --example bench_reference --release > target/
  bench-baseline/reference.txt` で Instant 計測を記録

## Guarantees

- 各最適化が「捕捉済みの現行出力を壊さない」ことを、テストの
  存在と git 履歴の両面で検証可能な形で保証する
- A3 の plan 再利用が出力のビット一致を維持し、将来 `Fft` が
  内部状態を持つ変更を導入しても本テストが検出すること
- A1 の overlap-save が `conv_direct` と predicate 等価であり、
  境界条件(空・短い入力・非倍数・4096 境界)で退行しないこと
- A4 の列置換が `new` 相当の `Option` 契約・RSS 非 ridge 意味・
  `fit_mix` 出力を oracle に対して維持し、行・列の二重表現を
  残さないこと
- 計測基盤(bench harness・baseline 手続き・IR 記録)が再現
  可能で、回帰判定(±5%)の根拠が残ること
- 公開シグネチャ・dsp ファサード消費経路・golden 判定意味が
  変わらないこと

## Non-goals

- `condition_number` の述語的等価(64 回 power iteration 推定の
  感度が κ に対しほぼ二次的で、正当な実装でも述語を超え得る —
  sanity bound のみ)
- stft フレームバッファの巻き上げ・出力行バッファ再利用
  (バッファは出力そのもの)
- A2 `ncc` plan 再利用・A5 `local_lag`→`ncc` 置換(issue #42
  系別チェーン)、al-math カーネル内部の等価性
- criterion ベンチの CI 回帰ゲート(CI 基盤未整備、整備時に
  baseline 比較を再検討)
- 永続化値(`ResidualEnergy`・`MixerEstimate`・feature record・
  gain params)のビット一致 — tolerance 内の変動は許容し golden
  pass を要求
- conv 内スペクトル複素乗算の SIMD 化・マルチスレッド化

## Risk

- この保証を失うと、最適化の「等価」主張が検証不能になり、
  数値ドリフトが `ResidualEnergy`・`MixerEstimate` 等の永続化値・
  golden 判定に混入する。characterization-first 順序の喪失は
  「変更前出力の捕捉」という検証軸そのものを失わせ、zero-fill
  防御の欠落は契約違反入力での panic に戻る。ベンチ baseline
  の欠落は回帰判定を印象論に退化させ、参照実装 `conv_direct`
  の喪失は overlap-save の正しさを再検証する術を失わせる
