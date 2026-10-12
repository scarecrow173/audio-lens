---
id: "SPEC-034EMwMaXGBCfumSazX7zs"
type: "spec"
status: "approved"
title: "機械的 cleanup 最適化バンドル: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16)"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source:
    - "../../crates/al-analysis/src/reference.rs"
    - "../../crates/al-analysis/src/stft.rs"
    - "../../crates/al-analysis/src/cheap.rs"
    - "../../crates/al-analysis/src/band_gain.rs"
    - "../../crates/al-analysis/src/expensive/channel.rs"
    - "../../crates/al-analysis/src/expensive/mod.rs"
    - "../../crates/al-locate/src/dsp.rs"
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
  implemented-by:
    - "../designs/mechanical-caller-cleanup.md"
    - "../plans/mechanical-caller-cleanup.md"
  depends-on:
    - "../discovery/mechanical-caller-cleanup.md"
    - "./al-math-vec-primitives.md"
    - "../adr/al-math-vec-primitives-slice-boundary.md"
    - "../adr/simd-reduction-lane-fma.md"
    - "./observability-foundation.md"
    - "../adr/stage-timing-manifest-persistence.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./caller-side-optimizations.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/mechanical-caller-cleanup.md"
  derived-by:
    - "../designs/mechanical-caller-cleanup.md"
    - "../plans/mechanical-caller-cleanup.md"
    - "../tasks/mech-cleanup-bench-baseline-characterization.md"
    - "../tasks/mech-cleanup-measurement-ir-closeout.md"
  verifies: []
  verified-by:
    - "../test-specs/mechanical-caller-cleanup.md"
  references: []
  defers: []
  deferred-by: []
---

# 機械的 cleanup 最適化バンドル: bandpass バッチ化・csd インライン化・pass 集約 (A6-A16)

## What Should Be Built

al-analysis / al-locate / al-media / al-dataset の呼出し側(caller)
コードに残る機械的 cleanup 項目(A6/A7/A8/A9 派生項/A12/A15/A16)を、
al-math カーネル実装・公開 API 形状・永続化フォーマットを一切変更
せずに最適化する。変更面は caller コードと既存 public API
(`bandpass_many`・`power`・`scale` 等)の**利用方法**のみ。新規公開
API は原則追加しない。全サイト・等価性クラスは discovery
(DISC-034EMosQizFcChc9KKeaWS)で現 HEAD の file:line 検証済み。
加えて spec review で production の `csd(` caller・strided 列
平均・per-sample 配置の各 site list を `rg` で再列挙し、列挙の
網羅性を現 HEAD に対して確認済み(`stft.rs` 内の `csd` 使用は
`#[cfg(test)]` 配下のみ)。

- **A6 — band_correlation の bandpass バッチ化**:
  `reference.rs:742-747` が `bands.iter().take(8)` ループ内で
  `bandpass(src_mono, …)` / `bandpass(cap_mono, …)` を計 16 回呼ぶ
  のを、`bandpass_many(src_mono, rate,
  &bands[..bands.len().min(8)])` と `bandpass_many(cap_mono, rate,
  &bands[..bands.len().min(8)])` の 2 呼出しへ置き換え、返り値を
  band 順に `corr0` して `band_corrs` を組み立てる。
  8 帯域キャップは維持するが、スライス境界は
  `bands.len().min(8)` とする — 現行の `take(8)` は bands < 8
  でそのまま縮退するのに対し、リテラル `&bands[..8]` は
  bands < 8 で panic するため禁止。
  `bandpass` は `bandpass_many` の薄い wrapper(`filter.rs:7-11`)
  であり、同一入力・同一 mask 規則(`lower_bin`、`filter.rs:51-54` 付近)
  ・同一 inverse・同一切戻し(`real[..len].to_vec()`)で変換数学は
  完全に同一 → **等価性: bit-identical**。plan/forward が 16→2 に
  縮小し当該セクション ~2× を見込む(実測で確認)
- **A7 — csd の per-frame Vec アロケーション除去**: `csd`
  (`stft.rs:173-179`、要素式 `(xr*yr + xi*yi, xr*yi - xi*yr)` =
  `conj(x)·y`、`stft.rs:177`)が返す bins 長 `Vec<(f64,f64)>` を、
  call site で scalar 複素積のインライン累積に置き換える。
  サイト(production `csd(` caller の全列挙。A8 の 1 要素呼出し
  `reference.rs:587` を除く):
  `reference.rs:183`(fir_ir 内 `cxx`/`cxy` 累積)、
  `reference.rs:539-545`(Welch `pxy` 累積)、
  `reference.rs:872-877`(L/R coherence `pab` 累積)、
  `band_gain.rs:159`(`coherence()` の `xy` 累積)、
  `cheap.rs:1105-1120`(stereo pair の per-frame
  `csd(&ma.re[f],…)` を `csd_re`/`csd_im` へ累積 — frames は
  `min(64)` にキャップ済み)、
  `expensive/channel.rs:102-118`(per-frame
  `csd(&mb.re[f],…)` が `band_bins` の要素のみを消費 —
  inline 積なら band bin だけを計算でき、全 site 中で最大の
  削減幅)。各 site は返り値 Vec をその場で要素ごと累積する
  だけなので、`c[k]` を同式の scalar に置き換えれば中間 Vec が
  消える(手書き `re*re + im*im` の先例が
  `reference.rs:541-542`/`874-875`・`cheap.rs:1117-1118` に
  既存)。`csd` 関数自体は公開 API として維持する(stft.rs
  テスト等が使用) → **等価性: bit-identical**(同一式・同一
  bin 昇順の直接累積で、Vec 経由の有無は丸めに無関係)
- **A8 — hft の 1 要素 csd 除去**: `reference.rs:581-598` の
  `hft` 構築が bin ごとに長さ 1 の `csd(&ms.re[f][k..k+1], …)` を
  呼び `c[0]` だけを使うのを、scalar `(cr, ci)` に置き換え、
  `((cr*cr + ci*ci).sqrt()) / xs` とする(現行の `c[0]` 利用式
  `:593` と同一)。frames × half(bins/2)回 — ~8.6M 個の微小 heap
  alloc が消える → **等価性: bit-identical**
- **A9(派生項のみ)— power 1 pass から magnitude/psd を派生**:
  `StftMatrix::psd`(`stft.rs:48-64`)・`magnitude`(`:68-78`)・
  `power`(`:81-89`)がそれぞれ frames×bins を walk して `mag_sq`
  する三重 pass を、`power()` 1 回 + caller 側局所派生に集約する。
  派生式は magnitude = `power[k].sqrt()`、psd = `power[k] *
  (1.0/norm)`(`norm = 0.375 * window * sample_rate`;
  `window`/`sample_rate` は pub フィールドで caller 再現可能。
  `window_power() * sample_rate` と同じ結合順 `(0.375 * w) * r`
  を使う)。**psd 派生は逆数乗算 `* (1.0/norm)` または `scale`
  プリミティブの形を必須とし、`/ norm` 除算リテラルは禁止**
  (ulp 級差を生じ得る)。**caller 側派生を採用**し、StftMatrix に
  新規公開 API は追加しない(「設計を変えない」制約上のデフォルト)。
  適用サイト: `cheap.rs:632-634`(mag → envelope `:660`、pow →
  band_power `:645,:654`、psd → tensor — 3 行列全てを 1 回の
  mag_sq pass から派生)、`reference.rs:653-654`(`psd_s`/`psd_c`
  を `power()` 行 + 要素ごと `* inv` で派生)。
  **`cheap.rs` 側では派生が `m` の move より前に完了すること**
  — `m` は `channel_stft.push(Some(m))`(`cheap.rs:665`)で
  move されるため、mag/psd の派生と `m.fft_size`/`m.window`/
  `m.sample_rate` への参照は全て `:665` より前に配置する。
  `cheap.rs:794-796` の `mean_row`(mag 上の bin ごと strided
  列平均、A16 mean_psd と同一 idiom)も、この row-major 派生
  pass で mag 行を産出するのと同じ走査内に `mean_row[k] +=
  mag_row[k]`(行昇順・0.0 起点・`k < half`)として融合し、
  最後に `mag.len()` で割る — 列走査の追加 pass を消す。
  監査済みの単独消費 call site(record のみ・現行維持可):
  `cheap.rs:942` の `mm.magnitude()`(AM spectrum 用の単独
  消費。派生しても bit-identical)、`reference.rs:1189` の
  `rm.psd()`(residual PSD tensor + band energy の単独消費。
  同様に記録のみ)。`reference.rs:1071` の `magnitude()` 使用は
  A16 dom() で除去 → **等価性: bit-identical**(式保持が条件)
- **A12 — uniq の二重ソート回避**: `cheap.rs:577-603`。
  全サンプルの `to_bits` を `HashSet<u64>` に集めて distinct count
  を先に取り、`2 <= count <= 1_000_000` の gate 内の場合のみ
  collect→`sort_by(total_cmp)`→min positive diff を実行する。
  gate 外(実運用では >1M 側)では両ソートと f64 への materialize
  を全て skip する。`set.len()` は dedup 後の `uniq.len()` と同値
  であり、`dig.quant_step_est` の support(`:603`)にはその count
  を使う → **等価性: bit-identical**(record 値レベルで identical。
  quant は同一集合の同一ソート順から算出、support も同じ count)
- **A15 — per-sample interleave の chunked 化**:
  `al-locate/src/normalize.rs:168-171`(frame×channel ごとに
  `bytes.extend_from_slice(&channel[frame].to_le_bytes())` で 8B
  単位 extend、~17M 回規模)、`al-locate/src/dsp.rs:17-29` の
  `deinterleave`(frame ごとに `out[ch].push(v)` する同一の
  per-sample 配置パターンの逆方向。kernel ではなく dsp facade
  helper であり、同一の機械的書換えで対象化 — 本番 caller は
  `decode.rs:125`/`:224`/`:377`)、および
  `reference.rs:1163-1170`(residual チャネルの sample ごと
  `v.push(ch[i])`)を、per-frame staged row 化・index 書込み・
  チャネルブロック単位 `extend_from_slice` 等の
  strided/chunked 書出し・書込みに置き換える(粒度は design
  で確定) → **等価性: bit-identical**(interleave/
  deinterleave 順序が同一なら出力バイト列/要素列は完全一致。
  配置問題であり値を変えない)
- **A16 — 小粒項目 4 件**:
  - mean_psd 行優先化: `reference.rs:655-660` の bin ごと strided
    列集計を行優先 `for row { mean[k] += row[k] }` へ。
    **scalar 形を採用する**(per-bin の加算順: 行昇順・0.0 起点を
    保つため **bit-identical**)。vec reduction プリミティブへの
    載せ替えは加算順が変わり ulp 級(tolerance-class)になるため
    本バンドルでは採用しない — バンドル内で唯一 tolerance 適用が
    許容され得た項目だが、安全側の scalar 形を選択肢として確定する。
    同一 idiom の兄弟 site も同じ書換えを適用する:
    `expensive/mod.rs:648-651` の `mean_power`(`power()` 行列上の
    bin ごと strided 列平均 → 行昇順 `mean_power[k] += row[k]`
    の row-wise 累積。`total_power`/`one_sided_power` の値は
    不変)。`cheap.rs:794-796` の `mean_row` も同 idiom だが
    mag 行列上の列平均のため、A9 側の row-major 派生 pass に
    融合する形で扱う(A9 項参照)
  - `dom()` の magnitude 行列除去: `reference.rs:1071-1084`。
    argmax のためだけに `m.magnitude()` 全行列を materialize して
    いるのを、frame ごとに `mag_sq(&m.re[f], &m.im[f],
    &mut scratch_row)`(dsp ファサード経由・al-math 直接依存は
    追加しない)を **1 本だけ確保して再利用する `fft_size` 長
    scratch buffer** へ書き出し、`k < m.fft_size/2` の範囲で
    `mean[k] += scratch_row[k].sqrt()` を累積する形へ
    (現行の累積が `mean = vec![0.0; m.fft_size/2]` への k<half
    限定であるのは `reference.rs:1073-1076` 通り)。
    **手書きの `re*re + im*im` scalar 式は禁止** —
    `magnitude()` の要素値は `mag_sq` → `mag_sq_kernel`
    経由で `simd.mul_add_e_f64s` を使い(`vec.rs:128-139`)、
    x86-v3 lane では真の FMA、Scalar lane および `len % lanes`
    の tail では非融合 `r*r + i*i` という位置依存の丸めを持つ。
    手書き式は lane 上で ulp 差・tail との不一致を起こし得る
    ため bit-identical にならない。`mag_sq` 自体を再利用すれば
    per-element セマンティクスと累積の加算順(行昇順・0.0 起点)
    が完全に一致し、行列 alloc が消える。`power()` の再利用は
    不可(sqrt が総和の内側にあり argmax(Σ√x) ≠ argmax(Σx))
    → **bit-identical**(bit 一致の根拠は式の一致ではなく
    `mag_sq` の再利用)
  - video luma/diff 融合: `al-media/src/video.rs:402-441`。
    frame ごとの全画素 luminance 和(`:427`)と prev との diff
    zip-sum(`:428-440`)を、同一イテレーション順の別 accumulator
    として 1 pass に fuse する。各和の加算順は不変。
    初帧の `prev == None` 分岐(`None => 0.0`、`:438-439` —
    `diff[0] = 0.0` 相当)は融合後も必ず保持し、`pixels == 0`
    (`width * height`、`:391`)の退化 case でも現行と同じ出力
    (空 luma の `0.0 / 0.0` → NaN 系列)を再現すること →
    **bit-identical**(ROI 和 `:413-424` は系列ごとの部分和で
    あり対象外)
  - tensor `read_range` 連続行 fast path:
    `al-dataset/src/tensor.rs:271-289` の要素ごと gather
    (`out[dst] = chunk_data[src]`)で、最内 dim が region 内に
    完全収まり src/dst とも連続な行をスライスコピー
    (`extend_from_slice`/`copy_from_slice` 系)する fast path を
    追加。境界条件の判定位置は design で確定 → 返却データは同一
    要素・同一順序で **identical**

プロセス契約(ユーザー指定の最高優先事項): 各最適化の着地前に
(1) 現行出力を捕捉する characterization テストが存在し pass
すること、(2) ベンチ baseline を変更前に採取し変更後に再測する
こと、(3) before/after の実測を Implementation Record に記録する
こと — 前チェーン(caller-side-optimizations)と同一手続き。

> 数値等価ポリシーの権威は `docs/adr/simd-reduction-lane-fma.md`
> (ADR-034EoHxzsvsHp9BOdCK2Cn)とする。本バンドルは全項目
> bit-identical クラスで着地するため tolerance 述語
> `|a - b| <= 1e-12 * max(1, |a|, |b|)` の実適用対象はないが、
> mean_psd を将来 vec reduction 化する場合にのみ同述語が許容
> クラスとなる(本バンドルでは scalar 形を採用し不適用)。

## Why Now

caller 側最適化チェーン(A1/A3/A4、SPEC-034EHH6AJBK8GJ2H9EUzjE)が
main にマージされ、残る performance survey finding のうち設計判断を
要さない機械的部分が discovery(DISC-034EMosQizFcChc9KKeaWS)で
確定した:

- 全項目のサイトが現 HEAD の file:line で再検証済み(survey 時点の
  行番号は A1/A3/A4 マージで変位していたため)
- 等価性クラスが項目ごとに固定済みで、設計判断を残すのは
  A15 の chunk 粒度・A16 tensor 判定位置など design レベルの細部のみ
- 前チェーンで test+bench-first 手続きと計測基盤(al-analysis の
  criterion `=0.8.2` dev-dep と `[[bench]] stft`/`conv` stanza、
  `examples/bench_reference.rs`、`target/bench-baseline/` の記録、
  manifest `duration_ms` 永続化)が整備済み

A8 単体で frames×half 規模(~8.6M)の微小 alloc が消え、A7/A9/A16
(dom)で行列・Vec の中間 materialize が縮小する。効果量は全て実測で
確認する hypothesis。

## Users and Value

- 解析パイプラインの利用者: reference/cheap/normalize/video 各
  stage で帯域フィルタの plan/forward 重複・中間 Vec/行列
  アロケーション・二重ソート・per-sample 書出しが削減され、stage
  `duration_ms` の改善として観測できる(実測で確認する hypothesis)
- 実装者: 数値を一切変えない機械的置換のため、characterization
  ゲートで安全性を直接検証できる。A9 の caller 側派生パターンが
  後続の pass 集約の先例になる
- レビュアー・後続の担当者: characterization-first のプロセス契約に
  より、各最適化の前後差分がテストと記録済み実測で検証可能

## Scope

### In Scope

- **A6**: `reference.rs` band_correlation の `bandpass_many`
  バッチ化(8 帯域キャップ維持・スライス境界は
  `bands.len().min(8)`、`bandpass`/`bandpass_many` の
  公開シグネチャ不変)
- **A7**: `reference.rs:183`/`:539-545`/`:872-877`・
  `band_gain.rs:159`・`cheap.rs:1105-1120`・
  `expensive/channel.rs:102-118` での `csd` 呼出しを scalar
  複素積のインライン累積へ置換(`csd` 関数自体は維持。
  channel.rs では `band_bins` の bin のみ計算)
- **A8**: `reference.rs:581-598` hft 構築の 1 要素 `csd` を
  scalar `(cr, ci)` へ置換
- **A9(派生項)**: `cheap.rs:632-634`・`reference.rs:653-654` で
  `power()` 1 回 + caller 側局所派生(mag=sqrt、psd=`* (1.0/norm)`)へ。
  `cheap.rs:794-796` の `mean_row` は同一 row-major 派生 pass
  に融合。`StftMatrix` の公開メソッドは全て維持
- **A12**: `cheap.rs:577-603` の HashSet pre-count + gate 内のみ
  sort(`quant_step_est` 値と support 双方を維持)
- **A15**: `normalize.rs:168-171`・`dsp.rs:17-29`
  (`deinterleave`)・`reference.rs:1163-1170` の chunked
  interleave/deinterleave(出力バイト列/要素列不変)
- **A16**: mean_psd 行優先 scalar 累積(`reference.rs:655-660`)
  と同 idiom の `mean_power` 行優先化
  (`expensive/mod.rs:648-651`)、`dom()` の scratch-buffer
  `mag_sq` 累積(`reference.rs:1071-1084`)、video luma/diff
  融合(`video.rs:402-441`)、tensor `read_range` 連続行
  fast path(`tensor.rs:271-289`)
- **プロセス**: 各項目について、現行出力を捕捉する
  characterization テストの先行作成(関数レベルまたは record
  レベル — 粒度は design で確定)、ベンチ baseline/事後計測、
  Implementation Record(`docs/impl/ir/`)への before/after 実測記録
- **ベンチ基盤**: 既存基盤を流用する。kernel 級計測は al-analysis
  の criterion benches(`[[bench]] stft`/`conv`、必要に応じて同
  pattern・同 version の stanza を追加可)と al-math benches、
  stage 級代表経路は `examples/bench_reference.rs` と manifest
  `duration_ms`、baseline 記録は `target/bench-baseline/` を使う。
  **criterion/example 基盤を持たない crate の項目(al-dataset
  の tensor `read_range`、al-media の video luma)は新規
  dev-dep 追加が禁止のため新たな bench harness を設けられない**。
  これらの before/after は、当該 crate に既存の example または
  test binary がある場合はその内部での `Instant` ベース
  マイクロ計測で採取し(al-dataset は `tests/integration.rs`
  が tensor round-trip を既にカバー)、適合する既存 binary が
  ない場合は code inspection + workspace bench 回帰ゲート +
  観測可能な範囲の manifest `duration_ms` で記録する。
  IR には項目ごとに採用した計測方法を明記する。
  A15 の normalize/dsp 項目(al-locate)は、適合する場合に限り
  既存の `Instant` ベース example `examples/bench_dsp.rs` を
  再利用してよい
- **回帰防止**: golden テスト(`al-analysis/tests/golden.rs` 等)
  の pass 維持

### Out of Scope

- **A2** `ncc` 呼出しごとの `RealFft::new` plan 再利用 — 新 API
  surface を要し issue #42 系の別チェーンへ defer 済み
- **A5** `local_lag`→`ncc` 置換 — 数値契約の判断を要する
- **A10** stage 並列化 — 並行性設計を要する
- **A13** fsync/durability 方針 — durability 契約の判断を要する
- **A14** PCM cache — メモリ設計の判断を要する
- A9 派生以外の項目: envelope の prefix-sum 化(加算順変更による
  cancellation/精度リスク)、`matrix_to_tensor` 統合(永続化
  ストレージ形式の変更)
- residual-gather/lod.rs 系項目 — 以前の deferral 継続
- mean_psd の vec reduction プリミティブ化(scalar 形を採用したため。
  将来採用する場合は tolerance-class の別判断として扱う)
- al-math カーネル実装(`Fft`/`RealFft`/vec プリミティブ/
  filter の変換数学)の変更 — 変更面は caller と既存 API の利用
  方法のみ
- 新規公開 API の追加(`bandpass_many` は既存 public。A9 は
  caller 側派生で API 追加なし)
- 永続化フォーマット・公開 API シグネチャ・record セマンティクスの
  変更(ユーザー binding constraint: 設計を変えない・汎用性を
  下げる過度な特化をしない)
- マルチスレッド化・並列化
- golden テストの判定意味・Validity ゲート閾値の変更
- 互換層・deprecation 期間(pre-release 方針により置換でよい)

## Requirements

- **プロセス契約(最高優先)**: 各項目は独立に適用可能な単位とし、
  各々について
  1. 変更前に現行出力を捕捉する characterization テストが存在し
     pass すること(テスト先行、異状のないことを確認してから
     実装する)
  2. ベンチを変更前(baseline 実測を記録)と変更後に実行すること
  3. before/after の実測値を Implementation Record
     (`docs/impl/ir/`)に記録すること
  4. characterization/baseline コミットが最適化コミットより先に
     land すること — git 履歴で順序が検証可能であること
- **数値等価(項目別クラス)**:

  | 項目 | サイト | 等価性クラス | 検証方法 |
  | --- | --- | --- | --- |
  | A6 | `reference.rs:742-747` | bit-identical | `band_corrs` 各要素 `to_bits` 一致 |
  | A7 | `reference.rs:183`,`:539-545`,`:872-877`,`band_gain.rs:159`,`cheap.rs:1105-1120`,`expensive/channel.rs:102-118` | bit-identical | 累積結果(`cxx`/`cxy`/`pxy`/`pab`/`xy`/`csd_re`/`csd_im`/`sxy` と下流 coherence・h・band gain・`stereo.coherence_mean`)`to_bits` 一致 |
  | A8 | `reference.rs:581-598` | bit-identical | `hft` 全要素 `to_bits` 一致 |
  | A9 | `cheap.rs:632-634`,`reference.rs:653-654`,`cheap.rs:794-796`(mean_row 融合) | bit-identical(式保持条件) | 派生行列と直接呼出し出力の `to_bits` 一致;`* (1.0/norm)` 形の保持(除算リテラル禁止);`mean_row` と下流 `spec.slope` 系 record の一致;`cheap.rs` 側派生が `channel_stft.push(Some(m))`(`:665`)前に完了(コード検査) |
  | A12 | `cheap.rs:577-603` | bit-identical(record レベル) | `quant_step_est` 値・support・Validity が一致 |
  | A15 | `normalize.rs:168-171`,`dsp.rs:17-29`,`reference.rs:1163-1170` | bit-identical | 出力バイト列/`interleaved` 要素列/`deinterleave` 返却 `Vec<Vec<f64>>` の完全一致 |
  | A16 mean_psd | `reference.rs:655-660`,`expensive/mod.rs:648-651`(mean_power) | bit-identical(scalar 採用) | `mean_psd_s`/`mean_psd_c`/`mean_power` 全要素 `to_bits` 一致(`total_power`/`one_sided_power` 含む) |
  | A16 dom() | `reference.rs:1071-1084` | bit-identical | `ref.harmonic_freq_ratio` record が一致(再利用 scratch への `mag_sq` 呼出し・手書き `re*re+im*im` なしをコード検査) |
  | A16 video | `video.rs:402-441` | bit-identical | `luminance`/`diff` 系列 `to_bits` 一致(初帧 `diff` = 0.0 分岐と `pixels == 0` 退化出力の保持) |
  | A16 tensor | `tensor.rs:271-289` | identical | 返却 `(out, region_shape)` が完全一致 |

  - fixture は決定的(シード固定パターンまたはテスト内の
    synthetic fixture)でエッジケースを含めること
  - tolerance 述語 `|a - b| <= 1e-12 * max(1.0, |a|, |b|)`(権威:
    ADR-034EoHxzsvsHp9BOdCK2Cn)は本バンドルでは実適用対象なし。
    mean_psd を vec reduction 化する変更だけが将来の許容クラス
    (本バンドルでは scalar 採用により不適用)
- **API/形式不変**: 全ての公開 API シグネチャ(`bandpass`/
  `bandpass_many`/`csd`/`StftMatrix::{psd,magnitude,power}` 等)、
  永続化フォーマット(normalized `.f64le` バイト列・tensor
  store・record/artifact schema)、出力セマンティクスを維持する。
  新規公開 API は追加しない(A9 は caller 側派生)
- **A9 式保持**: psd 派生は `power[k] * (1.0/norm)`(逆数乗算)
  または `scale` プリミティブ再利用の形に限る。`power[k] / norm`
  の除算リテラルは禁止。`norm` は `(0.375 * window) * sample_rate`
  の結合順で pub フィールドから再現する
- **A12 契約保持**: `set.len()` と dedup 後 `uniq.len()` の同値性
  を前提に `quant_step_est` の値・support・Validity 判定を不変に
  保つ。近似 count による support 変更は禁止
- **依存**: 新規依存を追加しない(`HashSet` は std)。al-analysis
  への al-math 直接依存の追加もしない(dsp ファサード一元化を
  維持)
- `mise exec -- cargo test --workspace`、workspace build、
  `cargo clippy`、`cargo fmt --check` が green であること

## Acceptance Criteria

各最適化は (a) characterization テストの先行 green、(b) 等価性
ゲート(全項目 `to_bits`/完全一致 — Requirements の表参照)、
(c) bench delta の記録、(d) 非対象経路でノイズ幅(目安 ±5%)を
超える回帰なし、の4条件を満たすこと。性能目標は方向性(実測で
確認)とし、項目ごとの効果量は IR に記録する。

- [ ] 各項目(A6/A7/A8/A9/A12/A15/A16 各小粒)について、変更前に
  作成した characterization テストが現行出力を捕捉し、最適化前に
  green・最適化後も green である(characterization + baseline
  コミットが最適化コミットより先という順序が git 履歴から確認
  できる)
- [ ] A6: 固定の決定的 fixture で `band_corrs` が変更前後で
  `to_bits` 一致するテストが pass する。コード検査で
  band_correlation セクションに単発 `bandpass` ループがなく
  `bandpass_many` 呼出しが src/cap の 2 回のみ・スライス境界が
  `bands.len().min(8)` の 8 帯域キャップで維持されていること
  (panic し得るリテラル `&bands[..8]` でないこと)を確認する
- [ ] A7/A8: 列挙サイト(`reference.rs:183`/`:539-545`/
  `:581-598`/`:872-877`・`band_gain.rs:159`・
  `cheap.rs:1105-1120`・`expensive/channel.rs:102-118`)で
  `csd` 呼出しがなく(コード検査)、蓄積結果と `hft` 出力が
  `to_bits` 一致する characterization が pass する
- [ ] A9: `cheap.rs:632-634`・`reference.rs:653-654` で
  magnitude/psd 派生行列が直接メソッド呼出しの出力と `to_bits`
  一致するテストが pass し、派生コードに `/ norm` 除算リテラルが
  ない(コード検査)。`cheap.rs` 側で全派生が
  `channel_stft.push(Some(m))`(`:665`)より前に完了し、
  `mean_row`(`cheap.rs:794-796`)が同じ row-major pass に
  融合されていること(コード検査)。`StftMatrix` に新規公開
  メソッドがない
- [ ] A12: 代表 fixture で `dig.quant_step_est` の値・support・
  Validity が変更前後で一致する characterization が pass する
  (>1M distinct の skip 経路と gate 内経路の双方を含む)
- [ ] A15: `normalize.rs` の normalized `.f64le` 出力・
  `dsp.rs` `deinterleave` の返却 `Vec<Vec<f64>>`・
  `reference.rs` の `interleaved` が変更前後でバイト/要素完全一致
  する characterization が pass する
- [ ] A16: `mean_psd_*`/`mean_power` が `to_bits` 一致(scalar
  行優先形であることをコード検査)、`ref.harmonic_freq_ratio`
  record が一致(dom() が再利用 scratch への `mag_sq` 呼出しで
  実装され手書き `re*re+im*im` がないことをコード検査)、video
  `luminance`/`diff` 系列が `to_bits` 一致(初帧 `diff` = 0.0
  分岐と `pixels == 0` 退化出力が保持されていることを
  コード検査+characterization で確認)、tensor
  `read_range` の返却データが fast path・fallback 双方で完全一致
  (最内 dim 部分包含のエッジケースを含む)するテストが pass する
- [ ] 各項目の bench delta(criterion または `bench_reference` +
  manifest `duration_ms`、bench 基盤なし crate は既存 example/
  test binary 内の `Instant` マイクロ計測または code
  inspection + 回帰ゲート + `duration_ms`)の before/after
  実測が Implementation Record に記録され(項目ごとの採用計測
  方法を明記)、非対象経路に ±5% を超える回帰がない。方向性
  目標: band_correlation セクションの改善が計測可能、A7/A8/A9/
  A16(dom)で alloc または中間行列の削減が bench 上観測可能
- [ ] workspace テスト・build・clippy・fmt が green で、golden
  テストが pass する
- [ ] `audit_docs --type spec` と IR の `audit_impl_record` が
  clean である

## Deferred Design Concerns

- A15 の書出しチャンク粒度(per-frame staged row vs チャネル
  ブロック)と `deinterleave` 側の変換粒度、A16 tensor fast
  path の「最内 dim が region に完全包含・dst 連続」判定位置、
  各 site の characterization 捕捉粒度
  (関数レベル or record レベル) — design で確定
- A2 `ncc` plan 再利用・A5 `local_lag`→`ncc` 置換 — 別チェーン
  として defer 確定(discovery の Tentative Conclusions)
- A10/A13/A14、A9-envelope prefix-sum、matrix_to_tensor 統合、
  residual-gather/lod — 理由付きで将来探索へ据置き(discovery の
  除外リスト)
- ADR は作成しない: 全項目が既存ポリシー(数値不変・pre-release
  置換・kernels untouched)内の機械的適用であり、mean_psd の
  ulp 許容判断も scalar 採用で不要になったため
- criterion ベンチの CI 回帰ゲート — 前 spec と同様、CI 基盤
  未整備のため見送り

## Sources and Relations

- 調査: `docs/discovery/mechanical-caller-cleanup.md`
  (DISC-034EMosQizFcChc9KKeaWS — 全 finding を現 HEAD の
  file:line で検証済み。スコープ・等価性クラス・計測手続きの確定
  事項、選択肢比較、Open Questions。本 spec は同 doc の Open
  Questions のうち A9 露出形 → caller 側派生、A16 mean_psd →
  scalar 行優先、を確定した)
- 前チェーン: `docs/specs/caller-side-optimizations.md`
  (SPEC-034EHH6AJBK8GJ2H9EUzjE — A1/A3/A4 適用済み、同一の
  test+bench-first 手続きと bench 基盤を継承)
- 数値ポリシー: `docs/adr/simd-reduction-lane-fma.md`
  (ADR-034EoHxzsvsHp9BOdCK2Cn — tolerance 述語の権威。本バンドル
  では全項目 bit-identical で着地)
- プリミティブ境界: `docs/specs/al-math-vec-primitives.md`・
  `docs/adr/al-math-vec-primitives-slice-boundary.md`
  (`mag_sq`/`scale` プリミティブの要素式セマンティクス —
  A9 派生・A16 dom() の bit-identical 根拠)
- 計測証拠: `docs/specs/observability-foundation.md`・
  `docs/adr/stage-timing-manifest-persistence.md`(stage
  `duration_ms` 永続化による before/after 証拠)
- 関連 issue: #42(DSP カーネル SIMD 化)に隣接するが、カーネル
  内部と A2/A5 は本バンドルのスコープ外
- 作業ブランチ: `feature/mechanical-caller-cleanup`
