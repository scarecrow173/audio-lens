---
id: "TASK-034EO484s25JM8bsb6zUCh"
type: "task"
status: "done"
title: "機械的 caller クリーンアップ: characterization テスト・bench 腕・baseline 採取"
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
    - "docs/plans/mechanical-caller-cleanup.md"
  implemented-by:
    - "docs/impl/ir/mech-cleanup-bench-baseline-characterization.md"
  depends-on:
    - "docs/plans/mechanical-caller-cleanup.md"
  blocks:
    - "docs/tasks/mech-cleanup-a6-bandpass-batching.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/mechanical-caller-cleanup.md"
    - "docs/specs/mechanical-caller-cleanup.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ: characterization テスト・bench 腕・baseline 採取

## Work

plan T1。design の char コミット(step 1)を担い、全ての
最適化適用に先立つ characterization テスト群・新規
criterion 腕・baseline 計測を land させる(spec プロセス
契約)。**最適化コードは一切含まず、既存公開 API のみで
compile する**。コミットは characterization + bench 腕の
挙動不変コミットとしてまとめ、このコミット後に baseline
を採取する。

**コミット1 — characterization + bench arm + baseline**:

- `crates/al-analysis/Cargo.toml` に `[[bench]] name =
  "band_correlation" harness = false`・`"csd_accum"`・
  `"quant_step"`・`"spectral_pass"` の stanza ×4 を追加
  (criterion `=0.8.2` dev-dep・`harness = false` pattern
  は先行チェーンで導入済み。新規依存ではない)
- `crates/al-analysis/benches/band_correlation.rs` 新設:
  A6 腕。8×`bandpass` loop vs `bandpass_many` の section
  直接計測(既存公開 API のみで両 arm を自己完結、
  `black_box` 使用、決定的入力)
- `crates/al-analysis/benches/csd_accum.rs` 新設:
  A7/A8 腕。per-frame `csd` 呼出し累積と長さ 1 `csd`
  パターン(現行形状)vs inline 積(同じ式を bench 内に
  記述)の alloc 削減を固定長 synthetic 入力で計測
- `crates/al-analysis/benches/quant_step.rs` 新設: A12 腕。
  >1M distinct を持つ決定的入力で現行 double-sort vs
  HashSet gate(bench 内に両経路を記述)を計測
- `crates/al-analysis/benches/spectral_pass.rs` 新設:
  A9/A16a/A16b 腕。frames×bins の `mag_sq` pass 数
  (`magnitude()`+`power()`+`psd()` 個別呼出し vs
  `power()` 一回 + caller 導出)・列走査 vs 行優先累積・
  `magnitude()` 行列 vs `mag_sq` scratch(dsp facade
  経由)を計測
- characterization テスト(design 配置表に従う、全て
  `to_bits`/バイト列/record 完全一致、期待値は al-testkit
  fixture 再現可能な値か test 内 oracle — magic literal
  禁止):
  - `crates/al-analysis/tests/reference.rs`: A6
    band_correlation tensor 要素 `to_bits` + `bands < 8`
    edge(低 sample rate で `.min(8)` 境界)、A8 hft
    tensor 全要素、A15 residual interleave 要素列、
    A16a `mean_psd_*`/`band_gains`/`psd_ratio` 等、
    A16b `ref.harmonic_freq_ratio`
  - `crates/al-analysis/tests/expensive.rs` +
    `src/expensive/mod.rs`/`channel.rs` tests: A7/A16a
    下流 record(`coherence`・`mean_power`・
    `total_power`/`one_sided_power` 等)`to_bits`
  - `crates/al-analysis/src/cheap.rs` `mod tests`: A7
    stereo(`stereo.coherence_mean`)、A9
    `magnitude`/`power`/`psd` 各 tensor + `spec.slope`
    record、A12 `dig.quant_step_est` value/support/
    `Validity` — in-gate(2〜1M)と >1M distinct skip の
    両経路を別 fixture で pin
  - `crates/al-analysis/tests/reference.rs` または対応
    テスト: A7 下流 record(h・coherence・band gain)
  - `crates/al-locate/tests/locate.rs`: A15 normalized
    `.f64le` バイト列完全一致
  - `crates/al-locate/src/dsp.rs` `mod tests`: A15
    `deinterleave` 返却列完全一致(trailing partial
    frame drop を含む)
  - `crates/al-media/src/video.rs` `mod tests`: A16c
    luminance/diff 系列 `to_bits`・先頭 `diff=0.0`・
    `pixels==0` degenerate 出力
  - `crates/al-dataset/src/tensor.rs` `mod tests` +
    `tests/integration.rs`: A16d `(out, region_shape)`
    完全一致(fast path・fallback・innermost 部分包含
    edge — fast path は T6 まで存在しないため、本
    コミットでは「完全包含・部分包含・非包含」の各
    region で現行 gather 経路の出力を oracle として
    捕捉する形にし、T6 で両経路比較へ拡張)
  - `crates/al-dataset/tests/integration.rs`: A16d 用の
    `Instant` micro-timing 区間(tensor round-trip を
    網羅する既存 test binary 内、新規 dev-dep なし)
- al-locate `examples/bench_dsp.rs` の確認:
  deinterleave/interleave が既存計時経路に載るかを調べ、
  載るなら流用、載らなければ A15 用の Instant 区間を
  追加する
- `examples/bench_reference.rs`(既存)が A6・A9・A16a/b
  の stage 級代表経路をカバーしていることを確認し、不足
  があれば同一 harness pattern で区間を追加する

**baseline 計測(コミット1 完了後に実施)**:

- `cargo bench -p al-analysis -- --save-baseline <name>`
  で 4 腕 + 既存 stft/conv の criterion baseline を
  `target/criterion/` に保存(baseline 名は統一して IR
  に記録)
- `cargo run --example bench_reference --release -p
  al-analysis`・`cargo run --example bench_dsp --release
  -p al-locate` の出力を `target/bench-baseline/` 配下へ
  ファイルリダイレクトで保存(Instant 系は複数回中央値)
- 実測値を `docs/impl/ir/` の Implementation Record
  ドラフトへ転記(ファイル名・分割は既存 IR 規約 —
  `caller-side-optimizations-*.md` — に倣う)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mechanical-caller-cleanup.md`(T1)
- `docs/designs/mechanical-caller-cleanup.md`
  (characterization 配置表・捕捉粒度決定・bench 構成・
  コミット順序)
- `docs/specs/mechanical-caller-cleanup.md`(プロセス契約・
  計測方式規定・In Scope)
- `docs/adr/simd-reduction-lane-fma.md`(数値不変の権威 —
  本バンドルでは tolerance predicate 不適用の確認)

## Dependencies

- 先行タスクなし。T2–T6 の全 opt は本タスクの
  characterization・baseline・bench 腕に依存する

## Affected Files

- `crates/al-analysis/Cargo.toml`(`[[bench]]` stanza ×4)
- `crates/al-analysis/benches/band_correlation.rs`(新設)
- `crates/al-analysis/benches/csd_accum.rs`(新設)
- `crates/al-analysis/benches/quant_step.rs`(新設)
- `crates/al-analysis/benches/spectral_pass.rs`(新設)
- `crates/al-analysis/tests/reference.rs`(characterization)
- `crates/al-analysis/tests/expensive.rs`(characterization)
- `crates/al-analysis/src/cheap.rs` `mod tests`(同上)
- `crates/al-analysis/src/expensive/mod.rs`/`channel.rs`
  tests(同上)
- `crates/al-locate/tests/locate.rs` +
  `src/dsp.rs` `mod tests`(同上)
- `crates/al-media/src/video.rs` `mod tests`(同上)
- `crates/al-dataset/src/tensor.rs` `mod tests` +
  `tests/integration.rs`(同上 + Instant 区間)
- `crates/al-locate/examples/bench_dsp.rs`(確認・必要時
  Instant 区間追加)
- `crates/al-analysis/examples/bench_reference.rs`(確認・
  必要時区間追加)
- `target/bench-baseline/`(Instant 出力記録、gitignore
  済み)
- `docs/impl/ir/<record>.md`(baseline 実測記録ドラフト)

## Done When

- [ ] `[[bench]]` stanza ×4 と対応する `benches/` 4
  ファイルが既存 pattern(`harness = false`)で存在し、
  `cargo bench -p al-analysis -- --list` で 4 腕が列挙
  される。各腕は既存公開 API のみで compile する
- [ ] design 配置表の全 characterization テストが存在し
  green — A6 tensor `to_bits` + `bands < 8` edge、A7
  下流 record、A8 hft、A9 mag/pow/psd/`spec.slope`、
  A12 両経路、A15 バイト列/列完全一致、A16a record、
  A16b `harmonic_freq_ratio`、A16c 系列 + degenerate、
  A16d `(out, region_shape)`
- [ ] A12 の characterization が in-gate と >1M skip の
  両経路を別 fixture で pin している
- [ ] al-dataset `tests/integration.rs` に A16d の Instant
  micro-timing 区間がある(新規 dev-dep なし)
- [ ] `bench_dsp` の A15 計測経路(流用か区間追加か)が
  決定され IR に記載済み
- [ ] baseline 実測が `target/criterion/`・
  `target/bench-baseline/` に記録され IR ドラフトに転記
  済み
- [ ] char/bench コミットがどの最適化コミットよりも先に
  land している(git 履歴で確認可能)
- [ ] 本タスクの変更に最適化コードが含まれない
  (production コードの差分なし — bench/test/example
  と docs のみ)

## Verification

- [ ] `mise exec -- cargo test -p al-analysis -p al-locate
  -p al-media -p al-dataset` が green(新規
  characterization テストを含む)
- [ ] `mise exec -- cargo bench -p al-analysis -- --list`
  で band_correlation・csd_accum・quant_step・
  spectral_pass が列挙される
- [ ] `mise exec -- cargo bench -p al-analysis --
  --save-baseline <name>` が完走し baseline を保存できる
- [ ] `mise exec -- cargo run --example bench_reference
  --release -p al-analysis` と `mise exec -- cargo run
  --example bench_dsp --release -p al-locate` が完走し
  出力が `target/bench-baseline/` に保存済み
- [ ] `mise exec -- cargo clippy --workspace --all-targets
  -- -D warnings`・`mise exec -- cargo fmt --all --
  --check` が green
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1`
  の直接依存に al-math が現れない
- [ ] 実測値が `docs/impl/ir/` のレコードに記録されて
  いる(目視)

## Out of Scope

- A6–A16 の全最適化コード(T2–T6)
- bench 腕の al-dataset/al-media 向け新規 dev-dep 導入
  (禁止。Instant micro-timing / code inspection +
  回帰ゲートで代替)
- `stft.rs`・`dsp.rs` の公開面変更(`bandpass`・`csd` は
  public のまま)
