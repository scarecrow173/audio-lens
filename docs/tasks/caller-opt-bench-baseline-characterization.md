---
id: "TASK-034EJgkPQJmqrMLNdP7lVh"
type: "task"
status: "todo"
title: "caller 側最適化: bench 基盤導入・baseline 計測・characterization テスト先行"
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
  blocks:
    - "docs/tasks/caller-opt-stft-fft-plan-hoist.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/caller-side-optimizations.md"
    - "docs/specs/caller-side-optimizations.md"
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

# caller 側最適化: bench 基盤導入・baseline 計測・characterization テスト先行

## Work

plan T1。design の T0・A1-char・A3-char・A4-char を集約し、全ての
最適化適用に先立つ計測基盤と characterization テストを land させる
(spec プロセス契約)。コミット境界は以下の4つに分ける。

**コミット1 — ベンチ基盤(design T0)**:

- `crates/al-analysis/Cargo.toml` に criterion `=0.8.2` dev-dep と
  `[[bench]] name = "stft" harness = false`・`[[bench]] name =
  "conv" harness = false` を追加(al-math `Cargo.toml` と同
  pattern。依存追加として IR に記録)
- `crates/al-analysis/benches/stft.rs` 新設: `stft()` を決定的入力で
  計測。window/hop は現行設定(4096/2048)固定、信号長を ~0.5M・
  ~1M samples で走査。group `stft`、`black_box` 使用
- `crates/al-analysis/benches/conv.rs` 新設: `conv_direct`(直接
  畳み込み参照実装、bench ファイル内に自己完結)の baseline 腕
  のみ — `dsp::conv` は T4 まで存在しない。直接畳み込みは
  O(n·m) のため n ≤ 262_144 に有界化し `sample_size(10)` + 短い
  `measurement_time` を設定
- `crates/al-math/Cargo.toml` に `[[bench]] name = "lstsq"
  harness = false` を追加し `crates/al-math/benches/lstsq.rs`
  新設: `LeastSquares` 構築 + `solve` を列数 {1, 2, 4} × 観測数
  {65_536, 262_144, 1_048_576} で計測(**rows API 上で** —
  T3 で列シグネチャへ追従)。入力列は `b.iter` の計測対象外で
  1回生成。`vec_primitives.rs` とは別 target
- `crates/al-analysis/examples/bench_reference.rs` 新設: al-locate
  `bench_*` examples の Instant ベンチ規約に倣い、`stft`(現行
  4096/2048)・有界 synthetic `analyze_pair`(fir_model 経路、
  数秒規模の LCG ノイズ + 構成済み mix)・`prepare_residual`
  (al-locate pub、A4 前後でシグネチャ不変の同一 harness)を計測

**コミット2 — A1-char**:

- `reference.rs` の `analyze_pair` 内 `fir_model` クロージャと
  `subtract` クロージャを module 内の名前付き関数へ切り出す
  (挙動不変の純粋リファクタ。`fir_ir` は CSD 累積・`hre`/`him`
  構築・irfft までを担い畳み込みは含まない。`conv_len` キャップは
  `fir_model` 側で `&ir[..ir.len().min(4096)]`、`fr == 0` 早期
  return は `fir_ir` 内に固定 — design の切り出しシグネチャに従う)
- `subtract` の `m[i.min(m.len())]` を
  `m.get(i).copied().unwrap_or(0.0)` の zero-fill に置き換える
  (clamp 不採用。Aligned shared-length 契約 `stage.rs:283-295`
  の下で本番到達不能の構造的防御。A1 適用(T4)より先に land
  させる確定事項)
- `mod tests` に characterization テスト + test 内 `conv_direct`
  (design の逐語同一コピー。`if x == 0.0 { continue }` スキップは
  意図的に落とす):
  - `fir_model`: `fir_ir` の IR を `conv_len` に切詰めて
    `conv_direct` に通した参照と `fir_model` 出力を要素毎
    `|a-b| <= 1e-12 * max(1.0,|a|,|b|)` で照合。fixture: 空・
    `n < conv_len`・`conv_len = 4096` 境界・全ゼロ・LCG ノイズ
  - `subtract`: ragged チャネル長(model < residual)fixture で
    panic せず短い側が zero-fill されること

**コミット3 — A3-char**:

- `stft.rs` `mod tests` にビット一致 characterization: 決定的
  fixture(sin 混合 + LCG ノイズ)の `StftMatrix` を、test 内で各
  フレームを独立に window + `dsp::fft`(single-shot 経路)した
  参照と `f64::to_bits` 比較。fixture: 通常複数フレーム・
  `samples.len() < window`(0フレーム)・単一フレーム・非2冪
  window・全ゼロ。`frames`/`fft_size`/`window`/`hop` メタも一致
  を確認

**コミット4 — A4-char**:

- `mixer.rs` `mod tests` に `fit_mix` characterization: 行ベース
  normal-equations oracle(`linalg.rs` `mod legacy` と同型の test
  内自己完結参照)との照合。`MixerEstimate.matrix`・`fit_quality`
  は predicate 内一致、`condition_number` は INFINITY 整合・位数
  一致の sanity bound のみ
- `al-math` 側の `mod legacy` が oracle として残っていることを
  確認(既存 `linalg.rs:364`)。lstsq ベンチ baseline は rows API
  上で計測する

**baseline 計測(コミット1・4 完了後に実施)**:

- `cargo bench -p al-analysis -- --save-baseline <name>` で stft・
  conv_direct の criterion baseline を `target/criterion/` に保存
- `cargo bench -p al-math --bench lstsq -- --save-baseline
  lstsq-rows` で rows API baseline を保存
- `cargo run --example bench_reference --release` の出力を
  `target/bench-baseline/` 配下へファイルリダイレクトで保存
- 実測値を `docs/impl/ir/` の Implementation Record ドラフトへ
  転記(ファイル名・分割は既存 IR 規約に倣う)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/caller-side-optimizations.md`(T1)
- `docs/designs/caller-side-optimizations.md`(characterization 配置
  表・ベンチ構成・コミット順序・切り出しシグネチャ)
- `docs/specs/caller-side-optimizations.md`(プロセス契約・In Scope)
- `docs/adr/caller-side-optimizations-structure.md`(zero-fill 同梱
  順序・`conv_direct` 置き場・ベンチ依存)
- `docs/adr/simd-reduction-lane-fma.md`(等価性 predicate の権威)

## Dependencies

- 先行タスクなし。T2(及び後続の全 opt)は本タスクの
  characterization・baseline に依存する

## Affected Files

- `crates/al-analysis/Cargo.toml`(criterion dev-dep + `[[bench]]` ×2)
- `crates/al-analysis/benches/stft.rs`(新設)
- `crates/al-analysis/benches/conv.rs`(新設、`conv_direct` 腕のみ)
- `crates/al-analysis/examples/bench_reference.rs`(新設)
- `crates/al-analysis/src/reference.rs`(名前付き関数切り出し +
  zero-fill + characterization テスト + `conv_direct`)
- `crates/al-analysis/src/stft.rs`(`mod tests` にビット一致
  characterization)
- `crates/al-locate/src/mixer.rs`(`mod tests` に `fit_mix`
  characterization oracle)
- `crates/al-math/Cargo.toml`(`[[bench]] name = "lstsq"`)
- `crates/al-math/benches/lstsq.rs`(新設、rows API baseline)
- `target/bench-baseline/`(Instant ベンチ出力記録、gitignore 済み)
- `docs/impl/ir/<record>.md`(baseline 実測記録ドラフト)

## Done When

- [ ] `crates/al-analysis` の criterion dev-dep と `[[bench]]`
  stanza ×2 が al-math と同 pattern で存在し、`cargo bench -p
  al-analysis` で stft・conv_direct のベンチが走る
- [ ] `crates/al-math` の `[[bench]] name = "lstsq"` と
  `benches/lstsq.rs` が存在し rows API baseline を計測できる
- [ ] `examples/bench_reference.rs` が stft・analyze_pair・
  prepare_residual を Instant 計測し実行できる
- [ ] `fir_ir`/`fir_model`/`subtract` が名前付き関数として
  `mod tests` から到達可能で、`fir_model` characterization が
  `fir_ir`+`conv_direct` 合成参照と predicate 内一致する
- [ ] `subtract` が `m.get(i).copied().unwrap_or(0.0)` に
  置き換わり、ragged fixture で panic しないテストが pass する。
  zero-fill が A1 適用より先のコミットに land している
- [ ] stft の `to_bits` ビット一致 characterization が全エッジ
  fixture で pass する
- [ ] `fit_mix` characterization が `matrix`/`fit_quality` で
  oracle と predicate 内一致し、`condition_number` が sanity
  bound のみである
- [ ] baseline 実測が `target/criterion/`(`--save-baseline`)と
  `target/bench-baseline/` に記録され、IR ドラフトに転記済み
- [ ] 全 characterization コミットがどの最適化コミットよりも
  先に land している(git 履歴で確認可能)

## Verification

- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  が green(新規 characterization テストを含む)
- [ ] `mise exec -- cargo bench -p al-analysis -- --list` で stft・
  conv ベンチが列挙される
- [ ] `mise exec -- cargo bench -p al-math --bench lstsq` が走り
  `--save-baseline` で baseline を保存できる
- [ ] `mise exec -- cargo run --example bench_reference --release -p
  al-analysis` が完走し出力が `target/bench-baseline/` に保存済み
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れない
- [ ] 実測値が `docs/impl/ir/` のレコードに記録されている(目視)

## Out of Scope

- `dsp::conv`・`pub use Fft`・`from_columns` 等新設・置換コード
  (T2–T4)
- `benches/conv.rs` の `dsp::conv` 腕(T4 で追加)
- `linalg.rs` のテスト列 fixture 書き換え(T3 で API 置換と共に)
