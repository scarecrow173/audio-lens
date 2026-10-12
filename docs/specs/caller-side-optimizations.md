---
id: "SPEC-034EHH6AJBK8GJ2H9EUzjE"
type: "spec"
status: "approved"
title: "caller 側最適化チェーン: fir_model 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4)"
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
  implements: []
  implemented-by:
    - "../designs/caller-side-optimizations.md"
  depends-on:
    - "./al-math-vec-primitives.md"
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
    - "./observability-foundation.md"
    - "../adr/stage-timing-manifest-persistence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/caller-side-optimizations-structure.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/caller-side-optimizations.md"
  derived-by:
    - "../designs/caller-side-optimizations.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# caller 側最適化チェーン: fir_model 畳み込み FFT 化・STFT plan 再利用・LeastSquares 列指向 (A1/A3/A4)

## What Should Be Built

al-analysis / al-locate の呼出し側(caller)コードにある3つの既知
ホットスポットを、al-math のカーネル(`Fft`/`RealFft`/vec
プリミティブ)を変更せずに最適化する。変更面は caller コードと
`linalg` の API 変更(列指向エントリへの置換)のみ。

- **A1 — fir_model 畳み込みの FFT 化**: `reference.rs` の
  `fir_model` クロージャ内のスカラ直接畳み込み
  (`conv_len = hre.len().min(4096)`、`y[i + k] += x * hv` の
  2重ループ、O(n×conv_len≤4096))を、既存 `RealFft` と plan
  再利用パターンに基づく FFT 畳み込み(overlap-save)に置き換える。
  A/B 比較用に自己完結な直接畳み込み参照実装を test/bench コードに
  保持する。併せて `reference.rs` の `subtract` クロージャ内
  `m[i.min(m.len())]` の境界外 index(残差が model より長い場合に
  `m.len()` で index し panic し得る)を小さな correctness タスク
  として修正する — `m.get(i).copied().unwrap_or(0.0)` の
  zero-fill へ置き換える。Aligned の shared-length 契約
  (`al-analysis/src/stage.rs:283-295` が全チャネルを同一 `len`
  にスライス)の下で models/residuals は長さ n を共有するため
  当該分岐は本番では到達不能であり、契約違反時(ragged な
  チャネルを持つテストヘルパ等)のみ到達する — 修正は挙動変更
  ではなく構造的防御
- **A3 — STFT の plan 巻き上げ**: `stft.rs` のフレームループが
  毎フレーム `dsp::fft` 経由で `Fft::new` を構築しているのを、
  plan をループ外に巻き上げて再利用する内部リファクタに改める。
  フレームバッファ(`vec![0.0; fft_size]` ×2)は `re.push(fre)` で
  出力 `StftMatrix` の行に move される(`stft.rs:113-121`)ため
  巻き上げ不可 — バッファ自体が出力であり、出力行確保は現行の
  まま残る。`dsp::fft` の公開シグネチャは変更せず、plan を
  持ち回る経路を dsp 側に追加する。出力は現行と**ビット一致
  必須**(`Fft::new` の twiddle は決定的で、`transform` は
  バッファを全上書きするため plan 再利用で保証できる)
- **A4 — LeastSquares 列指向エントリ**: `al_math::linalg::
  LeastSquares` に列優先エントリ `from_columns(&[&[f64]], λ)` を
  追加し、`fit_mix` が行う sample ごとの行 `Vec` マテリアライズ
  (`mixer.rs`、n_src=1 で ~8.6M alloc 規模)と linalg 内部の
  strided 列再収集・行セット clone を除去する。残差 RSS は連続
  バッファ上の `sub_scaled`/`sum_sq` で計算する。既存 rows API
  (`LeastSquares::new(&[Vec<f64>])`・`least_squares` free fn の
  行シグネチャ)は**置換**する(pre-release 方針により両建て
  しない。`least_squares` は列シグネチャで存続)。`fit_mix` と全
  caller(`reference.rs` ×3・`residual.rs`・`stage.rs`)を列指向
  に移行し、`reference.rs` の `.to_vec()` チャネルコピーは除去
  する(決定済み — `src.src`/`src.cap` の `Vec<&[f64]>` を直接
  渡す)

プロセス契約(ユーザー指定の最高優先事項): 各最適化の着地前に
(1) 現行出力を捕捉する characterization テストが存在し pass
すること、(2) ベンチマークを変更前(baseline)と変更後に実行する
こと、(3) before/after の実測を Implementation Record に記録する
こと。

> 数値等価ポリシーの権威は `docs/adr/simd-reduction-lane-fma.md`
> (ADR-034EoHxzsvsHp9BOdCK2Cn)とし、A1/A4 の許容誤差はその
> recomposition tolerance 述語 `|a - b| <= 1e-12 * max(1, |a|, |b|)`
> を要素毎に適用する。API・構造面の決定(列指向 `LeastSquares` API
> 形・畳み込みアルゴリズム選定・STFT plan 所有境界)の権威は
> `docs/adr/caller-side-optimizations-structure.md` とする。本 spec
> 内の再掲が ADR と乖離した場合は ADR を正とし、spec 側を追従修正
> する(契約文面の二重管理によるドリフト防止)。

## Why Now

vec プリミティブ集約チェーン(al-math-simd-reduction)と
observability 基盤チェーンの完了により、前提が揃った:

- A4 の残差計算に必要な `sub_scaled`/`sum_sq` が `al_locate::dsp`
  ファサード経由で利用可能になった
- stage `duration_ms` の manifest 永続化と criterion `=0.8.2`
  dev-dep(al-math)の先例により、stage 級・kernel 級の両面の
  計測基盤が整った
- discovery(`docs/discovery/caller-side-optimizations.md`、
  DISC-034EHtcL712sw3rAvtFX5g)でスコープ・数値契約・計測手続きが
  deep-dive で確定済み

`fir_model` の直接畳み込みは数分規模の信号(n ≈ 2–4.5M samples)
で ~9–18G MACs/pair/チャネルと見積もられ、reference stage で
最大級の単一計算である。

## Users and Value

- 解析パイプラインの利用者: reference stage の FIR 畳み込みが
  O(n·4096) から O(n log n) に縮み、pair あたり ~10 本の STFT の
  毎フレーム plan 構築コストが消え、stage `duration_ms` の
  改善として観測できる(実測で確認する hypothesis)
- 実装者: plan 再利用の正規経路(dsp 側 plan キャリア)と列指向
  lstsq API が確立し、A2(ncc plan 再利用)等の後続 caller 側
  最適化の足場になる
- レビュアー・後続の担当者: characterization-first のプロセス
  契約により、各最適化の前後差分がテストと記録済み実測で検証可能

## Scope

### In Scope

- **A1**: `reference.rs` の `fir_model` が行う畳み込みを FFT
  ベース(overlap-save、既存 `RealFft`/plan 再利用パターン)に
  置き換える。現行意味を維持する: `conv_len = hre.len().min(4096)`
  のキャップ、出力 = 線形畳み込みの先頭 n 要素(尾部打ち切り)。
  畳み込みを test/bench から到達可能な関数として切り出し、
  自己完結な直接畳み込み参照実装を test/bench コード内に保持
  する(A/B 比較用)。`fir_model` 自体も `analyze_pair` 内の
  クロージャ(`reference.rs:1052`)から名前付き関数(pub(crate)
  またはモジュール private + `#[cfg(test)]` テスト)へ切り出し、
  characterization を関数レベルで行う(畳み込み切り出しと同じ
  方針)。baseline の直接畳み込みは O(n·conv_len) のため bench
  fixture は有界化する(n ≤ ~1M samples、または criterion の
  `sample_size(10)`・短い `measurement_time` で、legacy
  baseline が sample あたり数分掛からないようにする)
- **A1 付随(correctness)**: `subtract` クロージャ内
  `m[i.min(m.len())]` を `m.get(i).copied().unwrap_or(0.0)` の
  zero-fill に置き換える。clamp(`m.len() - 1` 上限)は採用しない:
  隣接する `model.get(c).map_or(0.0)` 規約と一致し、空モデル
  防御も包含する。models/residuals は Aligned shared-length
  契約(`stage.rs:283-295`)の下で長さ n を共有するため本番では
  到達不能であり、到達するのは契約違反時(ragged チャネルを持つ
  テストヘルパ等)のみ — 構造的防御であり挙動変更ではない。
  修正は characterization コミットに同梱し、A1 最適化の適用
  (等価性ゲート実行)より先に land させる(到達不能のため捕捉
  出力への数値影響はないが、変更順序を記録上明確にするため)
- **A3**: `stft` の plan ループ外巻き上げ。dsp 側に plan を
  持ち回る経路(`al_math::fft::Fft` の dsp 公開または等価の
  薄いキャリア)を追加する。`dsp::fft` のシグネチャと
  single-shot caller(`codec.rs`・`dynamics.rs`・`ir.rs`・
  `noise.rs`・`reference.rs` の irfft)は変更しない。
  フレームバッファの巻き上げは行わない: 各フレームの
  `vec![0.0; fft_size]` は `re.push(fre)` で出力 `StftMatrix`
  の行に move される(`stft.rs:113-121`)ためバッファ自体が
  出力であり、出力行確保は残る(出力行のバッファ再利用は
  スコープ外)。チェック可能な不変条件は「stft ループ内に
  `Fft::new` がない」こと。出力はビット一致。なお
  `cheap.rs:940` も `stft` の caller だが、`stft` の公開
  シグネチャは不変のため修正不要(情報として記録)
- **A4**: `LeastSquares::from_columns(&[&[f64]], λ)` の追加と
  `fit_mix` の列指向移行。既存 rows API(`LeastSquares::new`・
  `least_squares` の行シグネチャ)は**置換**であり追加両建て
  ではない(pre-release 方針)。`least_squares` free fn の
  シグネチャ変更は `dsp.rs:114` のテストに波及する。caller
  移行(`mixer.rs`、`reference.rs` の channel family /
  crosstalk / R2 fit の3箇所、`residual.rs` の `subtract_fit`
  内部呼出し、`stage.rs` の cap_window fit)。`reference.rs` の
  `.to_vec()` チャネルコピーは除去する(決定済み —
  `src.src`/`src.cap` の `Vec<&[f64]>` を直接渡す。
  `residual.rs` の clone+resize は長さ合わせとして評価の上で
  最小化)。`subtract_fit`/`prepare_residual` の公開シグネチャは
  `&[Vec<f64>]` のまま維持し内部で列参照へ変換する(re-export
  面 `lib.rs:29-30` と caller `stage.rs:947`・
  `tests/locate.rs:300`・`residual.rs` テスト `:269,306` は
  無変更)。残差 RSS を列上の `sub_scaled`/`sum_sq` 計算へ
  置き換え
- **プロセス**: A1/A3/A4 それぞれについて、現行出力を捕捉する
  characterization テストの先行作成、ベンチ baseline/事後計測、
  Implementation Record(`docs/impl/ir/`)への before/after 実測記録
- **ベンチ基盤**: kernel 級計測(A1 畳み込み・A3 stft)は
  criterion を `crates/al-analysis` の dev-dependency
  (`=0.8.2`、al-math と同じ version・pattern であり新しい依存
  カテゴリではない)として追加し `benches/` に配置する。
  `crates/al-analysis/Cargo.toml` には al-math
  (`Cargo.toml:16-22`)と同じ `[[bench]] name = "..."
  harness = false` stanza を bench ごとに追加する。A4 の
  kernel 級計測は既存の al-math benches(criterion dev-dep
  導入済み)を使用し新たな dev-dep は追加しない。stage 級の
  代表経路計測は manifest 永続化済み `duration_ms` と、新設の
  `crates/al-analysis/examples/bench_reference.rs`(al-locate
  `bench_*` examples の Instant ベンチ規約に倣う)で行う —
  既存 Instant ベンチは stft/fir_model/analyze_pair/fit_mix を
  カバーしない(`bench_residual` は subtract ループのみ)ため、
  `bench_reference` は stft と fir_model 経路(または有界な
  synthetic analyze_pair fixture)を計測対象とする
- **回帰防止**: golden テスト(`al-analysis/tests/golden.rs`・
  `al-locate/tests/golden.rs` 等)の pass 維持

### Out of Scope

- stft フレームバッファの巻き上げ・再利用: `re.push(fre)` が各
  フレームの `vec![0.0; fft_size]` を出力 `StftMatrix` の行へ
  move する(`stft.rs:113-121`)ためバッファは出力そのもので
  あり、A3 の巻き上げ対象は plan のみ
- A2: `ncc` の呼出しごとの `RealFft::new` 再構築
  (`correlation.rs`)の plan 再利用 — 別 API 設計を要するため
  issue #42 系の別チェーンへ defer(discovery で確定済み)
- A5: `local_lag`→`ncc` 置換 — 契約判断を要するため別チェーン
- A6 以降の minor items(discovery の棚卸しを参照)
- al-math のカーネル内部(`Fft`/`RealFft`/vec プリミティブ/
  linalg の QR 数学)の変更 — issue #42 の別チェーン。本チェーンの
  変更面は caller と `linalg` の API 変更(置換)のみ
- マルチスレッド化・並列化
- al-analysis への al-math 直接依存の追加(dsp ファサード一元化を
  維持)
- golden テストの判定意味・Validity ゲート閾値の変更
- 互換層・deprecation 期間(pre-release 方針により置換でよい)

## Requirements

- **プロセス契約(最高優先)**: A1/A3/A4 はそれぞれ独立に適用
  可能な単位とし、各々について
  1. 変更前に現行出力を捕捉する characterization テストが存在し
     pass すること(テスト先行、異状のないことを確認してから
     実装する)
  2. ベンチを変更前(baseline 実測を記録)と変更後に実行すること
  3. before/after の実測値を Implementation Record
     (`docs/impl/ir/`)に記録すること
  4. `subtract` の OOB 修正は characterization コミットに同梱し、
     A1 最適化の適用(等価性ゲート実行)より先に land させる —
     本番では到達不能のため捕捉出力への数値影響はないが、変更
     順序を記録上明確にするため
- **数値等価**:
  - A3: `stft` 出力(`StftMatrix.re`/`im` 全要素と `frames`/
    `fft_size` メタ)は変更前後でビット一致必須
  - A1/A4: 固定 fixture 上で要素毎に
    `|a - b| <= 1e-12 * max(1.0, |a|, |b|)` を満たすこと
    (ADR-034EoHxzsvsHp9BOdCK2Cn の recomposition tolerance の
    適用。ベクトル出力は要素毎適用、スカラー出力は同述語)
  - fixture は決定的(シード固定パターンまたはテスト内の
    synthetic fixture)でエッジケースを含む: 空入力・
    `n < conv_len`・`n < window`(STFT 0フレーム経路)・
    単一フレーム・全ゼロ入力
  - 永続化値(`ResidualEnergy`・feature record・gain params・
    `MixerEstimate`)は tolerance 内で変動し得るが、golden テストは
    pass 必須
- **API**:
  - `LeastSquares::from_columns(&[&[f64]], λ)` は `new` と同じ
    契約(有限値・非負有限 λ・観測数 ≥ 列数・列等長)を列指向で
    持ち、rows API(`new`・`least_squares` の行シグネチャ)を
    置換する。`solve(rhs)` は `rhs.len() == 観測数`(ridge
    augmentation 後の列長ではない)を要求する。詳細は ADR が権威
  - `dsp::fft` の公開シグネチャは維持し、plan 持ち回り経路は
    dsp 側に追加する
  - 畳み込み関数と `fir_model` は test/bench から到達可能な
    名前付き関数であること(関数切り出し)
- **依存**: `crates/al-analysis/Cargo.toml` に al-math 依存を
  追加しない。bench のための `criterion = "=0.8.2"` dev-dep
  追加は al-math と同 version・pattern であり新規依存カテゴリでは
  ないが、依存追加として Implementation Record に記録する
- A1 の直接畳み込み参照実装は bench/test 内に自己完結で保持し、
  本番コード経路からは除去してよい
- `mise exec -- cargo test -p al-math -p al-locate -p al-analysis`
  および `cargo test --workspace`、`cargo clippy`、
  `cargo fmt --check` が green であること

## Acceptance Criteria

各最適化は (a) characterization テストの先行 green、(b) 等価性
ゲート(A3 はビット一致、A1/A4 は述語)、(c) bench delta の記録、
(d) 代表 Instant ベンチでのノイズを超える回帰なし、の4条件を
満たすこと。

- [ ] A1/A3/A4 それぞれについて、変更前に作成した characterization
  テストが現行出力を捕捉し、最適化前に green・最適化後も green で
  ある(着地前の存在が PR/記録から確認できる)
- [ ] A3: 固定の決定的 fixture(シード固定パターンまたは
  テスト内の synthetic fixture)で `StftMatrix` の re/im が
  変更前後でビット一致するテストが存在し pass する。
  エッジケース: `samples.len() < window`(0フレーム経路)・
  単一フレーム(`window <= n < window + hop` または
  `hop >= n`)・非2冪 window・全ゼロ。コード検査で
  「stft ループ内に `Fft::new` がない」ことを確認する
- [ ] A1: 固定 fixture(空・n<conv_len・conv_len=4096 境界・
  全ゼロ・代表ノイズ)で新畳み込み出力と直接畳み込み参照実装の
  出力が要素毎 `|a-b| <= 1e-12 * max(1,|a|,|b|)` を満たすテスト
  が存在し pass する。切り出した名前付き `fir_model` 関数
  単位でも同 predicate の characterization が pass する
- [ ] A4: `from_columns` が同 fixture で rows 経路相当の解・残差
  RMS を predicate 内で返すテストが存在し pass する。`fit_mix`
  出力の characterization は `MixerEstimate.matrix` と
  `fit_quality` が predicate 内で一致すること。`condition_number`
  は述語の対象外とする — AᵀA 上の 64 回 power iteration 推定で
  感度が κ に対しほぼ二次的となり、ill-conditioned fixture では
  正当な実装でも述語を超え得るため。残す場合も位数一致・
  有限/非有限(INFINITY 整合)の粗い sanity bound に留める
- [ ] `subtract` の zero-fill 修正を検証するテスト(ragged な
  チャネル長を持つ契約違反 fixture で panic しないこと —
  本番経路は Aligned shared-length 契約により到達不能)が
  存在する
- [ ] 各最適化の criterion bench delta と stage 級代表経路
  (`examples/bench_reference` + manifest `duration_ms`)の
  before/after 実測が Implementation Record に記録され、
  ノイズ幅(目安 ±5%)を超える回帰がない
- [ ] `LeastSquares::from_columns` が公開 API として存在し、
  `mixer.rs` の per-sample 行 `Vec` マテリアライズと `linalg.rs`
  の行セット clone・strided gather が除去されている
- [ ] `dsp::fft` のシグネチャが不変で、single-shot caller が
  無変更である
- [ ] `mise exec -- cargo tree -p al-analysis --depth 1` の直接
  依存に al-math が現れない(または `crates/al-analysis/
  Cargo.toml` の直接確認で代替可)
- [ ] A1 の直接畳み込み参照実装が bench/test コード内に存在し、
  A/B 比較に使用できる
- [ ] workspace テスト・clippy・fmt が green で、golden テストが
  pass する

## Deferred Design Concerns

- A2 `ncc` plan 再利用・A5 `local_lag`→`ncc` 置換 — issue #42 系の
  別チェーンとして defer 確定(discovery の Tentative Conclusions)
- overlap-save のブロック長・境界条件の細部、plan キャリアの具体形
  — design で確定(ADR は overlap-save 採用と dsp 側所有の境界を
  決める)
- criterion ベンチの CI 回帰ゲート — 前 spec(al-math-vec-
  primitives)と同様、CI 基盤未整備のため見送り。CI 整備時に
  baseline 比較の導入を再検討する
- 数値ポリシー専用の第2 ADR は作成しない: tolerance 述語は
  ADR-034EoHxzsvsHp9BOdCK2Cn が既に権威として確立しており、
  本チェーンはその再適用のみで新規の数値決定を含まない
  (More Information 参照)

## Sources and Relations

- 調査: `docs/discovery/caller-side-optimizations.md`
  (DISC-034EHtcL712sw3rAvtFX5g — 全 finding を file:line で
  検証済み。スコープ・数値契約・計測手続きの確定事項、
  選択肢比較、Open Questions)
- 決定: `docs/adr/caller-side-optimizations-structure.md`
  (列指向 `LeastSquares` API・overlap-save 畳み込み採用・STFT
  plan 所有境界・畳み込み/参照実装の置き場)
- 数値ポリシー: `docs/adr/simd-reduction-lane-fma.md`
  (ADR-034EoHxzsvsHp9BOdCK2Cn — A1/A4 に適用する
  recomposition tolerance 述語の権威)
- 前提: `docs/specs/al-math-vec-primitives.md`・
  `docs/adr/al-math-vec-primitives-slice-boundary.md`
  (`sub_scaled`/`sum_sq`/`corr0` の dsp ファサード経由利用)、
  `docs/specs/observability-foundation.md`(criterion dev-dep 先例)、
  `docs/adr/stage-timing-manifest-persistence.md`
  (stage `duration_ms` 永続化による stage 計時証拠)
- 関連 issue: #42(DSP カーネル SIMD 化)に隣接するが、カーネル
  内部と A2/A5 は本チェーンのスコープ外
