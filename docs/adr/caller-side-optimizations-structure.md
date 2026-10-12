---
id: "ADR-034EHHl9H4TTHHr0OQEa39"
type: "adr"
status: "accepted"
title: "caller 側最適化の構造決定: LeastSquares 列指向 API・FIR 畳み込みの FFT 化・STFT plan 所有境界"
created: "2026-10-12"
updated: "2026-10-13"
owners:
  - "scarecrow173"
relations:
  source:
    - "../discovery/caller-side-optimizations.md"
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
    - "al-math-vec-primitives-slice-boundary.md"
    - "simd-reduction-lane-fma.md"
    - "al-math-api-fft-split-re-im.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/caller-side-optimizations.md"
    - "spectral-envelope-fft-size-assert.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/caller-side-optimizations.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - "scarecrow173"
    consulted: []
    informed: []
---

# caller 側最適化の構造決定: LeastSquares 列指向 API・FIR 畳み込みの FFT 化・STFT plan 所有境界

## Context and Problem Statement

caller 側最適化チェーン(discovery
DISC-034EHtcL712sw3rAvtFX5g、実施スコープは
`docs/specs/caller-side-optimizations.md`)の3施策を実装するに
あたり、以下の API・構造決定が必要。

1. **A4 — LeastSquares の列指向エントリ**: `fit_mix` は各 sample
   を1行とする `Vec<Vec<f64>>` を確保して `LeastSquares::new` に
   渡し、`linalg` 内部でも strided な列再収集と行セット全体の
   clone を行っている。ソースチャネルは既に「列」として連続
   バッファで手元にあるため、行マテリアライズ・strided gather・
   行 clone を除去するエントリの API 形と、既存 rows API の存否を
   決める。
2. **A1 — fir_model 畳み込みの FFT 化方式**: `reference.rs` の
   `fir_model` は O(n×conv_len≤4096) のスカラ直接畳み込み。
   FFT ベース化は確定済みだが、overlap-save と全長 spectral
   path(`bandpass_many`/`ncc` 型)のどちらを採るか、畳み込み
   関数をどこに置き、test/bench からどう到達させるかを決める。
3. **A3 — STFT plan の所有境界**: `stft` が毎フレーム
   `dsp::fft` 経由で `Fft::new` を構築している。plan を持ち回る
   経路をどこに置くか(`dsp::fft` のシグネチャ維持は確定済み)。

制約: al-math のカーネルは変更しない(kernels untouched)。
al-analysis は al-math に直接依存せず DSP は `al_locate::dsp`
ファサード経由が規約(`al-math-vec-primitives-slice-boundary.md`)。
数値等価ポリシーは `simd-reduction-lane-fma.md` が権威で、本
チェーンはその述語を再適用するのみ(新規の数値 ADR は作らない —
More Information 参照)。pre-release 方針により互換層は設けず
置換でよい。

## Decision Drivers

* dsp ファサードによる消費経路の一元化を維持し、al-analysis に
  依存変更を発生させない(前 ADR と同じ driver)
* kernels untouched: 変更面は caller と `linalg` の API 変更
  (列指向エントリへの置換)のみ。`Fft`/`RealFft`/vec プリミティブ
  のカーネル数学は不変
* 「fail, never guess」: `LeastSquares::new` は不正入力で
  `Option::None` を返す契約を持ち、列指向エントリも同じ系に揃える
* 計算量とメモリ: n ≈ 2–4.5M samples、conv_len ≤ 4096 に対し、
  メモリを有界に保ちつつ O(n log n) へ縮める
* characterization-first: 畳み込み関数は test/bench から到達
  可能でなければ、現行出力の捕捉と A/B 比較ができない
* 数値契約: A3 はビット一致(同一カーネル再利用で保証可能)、
  A1/A4 は 1e-12 スケール述語内の差を許容
* 表現ドリフト防止: 行・列の二重表現を残さず、lstsq 内部の正本を
  1つにする

## Considered Options

A4 — LeastSquares 列指向エントリ:

* `from_columns(&[&[f64]], λ)` を追加し rows API は置換(列を
  内部正本に統一)
* `from_columns` を追加しつつ `new(&[Vec<f64>])` も残す(両建て)
* 現行 rows API 維持 + `fit_mix` 内で行バッファを再利用

A1 — 畳み込み方式と置き場:

* overlap-save(dsp 側に畳み込み関数を追加、block FFT サイズを
  有界化)
* 全長 spectral path(`ncc`/`bandpass_many` 型、next_pow2(n+M)
  の1回 FFT)
* direct conv 維持のまま SIMD 化

A3 — plan 所有境界:

* dsp 側に plan を持ち回る経路(`al_math::fft::Fft` の dsp 公開
  または薄いキャリア型)を追加し stft ループが利用(`dsp::fft`
  は維持)
* `dsp::fft` のシグネチャを plan 受取に変更
* stft が al_math を直接参照

## Decision Outcome

Chosen option: "A4 = `from_columns` 追加 + rows API 置換、A1 =
overlap-save を dsp 側関数として実装、A3 = dsp 側に plan 公開
経路を追加", because 列が既に連続バッファで存在するため行マテリ
アライズを構造的に除去でき、overlap-save は巨大な単一 FFT を避け
てメモリを有界に保ち、dsp への集約がファサード一元化と既存の
手書き wrapper 先例(`dsp::fft` 自体がその形)に一致するため。

契約の確定内容:

* **A4 API**: `LeastSquares::from_columns(columns: &[&[f64]],
  lambda: f64) -> Option<Self>` を追加する。`columns` は観測
  系列(= ソースチャネル)で、契約は `new` の列向き写し:
  全列が等長・有限値・観測数(列長) ≥ 列数・λ が非負有限で
  なければ `None`。`solve(rhs)` は `rhs.len() == 観測数`
  (ridge augmentation 後の列長ではない)を要求する
* **内部表現は列正本**: `Factor` は列をそのまま消費し strided
  gather を行わない。ridge augmentation は各列への追加要素
  (diag = √λ)として分解内部のみに存在させる。`LeastSquares` は
  ridge 適用前の入力列を 1 コピーだけ所有し(現行の
  `rows.to_vec()` 行 clone の置き換え)、残差 RSS は `Apply` で
  その augment 前列を使い `resid = rhs - Σ_j x_j·col_j` を連続
  バッファ上の `sub_scaled`/`sum_sq` で計算する(`residual.rs`
  の per-source `sub_scaled` 残差先例と同形)。RSS は現行どおり
  ridge 項を含めない — `Apply` の RSS は未 augment の観測行上で
  計算される契約(`linalg.rs:38`・`:122-128`)であり、QR 分解の
  尾部から RSS を取る実装にすると `residual_rms`・`fit_quality`
  の意味が変わるため避ける
* **rows API の存否**: `LeastSquares::new(&[Vec<f64>])` は除去し
  `from_columns` に置き換える(pre-release 方針により互換層を
  設けない。行・列の二重表現を残さない)。`least_squares` free
  function は列シグネチャ
  `least_squares(columns: &[&[f64]], rhs: &[f64], lambda: f64)`
  に置き換えて存続させ、al-locate root の re-export 公開面を
  維持する
* **fit_mix のシグネチャ**: `fit_mix(source_channels: &[&[f64]],
  capture_channels: &[&[f64]], ridge)` に変更し、caller 側の
  `&[Vec<f64>]` 用コピーを除去可能にする。`reference.rs` の
  `src_chs`/`cap_chs` の `.to_vec()` は `src.src`/`src.cap`
  (`Vec<&[f64]>`)をそのまま渡す形に置き換え、`stage.rs` の
  `cap_window` もスライス参照へ切り替える。`residual.rs` の
  clone+resize は長さ合わせとして必要なら残す(評価の上で最小化)
  。AᵀA 集計も列上の `dot` で行う
* **A1 方式**: overlap-save による FFT 畳み込みを採用する。
  `conv_len = hre.len().min(4096)` のキャップと「出力 = 線形
  畳み込みの先頭 n 要素(尾部打ち切り)」という現行意味を維持
  する。ブロック長・境界処理の具体値は design で確定するが、
  単一 plan をブロック間で再利用することは本決定に含める
* **A1 置き場**: 畳み込みは `al_locate::dsp` 側に畳み込み関数
  (仮称 `conv`、実名・シグネチャは design で確定)として実装
  し、内部で `al_math::fft::RealFft` を使う。`fir_model` は
  それを呼ぶ。al-analysis の bench から `dsp::` 経由で直接
  到達可能とする。`fir_model` 自体も `analyze_pair` 内の
  クロージャ(`reference.rs:1052`)から名前付き関数へ切り出し、
  characterization を関数レベルで行う(畳み込み関数の切り出しと
  同じ方針)。直接畳み込み参照実装は test/bench コード内に
  自己完結で保持し、A/B 比較に使う(本番経路には残さない。
  直接畳み込みは O(n·conv_len) のため baseline bench の
  fixture は有界化する — n ≤ ~1M samples または criterion
  `sample_size(10)`・短い `measurement_time`)
* **A3 plan 所有**: `dsp` が `al_math::fft::Fft` を公開
  (re-export または等価の薄いキャリア)し、`stft` は plan を
  ループ外に巻き上げて使い回す。フレームバッファは巻き上げ
  ない: 各フレームの `vec![0.0; fft_size]` は `re.push(fre)` で
  出力 `StftMatrix` の行に move される(`stft.rs:113-121`)
  ためバッファ自体が出力であり、出力行確保は残る(出力行の
  バッファ再利用はスコープ外)。チェック可能な不変条件は
  「stft ループ内に `Fft::new` がない」こと。`dsp::fft` の公開
  シグネチャは維持し、single-shot caller は変更しない。
  `Fft::new` の twiddle は決定的で `transform` はバッファを
  全上書きするため、同一 `Fft` カーネルの再利用で出力は
  ビット一致とする

### Consequences

* Good, because `fit_mix` の ~8.6M 規模の一時 alloc・strided
  gather・行 clone が構造的に除去され、lstsq 内部の正本が列に
  統一されて表現ドリフトが起きない
* Good, because overlap-save が block FFT サイズを有界に保ち、
  全長 spectral path の next_pow2(n+4096) ≈ 8M 要素の巨大
  変換とそのメモリ・キャッシュコストを避けられる
* Good, because 畳み込み・plan が dsp に集約され、al-analysis は
  依存追加なしで利用でき、FFT ブロック制御が caller に漏れない
* Good, because `dsp::fft` 維持により single-shot caller
  (`codec.rs`・`dynamics.rs`・`ir.rs`・`noise.rs`・
  `reference.rs` の irfft)に波及しない
* Good, because rows API 置換により二重エントリ・二重表現を
  残さず、validation 経路が1つになる
* Bad, because overlap-save は block ループと境界処理のコードを
  要し、全長 spectral path より実装が複雑(edge ケースは fixture
  と参照実装の A/B で担保する)
* Bad, because `from_columns` への置換は `linalg` の既存テストと
  `least_squares` の呼出し形を全て書き換える diff を生む
  (pre-release のため許容)
* Bad, because `LeastSquares` が行を保持しなくなるため、行単位の
  観測を直接必要とする将来の消費者は列へ再構成が必要になる
  (現状そのような消費者は存在しない)

### Confirmation

* `LeastSquares::from_columns` が存在し、`mixer.rs` の
  per-sample 行 `Vec` 構築と `linalg.rs` の行再収集・行 clone が
  コード検査で除去されていることを確認する
* 畳み込みが `al_locate::dsp` 経由で到達可能で、`reference.rs`
  の `fir_model` が直接畳み込みループを持たないことをコード検査
  で確認する
* `dsp::fft` のシグネチャと single-shot caller が不変であること、
  `stft` ループ内に `Fft::new` がないこと(フレームバッファの
  `vec![0.0; fft_size]` 確保は出力行として残る)をコード検査で
  確認する
* characterization テスト・等価性ゲート(A3 ビット一致、A1/A4
  述語)・bench delta 記録の存在を spec の Acceptance Criteria で
  確認する

## Implementation Plan

* Affected paths:
  `crates/al-math/src/linalg.rs`(`from_columns` 追加・内部列正本
  化・`new`/`least_squares` の列置換・`Apply` の RSS を
  `sub_scaled`/`sum_sq` 化)、
  `crates/al-locate/src/mixer.rs`(`fit_mix` シグネチャ変更・
  行マテリアライズ除去・AᵀA を列 `dot` 集計へ)、
  `crates/al-locate/src/residual.rs`・`stage.rs`・`lib.rs`
  (caller 移行・mirror 確認。`subtract_fit`/`prepare_residual`
  の `&[Vec<f64>]` 公開シグネチャは維持し内部で列参照へ変換
  — `lib.rs` re-export 面と `stage.rs:947`・
  `tests/locate.rs:300`・`residual.rs` テスト `:269,306` は
  無変更)、
  `crates/al-locate/src/dsp.rs`(畳み込み関数追加・`Fft` plan
  公開・`:114` の `least_squares` テストを列シグネチャへ追従)、
  `crates/al-analysis/src/reference.rs`(`fir_model` の畳み込み
  置換と名前付き関数への切り出し・`subtract` の zero-fill 修正・
  `.to_vec()` 除去)、
  `crates/al-analysis/src/stft.rs`(plan 巻き上げ)、
  `crates/al-analysis/Cargo.toml`(criterion `=0.8.2` dev-dep と
  `[[bench]] name = "..." harness = false` stanza — al-math
  `Cargo.toml:16-22` と同 pattern)・
  `crates/al-analysis/benches/`(A1 畳み込み・A3 stft の
  criterion bench)・
  `crates/al-analysis/examples/bench_reference.rs`(stft・
  fir_model 経路の stage 級 Instant ベンチ新設 — 既存
  al-locate `bench_*` examples はこれらをカバーしない)、
  `crates/al-math/benches/`(A4 lstsq の kernel 級 bench、既存
  dev-dep を利用)
* Patterns to follow: `filter.rs` `bandpass_many` の plan 再利用
  (1 plan を forward/inverse で使い回す)、`residual.rs` の
  per-source `sub_scaled` 残差先例(「ADR の recomposition
  tolerance 内」と明示して実装済み)、`LeastSquares::new` の
  Option 返却契約、`dsp::fft` の手書き wrapper 形
* Patterns to avoid: `dsp::fft` シグネチャの破壊、al-analysis
  への al-math 依存追加、行・列の二重表現の存置、al-math
  カーネルの変更、characterization テスト未作成での最適化適用、
  `Apply` の RSS を QR 尾部から取る実装(ridge 項が混入し
  `residual_rms`/`fit_quality` の意味が変わる)
* Sequencing: `subtract` の zero-fill 修正は characterization
  コミットに同梱し、A1 最適化の適用(等価性ゲート実行)より先に
  land させる。当該分岐は Aligned shared-length 契約
  (al-analysis `stage.rs:283-295`)の下で本番到達不能のため
  捕捉出力への数値影響はないが、変更順序を記録上明確にする
* Dependencies or configuration: `crates/al-analysis` に
  `criterion = "=0.8.2"` dev-dep(al-math と同 version・
  pattern、新しい依存カテゴリではない。依存追加として
  Implementation Record に記録)と `[[bench]] harness = false`
  stanza(al-math `Cargo.toml:16-22` と同 pattern)。stage 級
  計測は manifest `duration_ms` と新設の
  `examples/bench_reference.rs`(Instant ベンチ規約)を使用

## Verification

* [ ] `LeastSquares::from_columns(&[&[f64]], λ)` が公開 API とし
  て存在し、`new` 相当の契約(等長・有限・観測数≥列数・非負
  有限 λ → `None`)を列向きに持つテストが pass する
* [ ] `LeastSquares` が行セットを保持せず ridge 適用前の入力列を
  1 コピーのみ保持し、残差 RSS がその列上の
  `sub_scaled`/`sum_sq` で計算されている(コード検査 + 既存
  解精度テストの継続 pass。RSS は ridge 項を含めない現行契約
  `linalg.rs:38,122-128` を維持する)
* [ ] `mixer.rs` の per-sample 行 `Vec` マテリアライズがなく、
  `fit_mix` 呼出し全箇所(`reference.rs` ×3・`residual.rs`・
  `stage.rs`)が列指向シグネチャを使用している
* [ ] `fir_model` の畳み込みが dsp 経由の FFT 畳み込みに置き
  換わり、`fir_model` が名前付き関数として test から到達可能、
  直接畳み込み参照実装が test/bench コード内に存在して固定
  fixture で述語 `|a-b| <= 1e-12*max(1,|a|,|b|)` を満たす
  テストが pass する
* [ ] `subtract` の `m[i.min(m.len())]` が
  `m.get(i).copied().unwrap_or(0.0)` の zero-fill に置き換わり、
  ragged チャネル fixture(契約違反経路)で panic しない検証
  テストが存在する — 本番経路は Aligned shared-length 契約
  (al-analysis `stage.rs:283-295`)により到達不能の構造的防御
* [ ] `stft` の出力が固定の決定的 fixture(シード固定
  synthetic 入力 + `n < window`・単一フレーム・非2冪 window・
  全ゼロのエッジケース)で変更前後ビット一致するテストが
  pass し、`dsp::fft` と single-shot caller が不変である
* [ ] `cargo tree -p al-analysis --depth 1`(または Cargo.toml
  直接確認)で al-math 直接依存がない
* [ ] `mise exec -- cargo test -p al-math -p al-locate -p
  al-analysis`・`cargo test --workspace`・`cargo clippy`・
  `cargo fmt --check` が green で、golden テストが pass する
* [ ] kernel 級 criterion bench と stage 級代表経路の
  before/after 実測が Implementation Record に記録されている

## Pros and Cons of the Options

### `from_columns` 追加 + rows API 置換

* Good, because 行マテリアライズ・strided gather・行 clone が
  全て消え、lstsq の内部正本が列に統一される。呼出し側も
  チャネルコピーが不要になる
* Bad, because `linalg` の既存テストと `least_squares` 呼出し形を
  書き換える diff が生じる

### `from_columns` 追加 + rows API 両建て

* Good, because 既存呼出しに波及しない
* Bad, because 行・列の二重表現と二重 validation を保守し続け、
  行 clone も残り抜本的解消にならない。pre-release 方針にも反する

### 現行 rows API 維持 + fit_mix 内バッファ再利用

* Good, because API 変更不要で diff が最小
* Bad, because `Vec<Vec<f64>>` の1回分構築と内部 gather/clone が
  残り、~8.6M alloc の主因を解消できない

### overlap-save(dsp 側畳み込み関数)

* Good, because block FFT が有界でメモリ・キャッシュ効率がよく、
  conv_len キャップと相性がよい。dsp 集約で caller は1行呼出し
* Bad, because block ループ・境界処理・尾部打ち切りの実装を要し、
  全長 spectral path より複雑

### 全長 spectral path(ncc/bandpass_many 型)

* Good, because 既存パターンと同形で実装が単純
* Bad, because n ≈ 4.5M で next_pow2 換算 ~8M 要素の単一変換と
  複数本の巨大バッファが必要で、メモリ・キャッシュコストが大きい

### direct conv 維持のまま SIMD 化

* Good, because 丸め差が小さい
* Bad, because O(n×4096) の計算量自体が残り効果が限定的。
  kernel 側変更でスコープ(kernels untouched)にも違反する

### dsp に plan 公開経路を追加(`dsp::fft` 維持)

* Good, because 公開 wrapper の契約を守りつつ stft が plan を
  使い回せ、ファサード一元化を維持できる
* Bad, because dsp の公開面に plan キャリア(Fft 型または薄い
  wrapper)が増える

### `dsp::fft` を plan 受取に変更

* Good, because 実装は素直
* Bad, because 公開 wrapper の契約破壊で single-shot caller 全てに
  波及し、ビット一致ゲートの比較軸も崩れる

### stft が al_math を直接参照

* Good, because plan 制御が素直
* Bad, because al-analysis→al-math 依存追加が発生し、dsp
  ファサード一元化の規約を破る

## More Information

* 確定経緯・finding(file:line)・選択肢比較:
  `docs/discovery/caller-side-optimizations.md`
* 実施スコープ・受入基準・プロセス契約:
  `docs/specs/caller-side-optimizations.md`
* 数値ポリシーの権威: `docs/adr/simd-reduction-lane-fma.md`
  (ADR-034EoHxzsvsHp9BOdCK2Cn)。A1/A4 の許容誤差はその
  recomposition tolerance 述語を適用する。数値ポリシー専用の
  第2 ADR は作成しない — 述語が既に確立しており、本チェーンの
  新規決定は API・構造面のみのため(対象チェーンが異なる別の
  懸念は A2 plan 再利用等、別チェーンで扱う)
* 前提決定: `docs/adr/al-math-vec-primitives-slice-boundary.md`
  (dsp ファサード消費経路・等長 assert 系の panic/Option 契約の
  系)、`docs/adr/al-math-api-fft-split-re-im.md`(公開面の
  バックエンド中立化)、`docs/adr/spectral-envelope-fft-size-
  assert.md`(入口契約の前例)
* 関連: issue #42(DSP カーネル SIMD 化)— カーネル内部と
  A2 ncc plan 再利用・A5 local_lag→ncc は本 ADR のスコープ外
