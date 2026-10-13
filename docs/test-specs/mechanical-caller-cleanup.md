---
id: "TSPEC-034EO3X1gkbTna1zfQoEG8"
type: "test-spec"
status: "approved"
title: "機械的 caller クリーンアップ検証仕様"
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
    - "docs/designs/mechanical-caller-cleanup.md"
  derived-by: []
  verifies:
    - "docs/specs/mechanical-caller-cleanup.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 機械的 caller クリーンアップ検証仕様

## Purpose

`docs/specs/mechanical-caller-cleanup.md` の Acceptance
Criteria を検証可能な形に分解した検証仕様。機械的 caller
クリーンアップ・バンドル(A6 bandpass バッチ化・A7/A8
`csd` インライン化・A9 `power()` 一括化・A12 HashSet 事前
計数・A15 interleave/deinterleave ブロック化・A16a–d 行優先
累積/`dom()` scratch/video 融合/tensor fast path)が
「characterization テスト先行 + bench baseline/事後計測 +
Implementation Record 記録」のプロセス契約の下で、**全項目
bit-identical**(A16d は返却 `(out, region_shape)` の完全
一致)の等価クラスと公開・依存境界の不変を維持しながら
適用されたことを保証する。数値不変の権威は
`docs/adr/simd-reduction-lane-fma.md`
(ADR-034EoHxzsvsHp9BOdCK2Cn)だが、本バンドルでは
tolerance predicate `|a-b| <= 1e-12 * max(1,|a|,|b|)` を
一度も適用しない — 等価性は `f64::to_bits` / バイト列 /
返却値の完全一致で判定する。

各 characterization テストは対応する最適化コミットの前後
両時点で green であることを意図し、最適化適用後も継続して
残る。廃止してよいのは、該当する最適化が revert され、
かつ characterization が捕捉していた現行意味そのものが
仕様変更により消滅した場合のみ。

## Feature

al-analysis / al-locate / al-media / al-dataset の caller 側
機械的 cleanup が、変更前後で捕捉済みの振る舞い契約を
守ること — 永続化される値(record スカラー・tensor 要素・
`.f64le` バイト列)と非永続の中間量(下流 record 経由)の
等価性、公開シグネチャと依存境界の不変、計測基盤の存在と
baseline/事後比較の記録。

## Rules

### Rule A: characterization-first のコミット順序と捕捉粒度

- Rule: 各最適化の適用コミットに先行して、(1) 現行出力を
  捕捉する characterization テストが存在し pass すること、
  (2) 該当する criterion / Instant ベンチが変更前に
  baseline 計測されていること。この順序は git 履歴(char
  コミットが対応する opt コミットより先に land している
  こと)で確認可能でなければならない
- Rule: persisted 値(record スカラー・tensor 要素・
  `.f64le` バイト列)は `f64::to_bits` / バイト列の完全
  一致を決定的 synthetic 入力の出力へ適用する。期待値は
  `al-testkit` fixture 経由で再現可能な値に限り、magic
  literal のハードコードは禁止し、新旧同一経路の A/B 形
  (oracle をテスト内に残す)または record 同士の比較とする
- Rule: 非永続の中間量(A7 の累積器)は、その結果が流入
  する下流 record(coherence・h・band gain・
  `stereo.coherence_mean`)の `to_bits` で間接捕捉する
- Rule: 既存カバレッジ(`dsp.rs:196` deinterleave 単体
  テスト、`tensor.rs` read_range 単体テスト、`locate.rs`
  normalized_pcm layout テスト群、各 `tests/golden.rs` の
  シナリオ pin)は回帰として効くが、現行出力の値 pin
  ではないため spec 契約上の characterization とは別に
  追加する
- Rule: characterization コミットと bench 腕は既存公開 API
  のみで compile する(最適化コードを含まない)

### Rule B: A6 — band_correlation の bit-identical

- Rule: `reference.rs` の band_correlation tensor 全要素が
  変更前後で `to_bits` 一致する。`bandpass` は
  `bandpass_many(.., &[(lo, hi)])` `.pop()` の thin
  wrapper であり、mask 規則・切り戻し・出力順(bands 順)
  が同一であることに等価の根拠を持つ
- Rule: `bands < 8` の edge(低 sample rate)で `.min(8)`
  bound が維持されること。`&bands[..8]` リテラルは禁止
- Rule: コード検査の不変条件として「`bandpass_many` 呼出し
  が src/cap でちょうど 2 回」「`reference.rs` に
  `bandpass(` 呼出しが残らない」「`bandpass` import が
  `bandpass_many` へ入替済み」が保持される

### Rule C: A7/A8 — 累積器・hft の bit-identical

- Rule: `csd` は要素式 `conj(x)·y` = `(xr*yr + xi*yi,
  xr*yi - xi*yr)` を返す。各 caller は累積ループ内で同じ
  式を inline 計算し、`c[k]` 経由の読みを除去する。
  加算順(bin 昇順・frame 昇順)と各サイトの auto-power
  式(`powi(2)`・`r*r+i*i`)は現行のまま維持する
- Rule: 列挙 7 サイト(`reference.rs:183`・`:539-545`・
  `:872-877`・`band_gain.rs:159`・`cheap.rs:1105-1120`・
  `expensive/channel.rs:102-118`・A8 `reference.rs:587-593`)
  に `csd(` 呼出しが残らず、対応する consumer file
  (`reference.rs:18`・`cheap.rs:16`・`band_gain.rs:16`・
  `expensive/channel.rs:9`)の `csd` import が prune 済み。
  `csd` 関数自体は public のまま残す(`stft.rs:390` の
  `#[cfg(test)]` が使用)
- Rule: 下流 record(h・coherence・band gain・
  `stereo.coherence_mean`)と hft tensor 全要素が `to_bits`
  一致する。`cheap.rs` stereo 経路の frames cap
  `min(64)` と A8 の `xs` guard・else-`0.0` は保持する

### Rule D: A9 — `power()` 一括化導出の bit-identical

- Rule: `cheap.rs` の magnitude/power/psd 派生は
  `m.power()` 1 回の pass から導出し、magnitude =
  `power[k].sqrt()`、psd = `power[k] * (1.0/norm)`(norm =
  `(0.375 * window) * sample_rate`)とする。`/ norm`
  リテラルは禁止(ulp 差) — 現行 `psd()` の
  `scale(&power, 1.0/norm, …)` と同じ乗算のみ許容
- Rule: 全導出と `m` field の読みは
  `channel_stft.push(Some(m))`(`cheap.rs:665`)より前に
  完了し、`cheap.rs:668` の `let half = mag[0].len()/2`
  重複定義は push 前の `half` 再利用へ置き換える
- Rule: `mean_row`(`cheap.rs:794-796`)は row-major pass
  へ融合し、加算対象(`sqrt` 済み要素)・加算順(row
  昇順、k<half)・除算(`mag.len()`)が現行と同一で
  bit-identical。`spec.slope` 消費は不変
- Rule: `StftMatrix` に新規メソッドを追加しない。
  単一消費者例外 `cheap.rs:942` `mm.magnitude()`・
  `reference.rs:1189` `rm.psd()` は変更しない

### Rule E: A12 — quant_step_est の record 級 bit-identical

- Rule: `cheap.rs:577-603` の distinct count は
  `HashSet<u64>`(`v.to_bits()`)で事前計数し、
  `2 <= distinct.len() <= 1_000_000` の in-gate のみ
  `f64::from_bits` 実体化 → `total_cmp` sort →
  `windows(2)` 最小正差を実行する。gate 外では u64
  sort・f64 実体化・f64 sort のすべてを skip する
- Rule: support は `distinct.len()`(現行 `uniq.len()` と
  同値)を使い、`dig.quant_step_est` の value・support・
  `Validity` が record-level で一致する。近似 distinct
  count は禁止
- Rule: characterization は in-gate(2〜1M)と >1M
  distinct skip の**両経路**を別 fixture で pin する

### Rule F: A15 — interleave/deinterleave のバイト・要素完全一致

- Rule: `normalize.rs:168-171` は frame ごとに channel
  サンプルを再利用 row scratch へ詰めて 1 回
  `extend_from_slice` し、`Vec::with_capacity(frames*ch*8)`
  で事前確保する。生成される `.f64le` バイト列は完全一致
- Rule: `reference.rs:1163-1170` は `n*channels` 事前確保の
  indexed write(frame-major)で、interleave 要素列は完全一致
- Rule: `dsp.rs:17-29` は `chunks_exact(channels)` による
  channel-block strided gather で、返却 `Vec<Vec<f64>>` の
  各 channel 列は完全一致(trailing partial frame drop を
  含む)。frame-major の配置順を両方向で維持し transpose
  しない

### Rule G: A16 — 行優先累積・dom scratch・video 融合・tensor fast path

- Rule: `mean_psd_*`(`reference.rs:655-660`)・
  `mean_power`(`expensive/mod.rs:648-651`)・`mean_row`
  は scalar row-ascending 累積(0.0 始まり・row 昇順・
  `k` 昇順・最後に `/ frames` または `/ len`)を維持し、
  vec reduction primitive へ置き換えない(加算順が変わり
  tolerance-class になる)。`frames = ms.frames().min(
  mc.frames())` の `.take(frames)` 防御 bound は保持する。
  `reference.rs:653-654` の `psd_s`/`psd_c` は
  `power()` 行 + `row[k] * inv` へ置き換え psd 行列を
  実体化しない
- Rule: `dom()`(`reference.rs:1071-1084`)は呼出しごとに
  再利用する fft_size 長 scratch へ `mag_sq` を呼び、
  `k < fft_size/2` で `scratch[k].sqrt()` を累積し、
  `skip(1)`・`total_cmp` の argmax は不変。手書きの
  `re*re + im*im` は禁止(mag_sq → simd lane/tail の
  位置依存丸めを再現できない)、`power()` 再利用も禁止
  (argmax(Σ√x) ≠ argmax(Σx))。`mag_sq` は
  `al_locate::dsp` facade 経由のみ
- Rule: video luma/diff(`video.rs:402-441`)は 1 pass の
  別 accumulator(`luma_sum`/`diff_sum`)へ融合し、各和の
  加算順(昇順)は不変。diff 側は現行 `zip` の
  silently-truncate semantics を `p.get(i)` で再現し、
  `p[i]` index 化は禁止。先頭 frame の `prev == None` は
  `0.0` リテラル、`pixels == 0` では `0.0/0.0 = NaN`
  系列を現行通り出力する。ROI 和(`:413-424`)は対象外
- Rule: tensor `read_range`(`tensor.rs:271-289`)の fast
  path は、行頭(`local[ndim-1] == 0`)で outer dims が
  region 内かつ最内 extent が region 最内区間へ**完全包含**
  される場合のみ `copy_from_slice` で行コピーし、部分包含
  を含む他は現行 per-element gather へ fallback する。
  返却 `(out, region_shape)` は両経路で完全一致

### Rule H: ベンチ基盤・baseline・回帰 bounds

- Rule: `crates/al-analysis` に `[[bench]] name =
  "band_correlation" / "csd_accum" / "quant_step" /
  "spectral_pass" harness = false` stanza と対応する
  `benches/` ファイルがあり、各腕は既存公開 API のみで
  A/B 比較を自己完結できる(band_correlation: 8×bandpass
  loop vs bandpass_many、csd_accum: csd 呼出し vs inline、
  quant_step: double-sort vs HashSet gate、spectral_pass:
  mag_sq pass 数・行優先累積・dom scratch)
- Rule: `examples/bench_reference.rs` + manifest
  `duration_ms` が A6・A9・A16a/b の stage 級代表経路を
  計測する。al-locate `examples/bench_dsp.rs` は A15 の
  deinterleave/interleave が経路上にあれば流用、なければ
  Instant 区間追加に留める
- Rule: al-dataset は `tests/integration.rs` 内の Instant
  micro-timing、al-media は既存 example/test の Instant
  区間、なければ code inspection + workspace bench 回帰
  ゲート + manifest `duration_ms` とし、採用方式を IR に
  明記する。新規 dev-dep は追加しない
- Rule: baseline は `cargo bench --save-baseline` と
  example 出力の `target/bench-baseline/` 記録(コミット
  対象外)を変更前に採取し、変更後に同一条件で再測して
  before/after を `docs/impl/ir/` へ記録する。測定対象外
  経路に ±5% を超える回帰がないこと

### Rule I: 公開契約・依存境界の不変

- Rule: `bandpass`/`bandpass_many`/`csd`/
  `StftMatrix::{psd,magnitude,power}` の公開シグネチャと
  永続化フォーマット(normalized `.f64le` バイト列・
  tensor store・record/artifact schema)・record 意味は
  不変。新規公開 API を追加しない
- Rule: al-analysis に al-math 直接依存を追加しない —
  `cargo tree -p al-analysis --depth 1`(または
  `Cargo.toml` 直接確認)で判定する。`mag_sq`・
  `bandpass_many`・`scale` は `al_locate::dsp` facade 経由
  のみ
- Rule: 新規依存・al-dataset/al-media への新規 dev-dep を
  追加しない(`HashSet` は std)
- Rule: golden テスト(各 `tests/golden.rs`)が pass を
  維持し、golden 判定意味・Validity 閾値は変更しない

## Examples

- Example(commit 順序): `git log` で char コミット(全
  characterization + 新規 criterion arm)が A6-opt・
  A7A8-opt・A9A12-opt・A15-opt・A16-opt の各コミットより
  先にある
- Example(A6): 決定的入力で `band_correlation` を実行し、
  各 band の corr 要素が `to_bits` 一致。低 sample rate で
  bands が 8 未満になる fixture でも panic せず `.min(8)`
  bound が効く
- Example(A7): `expensive/channel.rs` の `sxy` 経路で、
  `band_bins` 内の bin のみ inline 積を計算し、下流の
  coherence record が `to_bits` 一致
- Example(A8): hft 構築で長さ 1 の `csd` 呼出しがスカラー
  `(cr, ci)` + `((cr*cr + ci*ci).sqrt()) / xs` へ置き換わり、
  hft tensor 全要素が `to_bits` 一致
- Example(A9): `cheap.rs` で `m.power()` 1 回から
  `mag`(sqrt)・`psd`(`* inv`)・`mean_row`(融合)を
  導出し、`spec.slope` record が `to_bits` 一致。
  `grep` で `/ norm` リテラルが cheap.rs 派生経路に存在
  しない
- Example(A12): in-gate fixture(例: 量子化 step を持つ
  決定的波形)と >1M distinct fixture(例: 高分解能
  ノイズ)の両方で `dig.quant_step_est` の value・support・
  `Validity` が一致
- Example(A15): normalized 出力の `.f64le` バイト列が
  byte-exact に一致し、`deinterleave` の返却列と residual
  interleave の要素列が完全一致
- Example(A16a): `mean_psd_s`/`mean_psd_c`/`mean_power`・
  `band_gains`/`psd_ratio`/`total_power`/`one_sided_power`
  が `to_bits` 一致
- Example(A16b): `dom()` が `mag_sq` scratch を使い、
  `ref.harmonic_freq_ratio` record が一致し、コード検査で
  手書き `re*re+im*im` がない
- Example(A16c): luminance 系列と diff 系列が `to_bits`
  一致し、先頭 `diff[0] == 0.0`、0 画素入力で `NaN`
  系列が出る
- Example(A16d): innermost dim が完全包含される region で
  fast path、部分包含 region で fallback、それぞれ
  `(out, region_shape)` が一致
- Example(bench): `cargo bench -p al-analysis
  -- --save-baseline <name>` で baseline 保存、opt 後に
  `--baseline <name>` で比較。`cargo run --example
  bench_reference --release` の出力を
  `target/bench-baseline/` へ保存

## Guarantees

- 各最適化が「捕捉済みの現行出力を壊さない」ことを、
  テストの存在と git 履歴の両面で検証可能な形で保証する
- 全項目の等価性が bit-identical(`to_bits`/バイト列/
  返却値完全一致)で判定され、tolerance predicate が
  一度も適用されないこと
- A6/A7/A8/A15 の import prune と配置変更が、公開面
  (`bandpass`・`csd`・deinterleave 返却型)を変えないこと
- A9 の caller 側導出が `* (1.0/norm)` 形のみ・`:665`
  move 前完了・`StftMatrix` 新規メソッドなしを維持すること
- A12 の support が `distinct.len()`(== `uniq.len()`)で
  あり、両経路が pin 済みであること
- A16 の累積が scalar row-ascending のまま・`dom()` が
  `mag_sq` 呼出しで lane/tail 丸めを再現・video が
  zip の truncate semantics・degenerate 分岐を保持・
  tensor fast path が完全包含に限定されること
- 計測基盤(bench harness・baseline 手続き・IR 記録)が
  再現可能で、回帰判定(±5%)の根拠が残ること
- 公開シグネチャ・dsp ファサード消費経路・golden 判定
  意味が変わらないこと

## Non-goals

- 非永続中間量(累積器)の直接 to_bits — 下流 record での
  間接捕捉で代替する(design 確定の捕捉粒度)
- `mean_psd`/`mean_power`/`mean_row` の vec reduction 化の
  検証(tolerance-class、本バンドルでは scalar 維持が契約)
- al-math カーネル内部・`stft.rs` の `csd`/`psd`/
  `magnitude`/`power` 実装の等価性(変更しない)
- 単一消費者例外(`cheap.rs:942`・`reference.rs:1189`)・
  ROI 和(`video.rs:413-424`)の最適化
- A2 `ncc` plan 再利用・A5 `local_lag`→`ncc`・A10/A13/A14・
  envelope prefix-sum・`matrix_to_tensor` 統合・
  residual-gather/lod 系(別チェーン/継続 defer)
- criterion ベンチの CI 回帰ゲート(CI 基盤未整備、先行
  チェーンと同じく整備時に再検討)
- 新規 dev-dep を用いた al-dataset/al-media の bench harness

## Risk

- この保証を失うと、機械的 cleanup の「bit-identical」主張
  が検証不能になり、ulp 級の数値ドリフトが
  `harmonic_freq_ratio`・`quant_step_est`・feature record 等
  の永続化値・golden 判定へ混入する。
  characterization-first 順序の喪失は「変更前出力の捕捉」
  という検証軸そのものを失わせ、bench baseline の欠落は
  回帰判定を印象論に退化させる。A16b で手書き
  `re*re+im*im` が混入すると SIMD lane/tail の位置依存
  丸めがずれて検出不能な差分になり、A16d の境界判定誤り
  は部分包含 region での読み出し破壊に直結する
