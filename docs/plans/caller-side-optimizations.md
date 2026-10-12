---
id: "PLAN-034EJg701ux76Ga7UWPP0j"
type: "plan"
status: "completed"
title: "caller 側最適化チェーン: 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4)"
created: "2026-10-12"
updated: "2026-10-13"
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
    - "docs/specs/caller-side-optimizations.md"
    - "docs/adr/caller-side-optimizations-structure.md"
    - "docs/adr/simd-reduction-lane-fma.md"
  implemented-by:
    - "docs/tasks/caller-opt-bench-baseline-characterization.md"
    - "docs/tasks/caller-opt-stft-fft-plan-hoist.md"
    - "docs/tasks/caller-opt-least-squares-column-api.md"
    - "docs/tasks/caller-opt-dsp-conv-overlap-save.md"
    - "docs/tasks/caller-opt-measurement-ir-closeout.md"
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
  verifies: []
  verified-by:
    - "docs/test-specs/caller-side-optimizations.md"
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化チェーン: 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4) Implementation Plan

## Goal

approved design `docs/designs/caller-side-optimizations.md`
(DESIGN-034EIsK2SiNLNZYXmumzOe) に基づき、al-analysis / al-locate の
caller コードにある3つのホットスポットを、al-math カーネル
(`Fft`/`RealFft`/vec プリミティブ/linalg の QR 数学)を変更せずに
最適化する。A3(stft の `Fft` plan 巻き上げ、出力ビット一致)、
A4(`LeastSquares::from_columns` 追加と rows API 置換・`fit_mix` 列
指向化・全 caller 移行)、A1(`dsp::conv` overlap-save 新設と
`fir_model` 置換)を、spec のプロセス契約(characterization テスト
先行 + bench baseline/事後計測 + Implementation Record への実測記録)
の下で直列に land させる。

## Scope

単一の実装ストリームとして扱い、T1–T5 の5タスクに分割する。3施策は
spec 上それぞれ独立に適用可能だが、共通の計測基盤(criterion dev-dep・
`benches/`・`examples/bench_reference.rs`)と全サイトの
characterization テストを T1 に集約し、opt は A3→A4→A1 の順で適用
する。design の推奨コミット順序は A1→A3→A4 だが、spec が各施策を
「独立に適用可能な単位」と定めるため順序の入れ替えは許容される —
本 plan は小さい内部リファクタ(A3)から大きい API 置換・新設関数
(A4→A1)へ積み上げる順を採用する。opt 同士を直列にするのは、
各 opt の bench delta を単一の進行する baseline に対して計測し、
等価性ゲートの比較軸を一意に保つため。

- In scope: `crates/al-analysis` の criterion `=0.8.2` dev-dep +
  `[[bench]]` stanza ×2 と `benches/stft.rs`・`benches/conv.rs` 新設、
  `crates/al-math` の `benches/lstsq.rs` + stanza 新設(既存
  criterion dev-dep を利用)、`examples/bench_reference.rs` 新設、
  `reference.rs` の `fir_ir`/`fir_model`/`subtract` 名前付き関数
  切り出しと `subtract` zero-fill 修正、stft ビット一致・fit_mix
  oracle・conv/fir_model predicate characterization テスト群、
  dsp `Fft` re-export + stft plan 巻き上げ(A3)、
  `LeastSquares::from_columns` + rows API 置換 + caller 移行(A4)、
  `dsp::conv` overlap-save + `fir_model` 置換 + conv bench 腕(A1)、
  baseline/事後 bench 実測の Implementation Record 記録
- Out of scope(spec/design の Out of Scope・Deferred を継承):
  stft フレームバッファの巻き上げ・再利用、A2 `ncc` plan 再利用・
  A5 `local_lag`→`ncc`(issue #42 系別チェーン)、al-math カーネル
  内部の変更、マルチスレッド化、al-analysis への al-math 直接依存、
  golden 判定意味・Validity 閾値の変更、互換層・deprecation 期間
  (pre-release 方針により置換)、criterion ベンチの CI 回帰ゲート

## Upstream Documents

- `docs/specs/caller-side-optimizations.md`(approved,
  SPEC-034EHH6AJBK8GJ2H9EUzjE): 受入基準 — A1/A3/A4 のスコープ、
  プロセス契約(characterization 先行・bench 前後・IR 記録)、
  数値等価(A3 ビット一致 / A1・A4 は recomposition tolerance
  述語)、API 契約(`from_columns`・`dsp::fft` 不変・置換方針)、
  依存制約
- `docs/adr/caller-side-optimizations-structure.md`(accepted,
  ADR-034EHHl9H4TTHHr0OQEa39): 列指向 `LeastSquares` API 形・
  rows API 置換・RSS 非 ridge 契約、overlap-save 採用と dsp 置き場、
  STFT plan 所有境界(dsp 公開)、切り出し・参照実装の置き場、
  zero-fill のコミット順序の権威
- `docs/adr/simd-reduction-lane-fma.md`(accepted,
  ADR-034EoHxzsvsHp9BOdCK2Cn): A1/A4 の等価性 predicate
  `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`(要素毎適用)の権威
- `docs/discovery/caller-side-optimizations.md`
  (DISC-034EHtcL712sw3rAvtFX5g): ホットスポット棚卸し・数値契約・
  計測手続きの確定根拠(file:line)

## Design Inputs

- `docs/designs/caller-side-optimizations.md`(approved): 本計画の
  正本。characterization 配置表・ベンチ構成(腕・fixture 有界化・
  baseline 手続き)・コミット順序(T0/char/opt 境界)、A3 の
  `Fft` re-export + stft 変更コード、A4 の `LeastSquares` 内部表現
  (列正本・`obs_columns` 1 コピー・RSS `sub_scaled`/`sum_sq`)・
  `fit_mix` シグネチャ・caller 移行マップ・`subtract_fit` 最小化、
  A1 の切り出しシグネチャ(`fir_ir`/`fir_model`/`subtract`)・
  `dsp::conv` overlap-save 設計(B・L・M-1 discard・正規化)・
  `conv_direct` 参照実装、ファイルマップ、テスト計画、リスク表

## File Map

| Path | Responsibility | Task |
|------|----------------|------|
| `crates/al-analysis/Cargo.toml` | criterion `=0.8.2` dev-dep + `[[bench]] name = "stft" / "conv" harness = false` stanza(al-math `Cargo.toml` と同 pattern) | T1 |
| `crates/al-analysis/benches/stft.rs`(新設) | `stft()` 決定的入力 criterion ベンチ(window/hop 現行 4096/2048 固定、信号長 ~0.5M/~1M) | T1 |
| `crates/al-analysis/benches/conv.rs`(新設) | T1 は `conv_direct` baseline 腕のみ(直接畳み込みを bench 内に自己完結で保持、n ≤ 262_144 に有界化 + `sample_size(10)` + 短い `measurement_time`)。`dsp::conv` 腕は T4 で追加(FFT 経路 n ∈ {65_536, 262_144, 1_048_576} × kernel ∈ {256, 4096}) | T1, T4 |
| `crates/al-math/Cargo.toml` | `[[bench]] name = "lstsq" harness = false` stanza 追加 | T1 |
| `crates/al-math/benches/lstsq.rs`(新設) | `LeastSquares` 構築 + `solve`(列数 {1,2,4} × 観測数 {65_536, 262_144, 1_048_576})。T1 は rows API baseline、T3 で列シグネチャへ追従 | T1, T3 |
| `crates/al-analysis/examples/bench_reference.rs`(新設) | stage 級 Instant ベンチ(al-locate `bench_*` 規約): `stft`・有界 synthetic `analyze_pair`(fir_model 経路)・`prepare_residual`(fit_mix 内部経由、公開シグネチャ不変のため A4 前後で同一 harness) | T1 |
| `crates/al-analysis/src/reference.rs` | `fir_ir`/`fir_model`/`subtract` の名前付き関数切り出し(挙動不変)+ `subtract` zero-fill(`m.get(i).copied().unwrap_or(0.0)`)+ characterization テスト + test 内 `conv_direct`(T1)。`fit_mix` 呼出し×3 の列移行 + `.to_vec()` 除去(T3)。`fir_model` の畳み込みを `dsp::conv` へ置換(T4) | T1, T3, T4 |
| `crates/al-analysis/src/stft.rs` | `mod tests` にビット一致 characterization(test 内参照 = フレーム毎 window + `dsp::fft`)(T1)。`dsp::Fft` import + plan 巻き上げ(T2) | T1, T2 |
| `crates/al-locate/src/dsp.rs` | `pub use al_math::fft::Fft`(T2)。`:114` `least_squares` テストの列 fixture 化(T3)。`conv` 追加 + test 内 `conv_direct` + equivalence テスト(T4) | T2, T3, T4 |
| `crates/al-locate/src/lib.rs` | dsp mirror に `Fft`(T2)・`conv`(T4)を追加 | T2, T4 |
| `crates/al-locate/src/mixer.rs` | `mod tests` に `fit_mix` characterization(行ベース oracle との predicate 照合、`condition_number` は sanity bound)(T1)。`fit_mix` 列シグネチャ化 + 行マテリアライズ除去 + AᵀA 列 `dot` 化(T3) | T1, T3 |
| `crates/al-locate/src/residual.rs` | `subtract_fit` 内部の列参照変換(公開シグネチャ `&[Vec<f64>]` 不変、短い入力のみ zero-pad 所有バッファ) | T3 |
| `crates/al-locate/src/stage.rs` | `:753-759` `cap_window`/src のスライス参照化 | T3 |
| `crates/al-math/src/linalg.rs` | `from_columns` 追加・`new` 除去・`least_squares` 列シグネチャ・`obs_columns` 1 コピー・RSS を `sub_scaled`/`sum_sq` 化・テスト列 fixture 化(`mod legacy` は oracle として残す) | T3 |
| `docs/impl/ir/<record>.md` | baseline・各 opt 後の before/after 実測記録 | T1–T5 |

変更しない(design file map の「変更しない」欄を踏襲):
`codec.rs`・`dynamics.rs`・`ir.rs`・`noise.rs`・`expensive/tests.rs`
(`dsp::fft` single-shot)、`cheap.rs:940`(stft caller、シグネチャ
不変)、`lib.rs` re-export 面の `least_squares` 行(名前は同じ)、
`tests/locate.rs:300`・`residual.rs` テスト `:269,:306`
(`&[Vec<f64>]` 維持)、al-math の `Fft`/`RealFft`/vec
プリミティブ/QR 数学。

## Implementation Sequence

| Task | 内容 | Depends on | Blocks |
|------|------|------------|--------|
| T1 | bench 基盤(criterion dev-dep + stanza + 3 ベンチファイル + `bench_reference.rs`)+ baseline 計測・記録 + 全 characterization(A1: 切り出し + zero-fill + `conv_direct` predicate、A3: ビット一致、A4: `fit_mix` oracle)+ linalg 側の `mod legacy` oracle 確認 | — | T2 |
| T2 | A3 opt: `dsp.rs` に `pub use al_math::fft::Fft` + `lib.rs` mirror、`stft` の plan 巻き上げ(ガード内・ループ外)。ビット一致ゲート + stft bench 再計測 | T1 | T3 |
| T3 | A4 opt: `LeastSquares::from_columns` + `new`/行 `least_squares` 除去・列置換、`fit_mix` 列シグネチャ + AᵀA 列 `dot`、全 caller 移行(`reference.rs` ×3・`residual.rs`・`stage.rs`・`dsp.rs:114`・linalg/mixer テスト)、`.to_vec()` 除去、`subtract_fit` 内部列参照化、RSS `sub_scaled`/`sum_sq`。predicate ゲート + lstsq bench 再計測 + bench ファイル列追従 | T2 | T4 |
| T4 | A1 opt: `dsp::conv`(overlap-save)新設 + dsp 内 equivalence テスト + `lib.rs` mirror、`fir_model` の畳み込みを `dsp::conv` へ置換、`benches/conv.rs` に `dsp::conv` 腕追加。predicate ゲート + conv bench 再計測 | T3 | T5 |
| T5 | 事後計測・closeout: 全 bench(criterion + `bench_reference`)再実行、baseline 比較(±5%)、Implementation Record へ before/after 記録、最終 workspace 検証 | T4 | — |

T1 内のコミット境界(design のコミット順序を T1 に集約した形):
1. **ベンチ基盤コミット**(design T0): `Cargo.toml` dev-dep + stanza、
   `benches/stft.rs`・`benches/conv.rs`(`conv_direct` baseline 腕
   のみ — `dsp::conv` は T4 まで存在しない)・
   `al-math/benches/lstsq.rs`、`examples/bench_reference.rs`。
   baseline を実行し `target/criterion/`(`--save-baseline`)と
   `target/bench-baseline/`(Instant 出力リダイレクト)に記録、
   Implementation Record(ドラフト)へ実測値を転記
2. **A1-char コミット**: `fir_ir`/`fir_model`/`subtract` の名前付き
   関数切り出し(挙動不変の純粋リファクタ)+ `subtract` zero-fill
   修正 + characterization テスト + test 内 `conv_direct` 参照実装。
   zero-fill は A1 適用(T4)より先に land(spec 確定事項 —
   Aligned shared-length 契約の下で本番到達不能の構造的防御)
3. **A3-char コミット**: stft ビット一致 characterization テスト
4. **A4-char コミット**: mixer `fit_mix` characterization +
   lstsq ベンチ baseline 計測(rows API 上で — T3 で列シグネチャへ
   追従する bench ファイルの before 値)

design の推奨順序(T0 → A1-char → A1-opt → A3-char → A3-opt →
A4-char → A4-opt)に対する本 plan の差異は (a) char コミットの T1
集約、(b) opt 順を A3→A4→A1 へ入替、の2点のみ。プロセス契約
(「各最適化の着地前に characterization + baseline が存在し pass
する」)はどちらの順でも充足される — git 履歴で確認可能。

## Dependencies and Risks

依存: `crates/al-analysis` への `criterion = "=0.8.2"` dev-dep 追加
(al-math と同 version・pattern、新規依存カテゴリではないが依存追加
として IR に記録)。`crates/al-math` の criterion dev-dep は導入済み
で新規追加なし。ランタイム依存の追加なし。al-analysis に al-math
直接依存を追加しない(dsp ファサード一元化を維持)。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | overlap-save の境界条件バグ(edge ブロック・`n < m`・非倍数) | med | med | `conv_direct` との fixture 行列(空・n<m・m∈{1,3,256,4096}・n∈{1,小,非倍数,1M級}・全ゼロ・LCG ノイズ)で A/B 担保。問題時は `fir_model` を `conv_direct` 呼出しに戻すだけでロールバック可(畳み込み呼出しは1箇所) |
| 2 | A3 plan 巻き上げで twiddle 再利用の仮定が崩れる(将来 `Fft` が内部状態を持つ場合) | low | low | `Fft::transform` は `&self` で現行は不変係数のみ参照 — ビット一致テストが検出する。ロールバックは `dsp::fft` 呼出しに戻す1行 |
| 3 | A4 置換の diff が大きい(linalg テスト全書換 + caller 5 箇所) | med | med | 列 fixture への機械的置換 + `mod legacy` oracle 維持で数学的正しさを担保。A4-opt コミットの revert でロールバック(独立コミット境界) |
| 4 | `condition_number` が AᵀA 集計順変更で閾値超過的に揺れる | med | low | 述語対象外として sanity bound(INFINITY 整合・位数一致)に限定 — spec/ADR 確定済み |
| 5 | bench baseline の手順ブレ(ノイズによる誤判定) | med | low | 同一条件で `--save-baseline`/`--baseline` を使い ±5% ノイズ幅を超える回帰のみ問題視。Instant ベンチは複数回中央値 |
| 6 | `subtract` zero-fill が実は到達する経路を隠す | low | low | Aligned shared-length 契約(`stage.rs:283-295`)を根拠に記録済み。ragged fixture テストが防御を明示 |
| 7 | char を T1 に集約したことで opt 適用時に対象コードが既に変わり char fixture が陳腐化 | low | med | 切り出しは挙動不変の純粋リファクタで、T2–T4 の opt は char 対象の公開シグネチャを変えない(`stft`・`fir_model`・`subtract_fit` 不変、`fit_mix` は char が oracle 参照でシグネチャ追従のみ)。T1 完了時に全 char が green であることを Done When に含める |

### Rollback Strategy

- 対象: 全タスクがコード・文書の追加・置換で不可逆なデータ変換は
  ない。A4 の rows API 置換は公開 API 変更だが pre-release 方針の下
  互換層を設けず、revert で復帰する
- Trigger: 各 opt の等価性ゲート失敗・golden 破壊・bench ±5% 超回帰
  で原因切り分けに時間を要する場合
- Procedure: 当該 opt コミットを `git revert` する。opt コミットは
  それぞれ独立(A3 は dsp `Fft` re-export + stft、A4 は linalg +
  mixer + caller、A1 は dsp `conv` + fir_model)。T1 の char・bench
  基盤は revert 後も残してよい(現行実装を捕捉するテストとして
  green を維持する — char は opt 前後両時点で pass する契約)
- Verification: revert 後に `mise exec -- cargo test --workspace` が
  green であること

## Task Breakdown

### T1: bench 基盤・baseline 計測・全 characterization — `docs/tasks/caller-opt-bench-baseline-characterization.md`

design の T0 + A1-char + A3-char + A4-char を集約。ベンチ基盤
(criterion dev-dep・stanza・`benches/stft.rs`・`benches/conv.rs`
conv_direct 腕・`al-math/benches/lstsq.rs`・
`examples/bench_reference.rs`)、`fir_ir`/`fir_model`/`subtract`
切り出し + `subtract` zero-fill + `conv_direct` 参照実装、stft
ビット一致・fir_model/conv・`fit_mix` oracle 等の characterization
テスト、baseline 計測(`--save-baseline` + `target/bench-baseline/`)
と IR ドラフト記録。

### T2: A3 stft plan 巻き上げ — `docs/tasks/caller-opt-stft-fft-plan-hoist.md`

`dsp.rs` に `pub use al_math::fft::Fft` + `lib.rs` mirror、`stft`
ループ外への plan 巻き上げ(ガード内、バッファ巻き上げなし)、
`dsp::fft` シグネチャ・single-shot caller 不変。ビット一致
characterization の green + stft bench 再計測。

### T3: A4 LeastSquares 列指向 — `docs/tasks/caller-opt-least-squares-column-api.md`

`from_columns` 追加と rows API 置換(`new` 除去・`least_squares` 列
シグネチャ)、`obs_columns` 1 コピー + RSS `sub_scaled`/`sum_sq`
(ridge 項非含有の現行契約維持)、`fit_mix` 列シグネチャ + AᵀA 列
`dot` 化、全 caller 移行(`reference.rs` ×3・`residual.rs`・
`stage.rs`・`dsp.rs:114`・linalg/mixer テスト)、`.to_vec()` 除去、
lstsq bench の列追従 + baseline 比較。

### T4: A1 dsp::conv + fir_model 置換 — `docs/tasks/caller-opt-dsp-conv-overlap-save.md`

`dsp::conv`(overlap-save、`RealFft` plan 1 本再利用・B 有界・
M-1 discard・inverse 1/N 正規化・空入力早期 return)新設、dsp
内 `conv_direct` equivalence テスト、`lib.rs` mirror、`fir_model`
畳み込みの `dsp::conv` 置換(`conv_len` キャップは fir_model 側)、
`benches/conv.rs` への `dsp::conv` 腕追加と baseline 比較。

### T5: 事後計測・IR 記録・最終検証 — `docs/tasks/caller-opt-measurement-ir-closeout.md`

全 criterion bench(stft・conv・lstsq)の `--baseline` 比較、
`bench_reference` Instant ベンチの `target/bench-baseline/` 比較、
±5% 回帰判定、Implementation Record への before/after 記録、
workspace 全体の最終検証(test・clippy・fmt・golden・`cargo tree`)。

## Verification

| spec AC(要旨) | 検証方法 | 担当 |
|---|---|---|
| 各最適化で characterization テストが変更前に存在し green、opt 前後で green | `cargo test` + git 履歴(char コミットが opt コミットに先行) | T1 + 各 opt |
| A3: 決定的 fixture で `StftMatrix` re/im・メタがビット一致、エッジケース網羅、ループ内 `Fft::new` なし | `cargo test -p al-analysis stft` + コード検査 | T1, T2 |
| A1: 新畳み込みと `conv_direct` が predicate 内、名前付き `fir_model` 単位でも pass、エッジケース網羅 | `cargo test -p al-locate conv`・`cargo test -p al-analysis reference` | T1, T4 |
| A4: `from_columns` が同 fixture で rows 相当解・残差 RMS を predicate 内、`fit_mix` の `matrix`/`fit_quality` が oracle と一致、`condition_number` は述語外 sanity | `cargo test -p al-math`・`cargo test -p al-locate mixer` | T1, T3 |
| `subtract` zero-fill が ragged fixture で panic しない | `cargo test -p al-analysis reference` | T1 |
| criterion bench delta と stage 級代表経路の before/after が IR に記録、±5% 超回帰なし | `--save-baseline`/`--baseline` 比較 + `bench_reference` + IR 記録 | T1, T2–T4, T5 |
| `from_columns` が公開 API・行マテリアライズ/行 clone/strided gather 除去 | コード検査 + `cargo test -p al-math` | T3 |
| `dsp::fft` シグネチャ不変・single-shot caller 無変更 | コード検査 + 既存テスト green | T2 |
| `cargo tree -p al-analysis --depth 1` に al-math 直接依存なし | `mise exec -- cargo tree -p al-analysis --depth 1` | T1–T5 全体 |
| `conv_direct` 参照実装が test/bench 内に存在し A/B 比較に使用可 | `benches/conv.rs`・`dsp.rs`/`reference.rs` tests 検査 | T1, T4 |
| workspace test・clippy・fmt green・golden pass | `mise exec -- cargo test --workspace` 等 | 各タスク + T5 |

## Open Questions / Gaps

- criterion baseline 名の具体値(`--save-baseline <name>` の name)
  は実施時にタスク内で統一する(例: `stft-pre-hoist`・
  `conv-direct`・`lstsq-rows`)。baseline 名は IR に記録する
- `bench_reference` の `analyze_pair` fixture は有界な synthetic
  (数秒規模 LCG ノイズ + 構成済み mix) — 具体的なサイズ・構成は
  T1 実施時に決め、IR に fixture 定義を記録する
- `docs/impl/ir/` の記録ファイル名・分割(1 ファイルか opt 毎か)
  は T1 で IR ドラフトを起こす際に既存 IR 規約(`docs/impl/ir/` の
  既存レコード)に倣う

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(T1→T2→T3→T4→T5 の直列依存。
  各 opt の bench delta は進行する baseline に対して計測するため
  並列化しない)
- Independent work streams: なし(T1 内の char コミット群は論理的に
  並行可能だが、同一ファイル群を共有するため直列扱いとした)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- design の characterization 配置表・ベンチ構成・コミット順序・
  caller 移行マップ・`dsp::conv` 設計を正本として転記済み。
  レビュアは (a) T1 への char 集約と opt 順 A3→A4→A1 が spec
  プロセス契約を損なわないこと、(b) Verification matrix が spec
  Acceptance Criteria を網羅していること、(c) T3 の大きな置換 diff
  が独立コミット境界で revert 可能なことを確認すること
- design の推奨コミット順序(A1 先行)からの入替は plan の
  Implementation Sequence に理由とともに記録済み。opt 適用順の
  変更を望む場合、T2–T4 の depends-on チェーンを組み替えればよい
  (各 opt は T1 のみに実質依存)
