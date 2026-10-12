---
id: "DISC-034EMosQizFcChc9KKeaWS"
type: "discovery"
status: "active"
title: "機械的 cleanup 最適化バンドル: bandpass バッチ化・csd インライン化・アロケーション削減 (A6-A16)"
created: "2026-10-12"
updated: "2026-10-13"
owners: []
relations:
  source:
    - "../../crates/al-analysis/src/reference.rs"
    - "../../crates/al-analysis/src/stft.rs"
    - "../../crates/al-analysis/src/cheap.rs"
    - "../../crates/al-analysis/src/band_gain.rs"
    - "../../crates/al-locate/src/normalize.rs"
    - "../../crates/al-media/src/video.rs"
    - "../../crates/al-dataset/src/tensor.rs"
    - "../../crates/al-math/src/filter.rs"
    - "../../crates/al-math/src/vec.rs"
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
    - "./caller-side-optimizations.md"
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
  derives-from:
    - "./caller-side-optimizations.md"
  derived-by:
    - "../specs/mechanical-caller-cleanup.md"
    - "../designs/mechanical-caller-cleanup.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 機械的 cleanup 最適化バンドル: bandpass バッチ化・csd インライン化・アロケーション削減 (A6-A16)

## Exploration Goal

caller 側最適化チェーン(A1/A3/A4、`caller-side-optimizations.md`)が
main にマージされた。残る performance survey の finding のうち、
設計判断を要さない「機械的 cleanup」部分集合(A6/A7/A8/A9 派生項/
A12/A15/A16)を次の実装チェーンとして確定する探索。

ユーザー制約: **設計を変えない・汎用性を下げる過度な特化をしない**。
全項目で API シグネチャ・永続化フォーマット・公開構造を維持し、
数値出力はビット一致、または既存の recomposition tolerance ポリシー
(述語 `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`、
ADR-034EoHxzsvsHp9BOdCK2Cn `docs/adr/simd-reduction-lane-fma.md:109`)
の範囲内に収める。

本ドキュメントは各 finding を現在のコード上の証跡(file:line)とともに
記録し、等価性クラス(bit-identical / tolerance-covered)を項目ごとに
固定して、後続の spec/ADR/design の入力とする。survey 時点の行番号は
A1/A3/A4 マージでずれているため、本記録の行番号は現 HEAD で再検証済み。

## Key Issues and Assumptions

### A6 — band_correlation の 16 回単発 bandpass

- 確認済み(実装): `reference.rs:742-747` が
  `bands.iter().take(8)` で 8 帯域を回し、各帯域で
  `bandpass(src_mono, rate, lo, hi)` と `bandpass(cap_mono, …)` を
  呼ぶ — 計 16 回の単一帯域 bandpass
- 確認済み(内部コスト): `bandpass` は `bandpass_many` へ委譲するだけ
  (`filter.rs:7-11`)。`bandpass_many` は呼出しごとに
  `RealFft::new(n)` plan 構築(`filter.rs:37`)+ next_pow2 パディング
  (`:36-39`)+ forward FFT(`:42`)を行い、帯域ごとに inverse
  (`:61`)する。単一帯域呼出しでは plan/forward が 16 回重複する
- 確認済み(batch 化後の形): `bandpass_many(src_mono, rate, &bands[..8])`
  + `bandpass_many(cap_mono, …)` で plan 2 個・forward 2 回・
  inverse 16 回。当該セクションで ~2× を見込む(実測で確認)
- 確認済み(等価性): **bit-identical**。`bandpass` が
  `bandpass_many` の薄い wrapper であるため、同一入力・同一 mask
  規則(`lower_bin` `filter.rs:51-54`)・同一 inverse・同一切戻し
  (`real[..len].to_vec()` `:62`)で変換数学は完全に同一

### A7 — csd の per-frame Vec アロケーション

- 確認済み(実装): `csd`(`stft.rs:173-179`)は呼出しごとに
  `Vec<(f64,f64)>`(bins 長)を返す。要素式は
  `(xr*yr + xi*yi, xr*yi - xi*yr)` = `conj(x)·y`(`stft.rs:176`)
- 確認済み(call site — fir_model は A1 で `conv` 化済みのため現行の
  csd 呼出しを再列挙):
  `reference.rs:183` `fir_ir`(fir_model 経路内、frame ごとに bins 長
  Vec)、`reference.rs:539` Welch 累積、`reference.rs:872` L/R
  coherence、`band_gain.rs:159` `coherence()`(frame ごと、~4.2K
  frames × 32KB 級)
- 確認済み(先行パターン): インライン scalar 累積の先例が既に存在
  — `reference.rs:541-542`/`585`(mag_sq の手書き `re*re + im*im`)、
  `reference.rs:185`/`874-875`(同)。各 call site は返り値 Vec を
  その場で要素ごと累積するだけなので、`c[k]` を
  `(xr*yr + xi*yi, xr*yi - xi*yr)` の scalar 式に置き換えるだけで
  中間 Vec が消える
- 確認済み(等価性): **bit-identical**。同一式・同一加算順を bin
  ごとに直接累積するだけで、Vec 経由の有無は丸めに無関係

### A8 — hft の (frame, bin) ごと 1 要素 csd

- 確認済み(実装): `reference.rs:581-598` の `hft` 構築が bin ごとに
  `csd(&ms.re[f][k..k+1], &ms.im[f][k..k+1], &mc.re[f][k..k+1],
  &mc.im[f][k..k+1])`(`:587-592`)を呼び、長さ 1 の Vec を作っては
  `c[0]` だけを使う。frames × half(bins/2)回 — 4.2K frames × 2048
  bins 規模で ~8.6M 個の微小 heap alloc
- 確認済み(等価性): **bit-identical**。`(cr, ci)` を scalar で
  計算し `((cr*cr + ci*ci).sqrt()) / xs` とすれば現行の
  `c[0]` 利用式(`:593`)と同一

### A9(派生項のみ)— power/magnitude/psd の三重 walk

- 確認済み(実装): `StftMatrix::psd`(`stft.rs:48-64`)・
  `magnitude`(`:68-78`)・`power`(`:81-89`)がそれぞれ
  frames×bins を walk して `mag_sq`(`vec.rs:274` 経由)を計算する
- 確認済み(call site): `cheap.rs:632-634` が `m.magnitude()`・
  `m.power()`・`m.psd()` の 3 行列を全て materialize(mag → envelope
  `:658`、pow → band_power `:645,:654`)。ほか `reference.rs:653-654`
  (psd 2 行列)、`reference.rs:1072`/`cheap.rs:942`(magnitude)
- 確認済み(等価性): **bit-identical**、ただし式の保持が条件。
  magnitude は `sqrt(power[i])`(`stft.rs:75` が mag_sq 結果へ
  `v.sqrt()`)、psd は `power[i] * (1.0/norm)` — 現行は
  `scale(&power, 1.0/norm)`(`stft.rs:58-61`)の**逆数乗算**なので、
  派生側も同じ式(または `scale` プリミティブ再利用)を使うこと。
  `power[i] / norm` の除算リテラルは逆数乗算と ulp 級で異なり得る
  (その場合も tolerance 内だが、bit-identical 側に寄せる)。
  `norm = window_power() * sample_rate`(`:51`)で `window` /
  `sample_rate` / `fft_size` は pub フィールド(`:18-24`)のため
  caller 側派生も API 変更なしで可能
- 確認済み(スコープ限定): 適用は「1 回の mag_sq pass から
  power/magnitude/psd を派生」のみ。**除外**: envelope の prefix-sum
  化(加算順変更による cancellation/精度リスク)、
  `matrix_to_tensor` 統合(永続化ストレージ形式の変更) — いずれも
  設計判断を伴うため本バンドルの「機械的」範囲外

### A12 — uniq の二重ソート

- 確認済み(実装): `cheap.rs:577-603`。`uniq` に全サンプルの
  `to_bits` を集め(`:577`)→ `sort_unstable`(`:578`)→ `dedup`
  (`:579`)→ gate `uniq.len() >= 2 && <= 1_000_000`(`:580`)を
  通る場合のみ f64 へ戻して `sort_by(total_cmp)`(`:581-582`)し
  min positive diff を計算。`uniq.len()` は `dig.quant_step_est` の
  support として記録される(`:603`)
- 確認済み(改善): HashSet で distinct count を先に取れば、count が
  gate 外(実運用では >1M 側)のとき両ソートと f64 への materialize
  を全て skip できる。`set.len()` は dedup 後の `uniq.len()` と
  同値。gate 内の場合のみ collect→sort を実行する
- 確認済み(等価性): **bit-identical**(record 値レベルで identical)。
  quant は同一集合の同一ソート順から算出、support も同じ count

### A15 — per-sample interleave

- 確認済み(実装1): `al-locate/src/normalize.rs:168-171` —
  frame ごと × channel ごとに
  `bytes.extend_from_slice(&channel[frame].to_le_bytes())` で
  8 バイト単位の extend(~17M 回規模)。出力は frame-major
  interleave の f64le バイト列
- 確認済み(実装2): `reference.rs:1163-1170` —
  `v.push(ch[i])` の sample ごと push で residual チャネルを
  interleave(capacity は `:1165` で事前確保済み)
- 確認済み(等価性): **bit-identical**。出力バイト列/要素列は
  interleave 順序が同一なら完全一致。strided/chunked 書出し
  (per-frame の staged row 化、index 書込み、チャネルブロック単位の
  `extend_from_slice` 等)は配置問題であり値を変えない

### A16 — 小粒項目(各々個別に検証済み)

- mean_psd 列走査: `reference.rs:655-660` が bin ごとに
  `psd_s.iter().take(frames).map(|r| r[k]).sum()` で strided に列を
  集計(cache unfriendly)。行優先 `for row { mean[k] += row[k] }` へ
  — per-bin の加算順(行昇順・0.0 起点)を保つ scalar 形なら
  bit-identical、vec reduction プリミティブへ載せる場合は加算順が
  変わり ulp 級。**本バンドル唯一の tolerance-covered 許容項目**
  (ユーザー指定: ulp-level 変更は mean_psd のみ)
- dom() の magnitude 行列: `reference.rs:1071-1084` が argmax のため
  だけに `m.magnitude()` 全行列(per-element sqrt 込み)を
  materialize。行を walk しながら `sqrt(re²+im²)` を `mean[k]` に
  直接累積すれば同一加算順で **bit-identical**、行列 alloc が消える。
  `power()` の再利用は不可(argmax(Σ√x) ≠ argmax(Σx) — sqrt が
  総和の内側にあるため)
- video luma/diff: `al-media/src/video.rs:402-441` が frame ごとに
  `frame.luma` を複数 pass する — ROI 和(`:413-424`)、全画素
  luminance 和(`:427`)、prev との diff zip-sum(`:428-439`)。
  luminance 和と diff 和は同一イテレーション順の別 accumulator
  なので 1 pass へ fuse しても各和の加算順は不変 → **bit-identical**
- tensor `read_range`: `al-dataset/src/tensor.rs:271-289` が要素ごとに
  `out[dst] = chunk_data[src]`(`:283-285`)を gather。最内 dim が
  region 内に完全収まる行では src/dst とも連続なので、その連続 run を
  スライスコピー(`extend_from_slice` 系)する fast path を追加 —
  返却データは同一要素・同一順序で **identical**

### 計測・検証基盤(既存、流用可能)

- criterion: al-analysis `Cargo.toml:19-30` に `=0.8.2` dev-dep と
  `[[bench]] stft`/`conv`(harness=false)が A1/A3/A4 チェーンで
  整備済み。al-math にも `benches/{dsp_smoke,lstsq,vec_primitives}.rs`
- `examples/bench_reference.rs` + `target/bench-baseline/` に
  `bench-reference*.txt`・`criterion-*.txt` の baseline 記録が既存
- golden テスト: `al-analysis/tests/golden.rs` 等 — 最適化後も pass
  必須
- stage 計時: `runner.rs` の `StageRecord.duration_ms` 永続化で
  stage 単位の before/after 証拠が取れる(observability チェーン成果)
- Implementation Record は `docs/impl/ir/` 配下(caller 側最適化
  チェーンの IR 先例: `caller-side-optimizations-bench-baseline-and-
  characterization.md` 等)

### 前提(ユーザー指定の process ポリシー — 制約として課す)

- サイトごとに test+bench-first: 各変更の前に現行出力を捕捉する
  characterization が存在し pass すること、ベンチ baseline を変更前に
  採取し変更後に再測すること、before/after 実測を Implementation
  Record に記録すること(前チェーンと同一手続き)
- kernels untouched 継続: al-math カーネル実装は変更しない。
  変更面は al-analysis / al-locate / al-media / al-dataset の caller
  と filter/stft の公開 API の**利用方法**のみ(新規公開 API は
  原則追加しない — `bandpass_many` は既存 public)
- pre-release 方針: 互換層・移行期間は設けない

## Alternatives and Comparison

### スコープ単位

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A6-A16 の機械的部分を 1 バンドル | 同一の tolerance/等価性ポリシーと test+bench-first 手続きを共有。設計判断を伴う項目を切り離して論点を限定できる | 差分が散在し site 数が多い | yes(確定) |
| 全残項目(設計系含む)を 1 チェーン | 網羅的 | A2/A5/A10/A13/A14 は契約・並行性・durability・メモリ設計の判断が要り「設計を変えない」制約と衝突 | no |
| site ごとに個別チェーン | 差分最小 | 同じ characterization/bench 基盤の重複 | no |

### A7/A8 の置換形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| call site で scalar 複素積をインライン化 | `csd` 公開 API 不変。返り値を要素消費するだけの site では中間 Vec が完全に消える。bit-identical | 式の重複は `conj(s)·c` 1 行程度で許容範囲 | yes |
| csd に out-buffer 版を追加 | API 追加で再利用可能 | 「新規公開 API を増やさない」制約と緊張。全 site がインラインで足りる | no |

### A9 派生の露出形

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| caller 側で `power()` 1 回 + 局所派生(mag=sqrt、psd=`* (1/norm)`) | API 不変。`window`/`sample_rate` は pub で norm 再現可能 | norm 計算式の知識が caller に漏れる | yes(候補) |
| StftMatrix に共有 pass の派生 API を追加 | 知識は stft 側に閉じる | 公開 API 追加。「設計を変えない」制約との兼ね合いを spec で判断 | maybe |
| 3 メソッド内部で共有キャッシュ | API 不変 | StftMatrix が不変構造なので内部可変状態が要り設計変更 | no |

### A12 の count 手段

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| HashSet<u64> pre-count → gate 内のみ sort | 標準ライブラリのみ。count/support/quant 全て identical | gate 内入力では HashSet 構築の分だけ増える(1M 以下なので限定) | yes |
| ソート前に早期終了付き推定 | alloc 最小 | 近似 count では support 値が変わり契約破壊 | no |

## Tentative Conclusions and Hypotheses

以下は調査でコード証跡を確認済みの事項:

- confirmed(スコープ): A6/A7/A8/A9-derivation/A12/A15/A16 を 1 つの
  「機械的 cleanup」バンドルとする。全項目が API 形状・永続化
  フォーマットを維持し、設計判断を要しない
- confirmed(等価性クラス): A6/A7/A8/A12/A15/A16(dom/video/tensor)
  は bit-identical。A9 派生は `* (1.0/norm)` 式を保持すれば
  bit-identical(`/norm` 除算リテラル化は ulp 級になるため禁止)。
  A16 mean_psd は scalar 行優先なら bit-identical、vec reduction
  化なら ulp 級 — バンドル内で唯一 tolerance 適用を許容する項目
- confirmed(除外と理由): A2 ncc plan API(新 API surface、issue
  #42 系に隣接 — 前 discovery で defer 済み)、A5 local_lag(数値
  契約の判断が要る)、A10 stage 並列化(並行性設計)、A13 fsync
  (durability 契約)、A14 PCM cache(メモリ設計)、A9 envelope
  prefix-sum(cancellation 精度)、A9 matrix_to_tensor(ストレージ
  形式変更)、residual-gather/lod.rs(以前の deferral 継続)
- confirmed(プロセス): 前チェーンと同じ test+bench-first。
  characterization 先行・baseline/after 実測・IR 記録
- hypothesis: A6 で当該セクション ~2×(forward/plan が 16→2 に
  縮小)。A8 で ~8.6M 規模の微小 alloc が消え hft 構築が顕著に
  軽くなる。A7 で frame 数×bins 長 Vec が site ごとに消える。
  A9 で frames×bins の mag_sq walk が 3→1 に。A12 で通常入力
  (uniq>1M)の二重ソートが消える。A15 で ~17M 回の 8B extend が
  chunked 書出しに。効果量は全て実測で確認する

## Open Questions

- A9: caller 側派生か StftMatrix 派生 API か → spec で確定
  (API 追加なしが制約上のデフォルト)
- A16 mean_psd: scalar 行優先(bit-identical)で留めるか vec
  reduction(tolerance)まで行くか → spec で確定。制約上は scalar
  形が安全側
- A16 tensor fast path: 「最内 dim が region に完全包含」の判定と
  dst 連続性の条件をどこで検査するか → design で確定
- A15: 書出しチャンクの粒度(per-frame staged row vs チャネル
  ブロック)→ design で確定。出力バイト列は不変
- A6: `bands.iter().take(8)` の 8 帯域 cap は維持(bandpass_many へ
  `&bands[..8]` を渡すだけ)— 確認事項として spec に明記
- site ごとの characterization の捕捉粒度(関数レベル or record
  レベル)→ spec で確定(前チェーンは関数レベル)

## Promotion Candidates

- [ ] spec-doc needed: 機械的 cleanup バンドルのスコープ・受入基準
  — 項目別等価性クラス(bit-identical / mean_psd のみ tolerance)、
  characterization/ベンチ先行手続き、golden 維持、IR への
  before/after 実測記録
- [ ] adr-doc needed: 不要見込み(全項目が既存ポリシー内の機械的
  適用。mean_psd の ulp 許容判断が ADR 級と判断される場合のみ検討)
- [ ] defer 先の記録: 除外項目(A2/A5/A10/A13/A14/A9-envelope/
  A9-tensor/residual-gather/lod)は理由付きで将来探索へ据置き

## Sources and Relations

- 調査対象コードは `relations.source` に記載(`reference.rs`・
  `stft.rs`・`cheap.rs`・`band_gain.rs`・al-locate `normalize.rs`・
  al-media `video.rs`・al-dataset `tensor.rs`・`filter.rs`・
  `vec.rs`)。本ドキュメントの finding は全て現 HEAD の file:line で
  検証済み(survey 記載の行番号は A1/A3/A4 マージで変位していたため
  再特定: 例 band_correlation は survey の :668-672 → 現 :742-747、
  hft csd は :507-519 → 現 :584-597、uniq は :577-582 → 現
  :577-603)
- 上流(derives-from/depends-on): `caller-side-optimizations.md`
  (前チェーン discovery — A1/A3/A4 適用済み、A2/A5 defer と残
  finding の発端)、`docs/adr/simd-reduction-lane-fma.md`
  (recomposition tolerance 述語の先例)、
  `docs/specs/observability-foundation.md`・
  `docs/adr/stage-timing-manifest-persistence.md`(stage 計時証拠)
- 関連探索: `al-math-simd-reduction.md`・
  `observability-foundation-logging-and-profiling.md`
- issue #42(DSP カーネル SIMD 化)に隣接するが、A2 ncc plan 再利用
  を含め kernel 側は本バンドルのスコープ外
- 作業ブランチ: `feature/mechanical-caller-cleanup`
