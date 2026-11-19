---
id: "KNOW-huh1feWoWSYtVFnuRlIkru"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "codebase"
title: "DSP/数学基盤: al-math・ベクトルプリミティブ・相関 plan"
created: "2026-11-19"
updated: "2026-11-19"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# DSP/数学基盤: al-math・ベクトルプリミティブ・相関 plan

`al-math` クレートは、AudioLens の数値計算を集約する基盤層で、pulp による実行時 SIMD ディスパッチを唯一の外部依存とする。GPU は転送コスト支配と決定性リスクで対象外(#42)。本書は公開 API の境界設計、ベクトルプリミティブ、FFT/相関カーネル、plan/workspace 分離の構造をまとめる。計測結果は `docs/evidence/dsp-kernel-verification.md` と `docs/evidence/performance-measurements.md` に記録する。

## API 境界: 中立化と split re/im

公開 API は f64 スライスとクレート所有構造体だけに中立化し、pulp 型(`Simd`/`WithSimd`/`c64`/`F64s`/`C64s`/`dispatch`)は `pub(crate)` に降格した。pre-release 方針で互換層・deprecation を設けず、外部利用は al-locate の dsp facade とベンチ/テストのみに限定された。`Fft::transform(&self, re, im, inverse)`、`RealFft::forward/inverse` は複素データを `&mut [f64]` の re/im ペアで受け取り、呼出しごとの `Vec<c64>` 確保と往復コピーを除去する。n≥16384 では thread_local workspace の out-of-place 経路を使う。

設計上の決まりごと: 「公開入口で dispatch を 1 回だけ呼び、内部は `Simd` ジェネリック単一ソース」が全カーネルの基本形。`in_re`/`in_im` の同一スライス共有は debug assert(release では検査しない)。`al_locate::dsp` の re-export が唯一の消費経路で、al-analysis は al-math に直接依存しない(`cargo tree --depth 1` で機械検証)。

## ベクトルプリミティブ(vec)

`al_math::vec` に 8 プリミティブを集約した: `dot`/`sum_sq`/`corr0`/`rms`/`scale`/`axpy`/`sub_scaled`/`mag_sq`。3 層構造(`pub(crate)` カーネル + `WithSimd` アダプタ + 入口 1 回 dispatch の公開 wrapper)。

契約の要点:

- **等長 panic**: zip の黙認を継承せず、非等長は panic(`check_same_len` 共有)。
- **値の契約**: `corr0` は Σxy/Σx²/Σy² を 1 パス融合で、Σx²>0 かつ Σy²>0 でなければ 0.0(中心化しない)。`rms` は空入力で 0.0。
- **FMA 規則**(ADR `simd-reduction-lane-fma` が権威): 積和は `mul_add_e_f64s` のみ。`mul_add_f64s`(scalar でも融合してビット一致を破る)と 2 命令形は禁止。`sub_scaled` は `negate_mul_add_e`、`mag_sq` は `mul_add_e(r, r, mul_f64s(i, i))`、`scale` は純粋乗算。
- **エイリアス禁止**: プリミティブは入出力スライスのエイリアシングを許容しない(論理 in-place は別バッファ)。
- 融合→再合成・除算→逆数乗算は結合順差で永続化値が ~ulp 揺れ得るため、「再合成許容ポリシー」として ADR で事前受容した。

## FFT・相関・QR

`Fft` は係数を変換長単位で保持する plan 型で、radix-2 二段融合 DIF butterfly を `Simd` 単一ソースで実装する。出力は自然順、forward 負符号/inverse 正符号+1/N。`RealFft` は半長複素入力で非負周波数 N/2+1 bins が正本。非 2 冪長は caller が next_power_of_two+ゼロ埋めを所有する現行互換方針。`bandpass` は IIR でなく spectral-mask 応答を保持し、実数 FFT 1 回+帯域ごと IFFT。`BandNcc` は素材/キャプチャの forward FFT を帯域間共有する。`least_squares` は Householder QR 化(`LeastSquares` が反射と R を所有し `fit_mix` は同一分解を rhs ごとに再利用)。数値ランク判定は R 対角と拡大行列ノルムの閾値で、非正則系は `None`。ridge は `[A; √λ I]` 拡大系、残差 RMS は元観測のみ。条件数は ridge 前の AᵀA に対して計算する。

## plan/workspace 分離(#52)

FFT 係数と packing/scratch を「不変 `CorrelationPlan`/`CorrelationWorkspace`」に分離し、NCC/PHAT を同一 kernel の借用曲線 API に集約した。shape 不一致は演算前 panic、curve は ws 借用(`&[f64]`、借用中は ws 再使用不可を型で保証)。全 scratch は constructor で確保し explicit path は TLS を使わない。`OverlapCorr` は `local_lag` の全走査を同一推定量で FFT 化した変種(FFT 分子+prefix 二乗和分母)。`CorrelationPool` は形状キーのワーカー単位プール(`MAX_SHAPES=8`、超過時全 clear)。

計測方法論として、変更前コミットの tracked source archive を分離展開して同一 harness で before 計測し、binary SHA・fixture・raw samples・RSS を保存する provenance 設計を採用した(最初の計測は binary SHA 保存漏れで非採用として修復した経緯がある)。計測値と実際の適用範囲(実 caller のみ、全体確保ゼロではない)は厳密に区別して記録する。

## 形状適応 dispatch(#71 P3)

`CorrelationPlan::new_ncc` に直接法経路 `PlanMode::NccDirect` を追加し、`K = signal_len − template_len` が小さい形状で分子を axpy 蓄積する。コストモデル `L·(K+1) ≤ C·n·log2(n)` で NccDirect/NccValid を選択し、6 代表形状で実測最速側と一致した。direct plan の workspace は FFT バッファを確保しない。帯域生成は `BandFilter`(forward スペクトル保持+per-band mask/IFFT)で逐次化し、同時保持を解消した。

## 実装上の落とし穴

- SIMD カーネルを経由する関数が呼出境界を作ると `#[target_feature]` コンテキストから切れて内部がスカラー化する(P2 で `#[inline(always)]` 化で修正した回帰)。
- split re/im の `cmul` 再合成は pulp `mul_c64s` と FMA の融合側が逆で ~1ulp 差(性能優先で意図的採用)。
- 等価なはずの置換でも FP 累算順差が同点 argmax のタイブレークを変え、合議受理/拒否を変え得る —— `docs/knowledge/locate-and-tracking.md` の曖昧性検出の経緯を参照。
- grep が取りこぼす参照(brace import・複数行 use)の検出はコンパイルを第一ゲートとし、その後 rg 監査(残存参照 0 件確認)を行う。

関連 Issue/PR: #40、#42、#52、#56、#59、#71、ruviform#168、ruviform#183、ruviform#255。
