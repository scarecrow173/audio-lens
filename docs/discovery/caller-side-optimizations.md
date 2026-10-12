---
id: "DISC-034EHtcL712sw3rAvtFX5g"
type: "discovery"
status: "active"
title: "caller 側最適化チェーン: FIR 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4)"
created: "2026-10-12"
updated: "2026-10-13"
owners: []
relations:
  source:
    - "../../crates/al-analysis/src/reference.rs"
    - "../../crates/al-analysis/src/stft.rs"
    - "../../crates/al-locate/src/dsp.rs"
    - "../../crates/al-locate/src/mixer.rs"
    - "../../crates/al-locate/src/residual.rs"
    - "../../crates/al-locate/src/stage.rs"
    - "../../crates/al-math/src/linalg.rs"
    - "../../crates/al-math/src/fft.rs"
    - "../../crates/al-math/src/correlation.rs"
    - "../../crates/al-math/src/filter.rs"
    - "../../crates/al-job/src/runner.rs"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../specs/al-math-vec-primitives.md"
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
    - "../specs/observability-foundation.md"
    - "../adr/stage-timing-manifest-persistence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./al-math-simd-reduction.md"
    - "./observability-foundation-logging-and-profiling.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/caller-side-optimizations.md"
    - "../adr/caller-side-optimizations-structure.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化チェーン: FIR 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4)

## Exploration Goal

vec プリミティブ集約チェーン(al-math-simd-reduction)と observability
基盤チェーンの完了を受け、次の最適化対象として caller 側(al-analysis /
al-locate の呼出し側コード)の既知ホットスポット調査結果 A1/A3/A4 を
1 つの実装チェーンにまとめる探索。al-math のカーネル自体
(Fft/RealFft/vec primitives)は変更しない。

本ドキュメントは調査で確定した各 finding をコード上の証跡
(file:line)とともに記録し、deep-dive で確定済みのスコープ・
数値契約・計測手続きを後続の spec/ADR/design の入力として固定する。

## Key Issues and Assumptions

### A1 — fir_model の直接畳み込み

- 確認済み(実装): `reference.rs:1052-1088` の `fir_model` クロージャが
  Welch 推定 FIR を適用する。STFT → CSD 累積(`:1053-1073`)→ inverse
  FFT(`:1075`)で impulse response `hre` を得た後、`:1076-1087` が
  スカラ直接畳み込み(`conv_len = hre.len().min(4096)`、
  `y[i + k] += x * hv` の2重ループ)。R2 残差の各 capture チャネルへ
  適用される(`:1126` `r2.iter().map(|ch| fir_model(ch))`)
- 確認済み(計算量): O(n × conv_len≤4096) を aligned 信号全長に実行。
  数分規模の信号(n ≈ 2–4.5M samples)で ~9–18G MACs/pair/チャネル
  となり、reference stage で最大級の単一計算と見積もられる
- 確認済み(数値契約): FFT ベース畳み込みはビット非一致。
  recomposition tolerance ポリシー(述語
  `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`、
  ADR-034EoHxzsvsHp9BOdCK2Cn `docs/adr/simd-reduction-lane-fma.md:109`
  の先例)の適用対象。永続化される `ResidualEnergy`
  (`residual.rs:104-109`、`ResidualPcmParams.energy` として
  `residual.rs:30,75` で manifest 永続化)は tolerance 内で変動し得る
- 確認済み(先行パターン): `al_math::fft::RealFft`(`fft.rs:717`)と
  plan 再利用パターン(`filter.rs:37` の `bandpass_many` が1 plan を
  forward/inverse で使い回す)が既存。`dsp::sub_scaled`/`sum_sq`/`corr0`
  は vec プリミティブ集約で dsp ファサード経由の利用が可能
  (`dsp.rs:34` で re-export 済み)
- 確認済み(潜在バグ): `reference.rs:1045` の `subtract` クロージャ内
  `m[i.min(m.len())]` は `i >= m.len()` のとき `m.len()` で index し
  境界外 panic の可能性がある(residual が model より長い場合)。
  到達可能性を評価し、境界で修正する小さな correctness タスクとして
  チェーンに含める → spec/ADR で確定: `m.get(i).copied().unwrap_or(0.0)`
  の zero-fill へ置換。Aligned shared-length 契約
  (al-analysis `stage.rs:283-295`)の下で本番到達不能の構造的防御

### A3 — STFT の per-frame plan 再構築

- 確認済み(実装): `stft.rs:106-131` のフレームループが毎フレーム
  `vec![0.0; fft_size]` を2本確保(`:113-114`)し `fft()` を呼ぶ
  (`:118`)。`dsp.rs:38-41` の `fft` wrapper は呼出しごとに
  `al_math::fft::Fft::new(re.len())` を構築する — `Fft` は全 twiddle
  テーブル(`fft.rs:9-16` `twiddles: Vec<f64>`)を持つため、plan
  構築コストが毎フレーム掛かっている
- 確認済み(呼出し量): pair あたり ~10 本の全信号 STFT が走る
  (`reference.rs:440-441` src/cap mono、`:792-793` stereo、`:1053`
  fir_model 内部 × capture チャネル数、`:1158` residual level ×4;
  ほか `band_gain.rs:151-152`、`cheap.rs:609`、
  `expensive/channel.rs:83,89,370-371`、`codec.rs:235`、
  `mod.rs:632`)× 各数千フレーム
- 確認済み(設計制約): `dsp::fft` は公開 convenience wrapper —
  シグネチャを維持し、plan を持ち回る内部経路を別途導入する。
  stft 以外の `fft()` 呼出し(`codec.rs:97`、`dynamics.rs:191`、
  `ir.rs:177`、`noise.rs:140`、`reference.rs:1075`)は single-shot で
  変更不要。al-analysis は al-math に直接依存しない規約(dsp
  ファサード一元化)を継続する
- 確認済み(数値契約): plan 再利用は同一の Fft math を使い回すだけで
  出力はビット一致が期待できる(必須条件として課す)

### A4 — fit_mix / LeastSquares の行マテリアライズ

- 確認済み(実装): `mixer.rs:55-57` が sample ごとに1行の
  `Vec<Vec<f64>>` を確保(n_src=1 で ~8.6M alloc 規模)、`:60-67` で
  AᵀA を行から二重ループ集計し、`:69` で `LeastSquares::new` へ渡す
- 確認済み(内部): `linalg.rs:56-101` の `Factor::with_simd` が
  `:62-68` で strided な列再収集(`rows.iter().map(|r| r[j])`)を行い、
  `:96` で行セット全体の clone(`rows.to_vec()`)を `LeastSquares` に
  保持する — これは `Apply`(`:122-127`)の残差 RSS 計算でのみ使われる
- 確認済み(caller): `reference.rs:731-733`(`:731-732` の
  `.to_vec()` チャネルコピーは `&[Vec<f64>]` シグネチャのためだけに
  存在 — 新 API で除去可能か評価)、`:882`、`:1105`;
  `residual.rs:153-161`(clone+resize `:153-160`、fit_mix `:161`、
  capture `to_vec()` `:162`); `al-locate/src/stage.rs:753-759`
  (cap_window の per-channel `.to_vec()` + fit_mix `:759`)
- 確認済み(数値契約): 列優先エントリ化で加算順が変わり、gains /
  ResidualEnergy に ulp 級の差が出得る — 上記 tolerance ポリシーの
  適用対象。残差計算は連続バッファ上の `sub_scaled`/`sum_sq` に
  置き換える(`residual.rs:163-172` が既に同形で「ADR の
  recomposition tolerance 内」と明示して実装済みの先例)

### 計測・検証基盤

- 確認済み(observability): stage 境界の `tracing::info_span!` と
  `StageRecord.duration_ms` の永続化が実装済み
  (`runner.rs:266-305`、`:288,:305` で record へ記録) — stage 単位の
  計時証拠が取得可能になった
- 確認済み(bench 先例): kernel 級計測は criterion(al-math
  Cargo.toml:10-14 で `=0.8.2`、公開 ≥7 日ルール適用済み)。
  al-analysis は現状 criterion dev-dep なし(al-testkit のみ) —
  benches が crate 内部を要する場合のみ同等の dev-dep 追加を検討
  (依存追加は確認対象)、不要なら既存 examples/ の Instant 手動計測
  規約(bench_dsp/bench_ncc/bench_locate_prepared/bench_residual/
  bench_correlation)を踏襲する
- 確認済み(golden): `crates/al-analysis/tests/golden.rs`・
  `al-locate/tests/golden.rs` 等が存在し、最適化後も pass 必須
- 前提: kernels untouched — al-math のカーネル実装は変更しない。
  チェーンの変更面は al-analysis / al-locate の caller と
  linalg の API 変更(列指向エントリへの置換)
- 前提: pre-release 方針により互換層・移行期間は設けない。
  API 変更は置換でよい
- 前提(ユーザー指定の process ポリシー、制約として課す):
  最適化の前にテストとベンチマークを作成し、結果に異状がないか
  確認してから実装する。before/after の実測を Implementation
  Record に記録する。具体的には (1) 現行出力を捕捉する
  characterization テストが各最適化の着地前に存在し pass すること、
  (2) ベンチマークを変更前(baseline)と変更後に実行すること、
  (3) A1 の baseline として自己完結な直接畳み込み参照実装を
  比較用に保持すること

## Alternatives and Comparison

### スコープ単位

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A1+A3+A4 を1チェーン | いずれも caller 側・同一の tolerance ポリシーと計測手続きを共有できる。characterization 基盤を1回整えれば3施策に使い回せる | 差分が大きい | yes(確定) |
| 個別チェーン | 各差分は小さい | 同じテスト/ベンチ基盤を3回用意する重複。依存関係もない | no |
| kernel 側最適化も同梱 | — | 「kernels untouched」のスコープ確定と矛盾。issue #42 系と混ざる | no |

### A1 実装方式

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| FFT ベース畳み込み(overlap-save または `RealFft`/`bandpass_many` 型の spectral path) | O(n·4096) → O(n log n)。既存 plan 再利用パターンと一貫 | ビット非一致 → tolerance 適用と characterization が必須。境界条件の設計が要る | yes |
| direct conv 維持のまま SIMD 化 | 丸め差が小さい | O(n×4096) の計算量自体が残り効果が限定的。kernel 側変更でスコープ違反 | no |

### A3 plan の所有境界

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| plan を持ち回る内部経路を dsp 側に追加し stft ループが利用(`dsp::fft` は維持) | 公開シグネチャ不変。dsp ファサード一元化を維持。他 caller 無影響 | dsp に plan キャリア型の追加 | yes |
| `dsp::fft` のシグネチャを plan 受取に変更 | 実装は素直 | 公開 wrapper の契約破壊。single-shot caller 全てに波及 | no |
| stft が al_math を直接参照 | plan 制御が素直 | al-analysis→al-math 依存追加が発生しファサード規約が崩れる | no |

### A4 API 形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| `LeastSquares::from_columns(&[&[f64]], λ)` 列優先エントリ | チャネルがそのまま列なので行マテリアライズ・strided gather・行 clone が全て消える。残差は連続バッファの `sub_scaled`/`sum_sq` で計算可 | 加算順変化で ulp 級差(tolerance 適用)。linalg に新エントリ | yes |
| 現行 rows API 維持 + fit_mix 内で行バッファ再利用 | API 変更不要 | `Vec<Vec<f64>>` の1回分構築と内部 gather/clone は残り抜本的解消にならない | no |

### ベンチ方法論

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| kernel 級は criterion(al-math `=0.8.2` dev-dep 先例) | 統計的計測・baseline 比較が標準 | 依存追加(選定ルール済み) | yes(確定) |
| al-analysis に criterion dev-dep 追加 | STFT/畳み込みの crate 内部計測が可能 | 依存追加は確認対象 | maybe — benches が内部を要する場合のみ(確定事項) |
| 既存 examples/ Instant ベンチ踏襲 | 依存追加なし | 統計的信頼性が低い | fallback(確定) |
| A1 は direct conv を bench/test fixture として保持 | A/B 比較が self-contained。before/after 実測ポリシーと整合 | 期間限定の2系統 | yes(確定) |

## Tentative Conclusions and Hypotheses

以下は全て deep-dive 対話でユーザー確認済み:

- confirmed: スコープ = A1+A3+A4 を 1 チェーン(caller-side
  optimizations; kernels untouched)
- confirmed: スコープ外/延期 — A2 ncc plan 再利用(`ncc` は呼出し
  ごとに `RealFft::new` を構築、`correlation.rs:65`。別 API 設計が
  要るため、隣接する issue #42 と絡めた別チェーンへ)、A5
  `local_lag`→`ncc` 置換(`reference.rs:150`。契約判断が要る)、
  A6 以降の minor items
- confirmed(数値契約): A3 はビット一致必須、A1/A4 は 1e-12 スケール
  述語の範囲内を許容。永続化値(ResidualEnergy・feature records)は
  tolerance 内で変動し得るが golden テストは pass 必須
- confirmed(process ポリシー): 各最適化は characterization テスト
  (現行出力の捕捉)とベンチ baseline を先に作り、異状がないことを
  確認してから実装。before/after の実測を Implementation Record に
  記録する
- confirmed(A1): legacy 直接畳み込み実装は bench/test fixture として
  アクセス可能な形で保持し、A/B 比較に使う
- confirmed(設計 note): `dsp::fft` の公開シグネチャは維持し、plan を
  持ち回る内部経路を別途導入する(A3)
- confirmed(API 方向): A4 は列優先エントリ
  `LeastSquares::from_columns(&[&[f64]], λ)` で行マテリアライズを
  除去する方向。`reference.rs:731-732` の `.to_vec()` コピーは
  ADR で除去確定(`src.src`/`src.cap` の `Vec<&[f64]>` を直接
  渡す)。既存 rows API は置換(pre-release 方針)
- confirmed(correctness): `reference.rs:1045` の
  `m[i.min(m.len())]` 境界外 index を小さな correctness タスクとして
  チェーンに含め、到達可能性評価と境界での修正を行う
  → spec/ADR で zero-fill(`m.get(i).copied().unwrap_or(0.0)`)に確定。
  characterization コミットに同梱して A1 適用より先に land させる
- hypothesis: A1 の FFT 化で reference stage 最大の単一計算が
  O(n log n) に縮み、stage `duration_ms` の大きな改善として
  観測できる(実測で確認)
- hypothesis: A3 の plan 巻き上げで STFT コストの ~30-50%
  が消え、~10 本/pair の STFT 全体に効く(実測で確認。
  フレームバッファは `re.push(fre)` で出力行に move されるため
  巻き上げ対象は plan のみ — spec で確定)
- hypothesis: A4 の列優先化で ~8.6M 規模の一時 alloc と strided
  gather が消え、fit_mix 経路が定数時間に近い overhead になる
  (実測で確認)

## Open Questions

- A1: overlap-save と全長 spectral path(bandpass_many 型)の選定、
  conv_len 4096 キャップの扱い、境界条件設計 → design で確定
- A3: plan キャリアの具体的形(dsp ファサードに何を露出するか)→
  design で確定。scratch buffer の所有は spec で確定済み:
  フレームバッファは `re.push(fre)` で出力 `StftMatrix` 行に
  move される(`stft.rs:113-121`)ため巻き上げ対象は plan のみ
- A4: `from_columns` の契約(等長・finite 検査の置き場、ridge
  augmentation の列への表現)→ spec/ADR で確定。既存 rows API の
  存否も ADR で置換に確定
- `reference.rs:1045` OOB の実際の到達可能性 → spec/ADR で確定:
  Aligned shared-length 契約(al-analysis `stage.rs:283-295`)の
  下で models/residuals は長さ n を共有し本番到達不能。
  zero-fill の構造的防御として修正
- al-analysis への criterion dev-dep 追加の要否 → spec で確定:
  追加する(`=0.8.2` + `[[bench]] harness = false` stanza、
  al-math `Cargo.toml:16-22` と同 pattern)
- characterization テストの捕捉粒度 → spec/ADR で確定: 関数
  レベル。畳み込みと `fir_model` を名前付き関数へ切り出して
  test/bench から到達可能にする

## Promotion Candidates

- [ ] spec-doc needed: caller 側最適化チェーンのスコープ・受入基準 —
  characterization テスト/ベンチ baseline の先行、A3 ビット一致・
  A1/A4 tolerance 適用の数値契約、golden テスト維持、A1 direct-conv
  fixture 保持、Implementation Record への before/after 実測記録
- [ ] adr-doc needed: A1 FFT 畳み込み化(非ビット一致・tolerance
  ポリシー適用)と A4 列優先 `LeastSquares` API 追加の方針
- [ ] correctness task: `reference.rs:1045` 境界外 index の到達可能性
  評価と修正(最適化と独立した小タスク)
- [ ] defer 先: A2 ncc plan 再利用・A5 local_lag→ncc は別チェーン
  (issue #42 系)として後続探索

## Sources and Relations

- 調査対象コードは `relations.source` に記載(`reference.rs`・
  `stft.rs`・`dsp.rs`・`mixer.rs`・`residual.rs`・al-locate
  `stage.rs`・`linalg.rs`・`fft.rs`・`correlation.rs`・`filter.rs`・
  `runner.rs`)。本ドキュメントの finding は全て file:line で検証済み
- 前提ドキュメント(depends-on で連結):
  `docs/specs/al-math-vec-primitives.md`・
  `docs/adr/al-math-vec-primitives-slice-boundary.md` — `sub_scaled`/
  `sum_sq`/`corr0` が dsp ファサード経由で利用可能(A4 残差計算の
  前提)
  `docs/adr/simd-reduction-lane-fma.md` — recomposition tolerance
  述語 `|a-b| <= 1e-12*max(1,|a|,|b|)` の先例(A1/A4 の数値契約)
  `docs/specs/observability-foundation.md`・
  `docs/adr/stage-timing-manifest-persistence.md` — `duration_ms` と
  stage 境界 tracing span による stage 計時証拠の前提
- 関連探索(related で連結): `al-math-simd-reduction.md`
  (前チェーン)、`observability-foundation-logging-and-profiling.md`
  (計測基盤チェーン)
- issue #42(DSP カーネル SIMD 化)に隣接するが、A2 ncc plan 再利用は
  別 API 設計として本チェーンのスコープ外(defer)
- 作業ブランチ: `feature/caller-side-optimizations`
