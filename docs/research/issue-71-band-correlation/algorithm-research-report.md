---
title: "AudioLens Issue #71：帯域相関・帯域ゲインの構造最適化に関する技術調査報告書"
repository: "https://github.com/scarecrow173/audio-lens"
issue: "https://github.com/scarecrow173/audio-lens/issues/71"
date: "2026-10-17"
status: "調査・設計検討（実装および追加ベンチマークは未実施）"
---

# AudioLens Issue #71：帯域相関・帯域ゲインの構造最適化に関する技術調査報告書

**対象**：[`scarecrow173/audio-lens` Issue #71 — perf: band 系相関の構造最適化 (locate band_ncc / reference band_gain)](https://github.com/scarecrow173/audio-lens/issues/71)
**調査日**：2026年10月17日
**対象コード**：調査時点の `main`（後述のリポジトリファイル・Issue記録を参照）
**文書種別**：アルゴリズム調査・代替案比較・実験計画。設計決定（ADR）や実測性能改善の証明ではない。

## 0. エグゼクティブサマリー

`band_ncc` と `band_gain` はどちらも帯域別相関に関わるが、**同じ単一カーネルの最適化で一挙に解決できる問題ではない**。`band_ncc` は数帯域の**全探索域・全長NCC**、`band_gain` は26帯域×約915窓×2チャンネルの**局所ラグ選択付きNCC・最小二乗ゲイン**が支配的である。

実装・計測資料、汎用の相関・時系列OSS（STUMPY、OpenCV、SciPy、FFTW）と音声・音響OSS（librosa、Essentia、aubio、pyfar、Google ViSQOL、pyroomacoustics、Chromaprint、audfprint、madmom、Rubber Band）の実装・公開資料を比較した結果、次を提案する。

1. **最初に試す：NCCの有効ラグ専用FFT長**。STUMPY実装に実例があり、`S + L - 1` ではなく `S` 以上のFFT長で、テンプレートが完全に収まる有効ラグの分子を計算できる。`band_ncc`・`full_ncc` の一部形状に効く。ただし **GCC-PHATには無条件に適用不可**。入力形状によってはFFT長が変わらず効果がない。
2. **同時に比較：Direct SIMD / Valid FFT / Blocked FFT の適応選択**。SciPy、OpenCVの実例を踏まえ、テンプレート長・有効ラグ数・メモリ制約に応じて方式を選択する。固定閾値は先に決めず、実測でキャリブレーションする。
3. **`band_gain` のメモリ構造を改善：Streaming Bandpass**。source/capture全26帯域の時間波形を保持せず、全長forward FFTを共有し、帯域ごとにIFFT→窓解析→破棄する。既存のスペクトルマスクの意味、帯域数、全窓の証拠を保持できる。ピークメモリには大きな削減余地があるが、これだけでIFFT/NCCの計算回数は減らない。
4. **`band_gain` の局所NCCを実験：Blocked FFT**。窓長4,800・探索信号8,640・有効ラグ3,841では現行FFT16,384の代わりにFFT8,192の2ブロック案を試せる。単純な`N log N`モデルでFFT演算規模が約23%減る可能性があるが、実時間の改善率ではない。
5. **音声専用OSSからの知見は別系列として扱う**。librosaのmultirate IIR、pyfarのSOS/FIR 1/3オクターブ、ViSQOLの段階的整列・帯域パッチ比較、Essentia/aubioのスペクトル集約は有望。ただし、**既存NCC/M2を同じ数値のまま高速化する実装とは限らない**。新推定器・追加特徴量として別バージョン管理する。

**重要な判定**：現行の帯域別NCCは「**各帯域で正規化した相関を加算**」する。各帯域の周波数スペクトルを単純に合成してIFFTを1回にする方法は、原則として同値ではない。`band_gain` の局所窓切り出しも周波数領域での無条件の演算融合を妨げる。

---

## 1. 調査範囲・根拠・確実性

### 1.1 一次資料

AudioLens内部：

- [Issue #71](https://github.com/scarecrow173/audio-lens/issues/71)
- [計測資料：`reference-analysis-analyze-pair-band-gain.md`](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/reference-analysis-analyze-pair-band-gain.md)
- [`al-analysis/src/band_gain.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-analysis/src/band_gain.rs)
- [`al-locate/src/methods.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/methods.rs)
- [`al-locate/src/prepared.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/prepared.rs)
- [`al-locate/src/stage.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/stage.rs)
- [`al-locate/src/pool.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/pool.rs)
- [`al-math/src/correlation.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/correlation.rs)
- [`al-math/src/filter.rs`](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/filter.rs)
- [定位仕様](https://github.com/scarecrow173/audio-lens/blob/main/docs/specs/source-localization.md)・[reference仕様](https://github.com/scarecrow173/audio-lens/blob/main/docs/specs/reference-analysis.md)
- [既存 #40 調査](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/issue-40-locate-pair-plan-cache.md)・[既存 #53 調査](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/issue-53-local-lag-fft.md)

OSS/公開資料は末尾の**参考文献・実装リンク**に集約した。

### 1.2 根拠の区分

| 記号 | 意味 | この文書での扱い |
|---|---|---|
| **[確認]** | ソース実装、Issueまたは既存の計測文書から確認可能 | 現状把握の根拠 |
| **[数学]** | 定義・仮定の下で数式として成立 | 実行時間や浮動小数点bit一致は含まない |
| **[試算]** | 入力形状・理論演算量・メモリ占有の概算 | 実測値として扱わない |
| **[仮説]** | 高速化効果、ユースケースでの有効性の見込み | 試作・検証が必要 |
| **[変更]** | 既存推定量・契約からの変更を伴う | 別ID・バージョン・比較実験が必要 |

外部OSSの「機能が存在すること」と、「AudioLensで高速化できること」は別の主張である。**本調査では移植実装も追加ベンチマークも行っていない**。

---

## 2. AudioLensの現行計算と実測ボトルネック

### 2.1 既存ジョブの計測結果 [確認]

実サンプルはおよそ**46秒、48kHz、`dropout.avi` + `normal.avi` + `source-bgm.wav`**。Issue #71 と計測資料に記載された値である。

| 対象 | 既存計測 | 内容 |
|---|---:|---|
| `locate::band_ncc` | **174秒 busy / 16呼出**、平均約10.9秒/呼出 | locate 全体の大きなコスト。帯域別の全探索域NCC |
| `reference::band_gain` | **約23秒 / 2ch pair** | reference `analyze_pair` 約27.5秒/pairの約83% |
| `band_gain::prepare_with_bands` | **約4.35秒/ch** | 全区間26帯域の spectral-mask/IFFT + coarse resample |
| `band_gain::windows` | **約6.92秒/ch** | 26帯域×約915窓のFFT-NCC |
| `band_gain` 内のその他 | **約0.18秒/ch** | coherence・レコード選択等 |

**注意**：`busy` は処理に費やした時間の計測値であり、そのままジョブ全体のwall-clock時間として足し引きできない。測定対象・並列構成・計装範囲を揃えて比較する。

計測資料により、I/Oが恒常的な支配ボトルネックという仮説は棄却されている。ただしdirty-page throttling等の環境影響による一過性変動はあり得る。比較実験ではCPU busy・wall・I/O時間を分ける。

### 2.2 `band_ncc`：全探索域の帯域別NCC [確認]

1. `prepared.rs` がNyquistでクリップした固定帯域（概ね5帯域）を準備する。
2. `InputCache` は入力・ch・レート・版・フィルタ設定で帯域波形をキャッシュする。
3. 各有効帯域について `CorrelationPool::ncc(t_b, s_b, max_lag)` を実行する。
4. **帯域別正規化を先に行った相関曲線を加算**し、候補ピーク・背景・競合・曖昧性を評価する。

\[
C(\tau)=\sum_{b\in B_{\mathrm{active}}}\frac{\sum_{i=0}^{L-1}x_b[i]y_b[i+\tau]}{\sqrt{\bigl(\sum_i x_b[i]^2\bigr)\bigl(\sum_i y_b[i+\tau]^2\bigr)}}.
\]

- テンプレートエネルギーが所定以下の帯域は無視する（`MIN_ENERGY=1e-9`）。
- バンドごとの相関曲線の和を**帯域数で除算しない**。
- 候補のピーク対背景比・ピーク幅・順位・極性・曖昧性判定が後段の契約を構成する。
- 既存 #40/#52 で入力帯域キャッシュと相関plan/workspace再利用は進んでいる。#42でSIMD系の基盤最適化も対応済み。
- `stage.rs` にはワーカー単位の `run_pool` があり、単純な外側並列化は未着手ではない。

### 2.3 `band_gain`：局所ラグ選択付きM2 [確認]

既定設定（48kHz時）：

| パラメータ | 値 |
|---|---:|
| 帯域数 | 40Hz〜16kHz内の1/3オクターブ、計測例では26帯域 |
| 窓長 `W` | 0.1秒 = **4,800サンプル** |
| hop `H` | 0.05秒 = **2,400サンプル** |
| 最大ラグ `R` | 0.04秒 = **1,920サンプル** |
| 窓ごとの相関対象 | `x[start:start+W]` 対 `y[start-R:start+W+R]` |
| 有効ラグ数 | `2R+1` = **3,841** |
| 現行 `CorrelationPlan` FFT長 | **16,384** |
| おおよそのNCC呼出 | 26帯域×約915窓×2ch = **約47,600回/pair** |

各帯域・窓で絶対相関ピークを求め、同点許容差での選択順を保ち、ラグを選択した後で

\[
 g_b(t)=\frac{\langle x_b[t:t+W],\,y_b[t+\tau_b(t):t+\tau_b(t)+W]\rangle}{\langle x_b[t:t+W],\,x_b[t:t+W]\rangle}
\]

を計算する。

主な既存契約：

- 窓の始端を `guard + R` だけ空け、末尾でも除外する。`guard=ceil(sample_rate / band_lower_hz)`。
- 最大 `abs(corr)` の候補から、`1e-12 * max(peak, 1)` の同点許容差内のものを選ぶ。`abs(lag)`、次に固定の符号順でタイブレーク。
- gain分母エネルギーが `1e-20` 以下ならNaN。相手のエネルギー・符号・境界ラグ・閾値により `silence`、`search_edge`、`polarity_unsupported`、`low_correlation`、`supported` を区別する。
- `support`、window start、lag、gain、correlationを保持し、帯域ごとに窓数が異なる場合はTensorをNaN/0でパディングする。
- 正負極性を分けて、相関絶対値上位の `ceil(個数/2)` を採用し、**符号付きgainの中央値**を取る。
- 400Hzアンカーは同じ帯域・同じ極性グループの結果に限定する。
- 帯域 coherence と correlation による validity 判定、`ref.band_gain_m2`、`ref.band_lag_ls.v1`、`ref.gain_400hz` および中間Tensorを保持する。

このため、単純な**帯域パワー比**、**Pearson相関**、**全帯域共通のラグ**は同じ推定器ではない。

### 2.4 メモリの構造問題 [試算]

`band_gain` は `InputCache` を関数内に作り、全26帯域のsource/capture波形を保持する。1チャンネル・1入力あたりの `f64` 波形データ容量は、入力長2,205,000サンプルで：

\[
26\times2{,}205{,}000\times8=458{,}640{,}000\ \mathrm{bytes}\approx459\ \mathrm{MB}.
\]

| 条件 | 帯域波形だけの概算 |
|---|---:|
| 1入力×1ch×26帯域 | 約459 MB |
| source+capture×1ch | 約917 MB |
| source+capture×2ch | 約1.83 GB |

`InputCache` は `analyze()` 中に保持されるため、2ch分を蓄積し得る。これは**波形ベクトル本体の試算**であり、FFT workspace、キャッシュ内のオブジェクト、ほかの分析データ、OSのピークRSSとは異なる。さらに `prepare_with_bands` が算出する `coarse` resample は、現在の `band_gain` では使われていない。

---

## 3. OSS実装・プロジェクト設計思想の詳細調査

本章は、**プロジェクトが本来何を解くために設計されているか**と**実コードから確認できる計算構造**を個別に検討する。調査対象は従来の12 OSSにmadmom・Rubber Bandを加えた14 OSSである。手法の優劣は「音声ライブラリかどうか」ではなく、**AudioLensの測定契約とどこまで同じか**で判断する。

対応するOSS単独の報告書：[`audiolens_issue71_oss_deep_dive.md`](audiolens_issue71_oss_deep_dive.md)（同日付）。

### 3.1 OSSの位置付けと成熟・依存関係

GitHub `repository` メタデータ（2026-10-17取得）に記録された `pushed_at` を付記する。**これはデフォルトブランチや関連refへのGitHub上の最後のpushを示すメタデータであって、正式リリース日・運用品質・セキュリティ保守を保証しない**。数値は変わるため採用時に再確認する。

| OSS | 主目的・方向性 | 主言語 / インターフェース | ライセンスの要点 | GitHub push日時（UTC） | Issue #71との関係 |
|---|---|---|---|---|---|
| [librosa](https://github.com/librosa/librosa) | 音楽情報検索・音声解析の研究/特徴抽出 | Python、NumPy/SciPy | ISC | 2026-10-06 | Multirate、CQT、DTW、STFTメモリ管理 |
| [Essentia](https://github.com/MTG/essentia) | 音楽音響の幅広い高性能解析エンジン | C++、Python/JS | **AGPL-3.0**、別途商用ライセンス | 2026-10-14 | Standard/Streaming、スペクトル帯域演算 |
| [aubio](https://github.com/aubio/aubio) | 低遅延・因果的な音楽イベント検出 | C、Python | GPL-3.0（GitHub判定） | 2026-04-10 | 小粒度FFT/フィルタバンク・イベント特徴 |
| [pyfar](https://github.com/pyfar/pyfar) | 音響測定・研究での再現性と特性の可視化 | Python、SciPy | MIT | 2026-10-16 | IEC系1/3oct、SOS/FIRの別推定器 |
| [ViSQOL](https://github.com/google/visqol) | 人間が感じる劣化音質の客観予測 | C++、Python API | Apache-2.0 | 2025-05-17 | 全体整列→周波数・時間パッチ比較 |
| [pyroomacoustics](https://github.com/LCAV/pyroomacoustics) | マイクアレイ・室内音響のアルゴリズム試験台 | Python/C++ | MIT | 2026-07-17 | GCC-PHAT、TDOA、適応フィルタ |
| [STUMPY](https://github.com/stumpy-dev/stumpy) | 時系列近傍探索/Matrix Profileの高速計算 | Python/Numba/GPU | BSD-3-Clause（README） | 2026-10-16 | **Valid FFT NCCの分子**、STOMP型更新 |
| [SciPy](https://github.com/scipy/scipy) | 汎用の検証済み数値計算アルゴリズム基盤 | Python/C/C++/Fortran | BSD-3-Clause | 2026-10-16 | Direct/FFT選択、Overlap-Add |
| [OpenCV](https://github.com/opencv/opencv) | 画像解析での高速テンプレート照合 | C++、多言語API | Apache-2.0（現行repo） | 2026-10-16 | Blocked DFT、非中心化 NCC |
| [FFTW](https://www.fftw.org/) | 多様なFFT形状を効率よく計算する最適化基盤 | C | GPL系・商用ライセンス条件要確認 | 2026-10-15（ミラー） | 計画・バッチ・メモリ配置、演算方式比較 |
| [Chromaprint](https://github.com/acoustid/chromaprint) | ほぼ同一の録音を識別するコンパクトな指紋 | C/C++ API | 自前MIT、同梱部を含め**全体はLGPL-2.1相当**と公式記載 | 2026-10-15 | 候補位置をハッシュから提案 |
| [audfprint](https://github.com/dpwe/audfprint) | 雑音を含む部分録音をランドマークで照合 | Python | MIT | 2019-09-24 | 素材照合の候補生成、NCC非代替 |
| [madmom](https://github.com/CPJKU/madmom) | 音楽イベント解析・MIRの参照パイプライン | Python/Cython | コードBSD、**モデルはCC BY-NC-SA 4.0** | 2026-03-20 | onset/beat、欠落・タイミング特徴の補助 |
| [Rubber Band](https://github.com/breakfastquay/rubberband) | 高品質なtime-stretch/pitch-shift処理 | C++、C API | GPL-2.0-or-later / 商用ライセンス | 2025-03-03（GitHub mirror） | 揺れ/伸縮のテスト信号生成、検出器自体ではない |

**ライセンスに関する注意**：上表は公開情報の技術調査であり法務判断ではない。特に Essentia、aubio、FFTW、Chromaprint、Rubber Band、madmomのモデルは**コードを読んで設計を学ぶこと**と**実装をコピー/リンク/モデルを配布すること**で条件が異なる。採用時には依存先も含め、ライセンス文書を個別検証する。

### 3.2 音声・音楽・音響を中心とするOSSの詳細

#### 3.2.1 librosa — MIR向けの柔軟な分析・実験プラットフォーム

**プロジェクト全体の方向性。** 音楽情報検索（MIR）を構築するためのPythonライブラリ。音声読込、STFT、Mel/CQT/Chroma、MFCC、onset、beat、テンポ、音調、DTW、分離系の分析構成要素を広範に提供する。科学技術計算の道具を組み合わせて試すのに向き、同一FFTを繰り返し実行する低レベルC/Rust実装の代替として設計されているわけではない。 [概要](https://github.com/librosa/librosa)・[公式API](https://librosa.org/doc/latest/)

**アルゴリズムと具体コード。**

1. [`iirt` — `core/spectrum.py`](https://github.com/librosa/librosa/blob/main/librosa/core/spectrum.py)：半音単位などのフィルタ群に対し必要なサンプルレートへ再サンプリングし、IIR `sosfiltfilt` / `filtfilt` を適用後、短時間平均二乗パワー（STMSP）を出す。コードでは再サンプルを同レート帯域で共有するため `np.unique(sample_rates)` を用いる。既定はMIDI 24–108の85半音帯域に近い設定で、**1/3オクターブではない**。APIの目的は**band power TF表現**であり、帯域波形に対する全ラグ相関ではない。[iirt API](https://librosa.org/doc/main/generated/librosa.iirt.html)・[filterbank API](https://librosa.org/doc/main/generated/librosa.filters.semitone_filterbank.html)
2. [`stft` — `core/spectrum.py`](https://github.com/librosa/librosa/blob/main/librosa/core/spectrum.py)：窓フレーム配列を参照しつつ `MAX_MEM_BLOCK`から一度に実行する FFT 列数を決め、`scipy.fft.rfft`でブロック処理する。**バッチサイズ/メモリ予算を実行計画に反映する**良い実例。
3. [`cqt`](https://librosa.org/doc/latest/generated/librosa.cqt.html)：対数周波数間隔・帯域依存の時間分解能を持ち、実装系によって再帰的ダウンサンプリングを使う。音楽的な周波数構造には向くが、既存1/3oct矩形フィルタの置き換えではない。
4. [`sequence.dtw`](https://librosa.org/doc/latest/generated/librosa.sequence.dtw.html)：subsequence matching、局所ステップやグローバル帯制約を備え、全体アライン後に時間伸縮を調べる参考になる。入力はChromaなどの特徴行列で、原波形のサンプル精度NCCとは別物。
5. [`filters.mel`](https://github.com/librosa/librosa/blob/main/librosa/filters.py)：FFTビンからMel帯域への重み行列。音響特徴を共通STFTから分配する構造を学べる。

**AudioLensへの移植可能性。** `band_gain` の高価な26帯域IFTTをそのまま減らすのではなく、(a) スペクトルだけで足りる低コスト帯域特徴と、(b) 独立にラグを探索するM2を分離、(c) M2の**別モード**として帯域ごと低レート相関を試作、(d) 入力/作業メモリ予算を厳密に管理、という4点が有用。

**非互換・限界。** `sosfiltfilt`は元の全長矩形 spectral mask と帯域応答、端点処理、サンプル位置が異なる。`iirt`出力はパワーであり、`gain = dot(a,b)/sum_sq(a)`もラグの符号/位相も失う。M2との比較には**同じフィルタ応答であるという前提を置けない**。

**評価**：設計知見◎／M2互換高速化△／M2とは別の高速解析◎。採用候補は**API設計とメモリ管理・multirateの原理**であって、パッケージ全体の移植ではない。

#### 3.2.2 Essentia — 大規模な音楽解析を構成するC++アルゴリズムエンジン

**プロジェクト全体の方向性。** Music Technology Group（UPF）発の音声・音楽解析ライブラリ。Spectral/Temporal/Tonal/Rhythm/Music descriptors、入出力、集計、学習モデル統合などの**大きなアルゴリズムカタログ**を備える。公式の設計方針は、正確性、使いやすさ、コード保守性、性能、アルゴリズム範囲の順で重視。研究の対話的試作と大規模バッチ抽出の両方を想定し、C++本体とPython/JSなどのインターフェースを提供する。[設計思想](https://essentia.upf.edu/introduction.html)・[全体](https://essentia.upf.edu/)

**特に参考になるアーキテクチャ。**

- **Standard mode**：呼出側がデータ確保と処理タイミングを制御する命令的API。データコピーを抑え、特定カーネルの性能検証がしやすい。
- **Streaming mode**：Source/Sinkを接続して有向グラフ化し、スケジューラに段階間のデータ送受信を任せる。同じ演算実装を可能な限り共有しながらパイプラインを組む。AudioLensのCheap/Reference/Expensiveという段階、特徴量共通中間表現、バッファ寿命の分離に非常に近い**設計参照**。[Streaming architecture](https://essentia.upf.edu/streaming_architecture.html)
- `BarkBands`、`ERBBands`、`EnergyBand`などでは、入力スペクトルから帯域強度/エネルギーをまとめて計算する。[アルゴリズム一覧](https://essentia.upf.edu/algorithms_overview.html)・[ERBBands](https://essentia.upf.edu/reference/streaming_ERBBands.html)
- [`CrossCorrelation`](https://github.com/MTG/essentia/blob/master/src/algorithms/standard/crosscorrelation.cpp)は対象ラグの直接積和で、巨大な全域相関や `band_gain` の約47,600回のFFTを削減する完成済み手法ではない。

**AudioLensへの移植可能性。** FFTを共有する複数の「スペクトル集約だけで良い」特徴量を1パスへ合成し、M2等の波形・位相を要する特徴とは計算パスを分ける。**入力データ取得/共通スペクトル/窓フレーム/FFT/特徴・recordsへの分配をDAGとして明示する**点で、EssentiaのStreaming設計は重要。ただし `band_gain` の既存 `Window.reason` や `support` は別途実装する。

**非互換・限界。** `ERBBands` や `BarkBands` はエネルギー表現を返し、帯域ごとの全域NCCや局所LSゲインには直結しない。**AGPL-3.0**はコード移植・直接依存時の重要条件。設計から得られたパターンを独自に実装する方針と、ソース流用は明確に区別する。[ライセンス](https://essentia.upf.edu/documentation.html)

**評価**：**パイプライン設計◎**／スペクトル特徴最適化◎／NCC・M2カーネルの直接改善△。

#### 3.2.3 aubio — 因果的・低遅延の音楽イベント検出

**プロジェクト全体の方向性。** Cで書かれた低遅延の音楽イベント検出ライブラリ。onset、pitch、beat/tempo、MFCC、相対的に軽量なFFTや音声フィルタを備える。開発者は因果的（未来サンプルに依存しない）な演算に重きを置き、ライブ処理/楽器/エフェクト用途を重視する。AudioLensの**オフラインかつ全証拠保存**とは優先順位が異なる。[プロジェクト紹介](https://aubio.org/)・[マニュアル](https://aubio.org/manual/latest/index.html)

**具体コード。** [`src/spectral/filterbank.c`](https://github.com/aubio/aubio/blob/master/src/spectral/filterbank.c) の `aubio_filterbank_do()` は、入力FFTの振幅ベクトル（必要なら冪）をフィルタ係数行列 `fmat_vecmul` で複数帯域へ射影する。帯域数ぶん全長IFFTをしない理由は、**出力がエネルギー/スペクトル特徴で、時間波形ではないから**。`src/spectral/fft.c`、`src/pitch/`や`src/onset/`は軽量フレーム処理の参考。

**AudioLensへの移植可能性。** 既に存在するSTFTからMel/Bark系の複数帯域特徴をまとめて計算し、オンセット候補をCheap featuresへ追加して異常検知の窓を提案すること。リアルタイムデータ構造は将来のストリーミング対応を検討するときに参考になる。

**非互換・限界。** フィルタバンク係数が与える帯域パワーを、各帯域のNCCやラグ補償LSゲインに置き換えてはいけない。イベント検出は既存数値を再現する性能最適化には直接寄与しない。ライセンスはGPL系。

**評価**：Cheap features拡充○／低遅延DSP設計○／#71の支配カーネル置換△。

#### 3.2.4 pyfar — 音響測定でのフィルタの意味・再構成性・追跡可能性

**プロジェクト全体の方向性。** 音響測定・研究を重視するPythonエコシステム。Signal/TimeData/FrequencyData、Filters、座標・指向性表現、評価・可視化・音響機器との接続を統一的に扱う。文書化、テスト、データ構造の意味を重視する点は、AudioLensの構造化結果保存と相性が良い。[公式サイト](https://pyfar.org/)・[API体系](https://pyfar.readthedocs.io/en/stable/api_reference.html)

**2種類の重要なフィルタバンク。**

1. [`fractional_octave_bands`](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html)：IEC 61260-1:2014 に対応する条件で設計された**Butterworth SOSの-3dBカットオフ**を持つエネルギー保存指向のフィルタ群。有限次数やNyquist近傍には例外がある。
2. [`reconstructing_fractional_octave_bands`](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html)：**-6dB、線形位相FIR、`n_samples/2`の遅延**という別設計。加算再構成（振幅保存）を意識する。両者は「fractional octave」という名前でも測定結果が一致しない。[実装](https://github.com/pyfar/pyfar/blob/main/pyfar/dsp/filter/fractional_octaves.py)

**AudioLensへの移植可能性。** `ref.band_gain_m2` と `ref.band_*` に対して、**フィルタ方式ID、振幅応答、群遅延、サンプルレート、端点処理、guardの設計を別々に記録する**べきことが明確になる。既存の矩形spectral maskを残し、音響規格に近い帯域測定は**別feature family**として実装・比較するのが望ましい。

**非互換・限界。** SOSの位相/過渡・FIRの遅延は現在の矩形マスクと異なる。帯域ゲインや相関のvalidityが変わり得る。「フィルタバンクの形式が一般的」という理由だけでM2の出力上書きはできない。

**評価**：音響計測の仕様設計◎／1/3oct別経路◎／互換性維持の直接高速化△。

#### 3.2.5 Google ViSQOL — リファレンス音声の知覚的劣化評価

**プロジェクト全体の方向性。** ViSQOL (Virtual Speech Quality Objective Listener) は、参照音声と劣化音声の**時間周波数構造**の類似性から、知覚音質（MOS-LQO）の予測を出すフルリファレンス評価器。音声/音楽等のモードでモデル、推奨サンプルレート、内部処理が異なる。公式READMEによればaudio modeは48kHzを期待し、多チャンネル入力は比較時にmonoへdownmixする。speech modeは16kHzを期待する。**出力の目的は知覚品質予測で、発生した音声バグのメカニズムを分解して記録するAudioLensとは違う。** 公式READMEはコーデックやVoIP劣化評価を主要用途とし、他用途では性能に限界があると明記する。[README](https://github.com/google/visqol)

**ソース確認されたパイプライン。**

- [`src/alignment.cc`](https://github.com/google/visqol/blob/master/src/alignment.cc)：全体の音声包絡（upper envelope）相互相関から粗い全体整列。
- [`src/gammatone_spectrogram_builder.cc`](https://github.com/google/visqol/blob/master/src/gammatone_spectrogram_builder.cc) と [`src/gammatone_filterbank.cc`](https://github.com/google/visqol/blob/master/src/gammatone_filterbank.cc)：Gammatone/ERBの周波数方向へ時間フレームのフィルタ信号を集約し、帯域パワーで比較可能な表現へ。
- [`src/comparison_patches_selector.cc`](https://github.com/google/visqol/blob/master/src/comparison_patches_selector.cc)：参照パッチごとに探索窓内の候補パッチを比較し、累積類似度とbacktraceで**時間順序・途中スキップ**を扱う（DTWに近いDP）。
- 音質スコア学習/マッピングとパッチ表現の生成は上記とは別ステップ。

**AudioLensへの移植可能性。** 素材位置の粗整列→時間・帯域の局所比較→異常区間で精密解析という段階化。`band_gain` が毎窓±40ms全探索を繰り返すコストを減らす**別経路**として、共通ラグ候補/前窓予測の周りから段階的に探索する案はある。

**重大な制約。** AudioLensは**帯域ごとに独立した遅延**も異常として保存する。ViSQOL風に全帯域共通の整列結果へ強制固定すると、周波数依存遅延、位相異常、途切れの証拠が消える。またViSQOLはスコアを出すために多くの特徴を集約し、低相関帯域のM2正当性を保証しない。公式audio modeのmono downmixは、AudioLensが重視するsource→L/R独立の異常解析の代用にならない。現在のgammatone実装はフレームごとにフィルタ状態を初期化する部分があり、そのカーネルが特に高速とは言えない。

**評価**：段階化・局所整列の設計◎／互換性維持のM2高速化△／**別の知覚品質参考指標**○。

#### 3.2.6 pyroomacoustics — 室内音響シミュレーションとマイクアレイDSPの研究基盤

**プロジェクト全体の方向性。** 複数音源・マイク・反射を伴う室内音響のシミュレーションと、beamforming、DOA（到来方向）推定、適応フィルタ、分離、ノイズ除去等を**比較可能な参照実装として提供する**ことが主目的。C++の伝搬計算とPythonの実験用APIで構成する。[公式Summary](https://pyroomacoustics.readthedocs.io/en/stable/)

**関連ソース。** [`pyroomacoustics/sync.py`](https://github.com/LCAV/pyroomacoustics/blob/master/pyroomacoustics/sync.py) の `tdoa()` と `correlate()` はFFTでcross-spectrumを作り、オプションのPHAT正規化を施して lag を求める。STFTエンジンやadaptive filteringは独立モジュールに切り分ける。

**AudioLensへの移植可能性。** ゲームのcaptureが複数ch/残響/クロストークを含む場合、遅延推定とチャネル混合・gain・残差推定を別の推定器として並立させる指針になる。現行のGCC-PHATやmix解析との整合チェックに使える。

**非互換・限界。** PHATはスペクトル振幅で重みを正規化する別の統計量。`band_ncc`の各帯域非中心化NCCや、M2のゲイン・supportを計算するわけではない。室内伝搬を想定した典型例は、ゲームのデジタルキャプチャ障害とそのまま一致しない。

**評価**：チャネル/遅延設計○／基準実装とのA/B評価○／#71直接の高速化△。

### 3.3 高速相関・時系列検索・FFTに特化したOSSの詳細

#### 3.3.1 STUMPY — Matrix Profile とSliding Dot Productの高速アルゴリズム

**プロジェクト全体の方向性。** あらゆる時系列に対する近傍部分系列・motif・discord・異常発見・セグメンテーションを高速に求めるPython/Numbaのライブラリ。`mass`（query-to-seriesのdistance profile）や `stump`（Matrix Profile）を中心に、高性能計算をアルゴリズム化している。音響専用ではないが、AudioLensの**スライディング相関を高速に計算したい**という問いには、汎用音声特徴OSS以上に直接的な先例がある。[公式概要](https://stumpy.readthedocs.io/en/latest/)・[API](https://stumpy.readthedocs.io/en/stable/api.html)

**具体実装。**

- [`stumpy/sdp.py`](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/sdp.py) は `n=len(T)`、`m=len(Q)` に対して `next_fast_len(n)` でFFT長を選び、反転したqueryとTの**循環畳み込みのvalid部分 `[m-1:n]`**を切り出す。全線形畳み込みのための`n+m-1`を使わない。
- 同ファイルには、直接演算（Numba）、SciPy直接/FFT自動選択、Overlap-Add、pocketfft、pyFFTWキャッシュ実装があり、**同一APIで実行方法を比較**する作りになっている。
- [`stumpy/stump.py`](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/stump.py) は窓相関を対角線上で更新する STOMP 系アルゴリズムを使用。隣接窓の相関の積和を引き継ぐ。

**適用の核心（数理的に適用可能）。** `band_ncc` の *分子* はNCC有効ラグの範囲で FFT長`nextpow2(S)`の循環相関として計算できる。既存の`CorrelationPlan::new`の`nextpow2(S+L-1)`より短くなるケースでは、FFT作業領域と演算量が減る。*分母*は現行どおりテンプレートエネルギーとスライディングキャプチャ窓エネルギーで計算する。

**境界条件。** STUMPYの通常のMatrix Profileは**z-normalized（平均を引くPearson系）**で、AudioLensの `corr0` は**平均を引かない非中心化NCC**。分子のSliding Dot Productだけを移植すべき。`gcc_phat`のFFT長を同様に短縮するとPHATの周波数重みが変わるため、GCC-PHATには流用しない。

**STOMPがM2に向かない理由。** M2のデフォルトはhop=窓長の半分・最大ラグ±1920で、各時刻/ラグの直積更新は概算`2.205e6×3841≈8.47e9`箇所/帯域/chとなる。ラグが非常に狭く、hopが極端に小さい設定でのみ比較対象。演算回数とメモリ帯域の実測が必要。

**評価**：**band_nccのValid FFT P0**／`band_gain`のSTOMP方式P3。

#### 3.3.2 SciPy — 演算形状から数値カーネルを選ぶ基盤

**方向性。** Python科学計算向けの幅広い検証済みアルゴリズム群。`signal`は波形処理・畳み込み・FFT・フィルタを含み、**正しさと汎用性を保ちながら、入力条件で最も適切な方法を選ぶ**設計が目立つ。[SciPy signal](https://docs.scipy.org/doc/scipy/reference/signal.html)

**関連APIとコード。**

- [`convolve`](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.convolve.html)：`direct`、`fft`、`auto`を公開。
- [`choose_conv_method`](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.choose_conv_method.html)：入力形状・dtype等から推定し、`measure=True`なら実機でdirect/FFTを計測して選ぶ。
- [`fftconvolve`](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.fftconvolve.html)：大きい信号向けだが、**必要な出力点が少ない場合は非効率**になり得る。
- [`oaconvolve`](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.oaconvolve.html)：長さが著しく異なる入力でOverlap-Addを使い、ブロックサイズを選ぶ。
- [実装 `_signaltools.py`](https://github.com/scipy/scipy/blob/main/scipy/signal/_signaltools.py)：`next_fast_len`とコスト比較、ブロック計算の分岐。

**AudioLensへの移植可能性。** `(template_len, signal_len, n_lags, operation_kind, memory_budget)` を実行形状として、Direct SIMD、Valid FFT、Blocked FFTの選択を決める。単に全件をFFTにするより、equal-length/狭ラグで有利。`band_gain`の局所窓も同じdispatch思想で比較できる。正規化とpeak/evidence処理は共通に保持する。

**非互換・注意。** SciPyの畳み込みはNCCでないため、分母とゼロエネルギーガードは別実装。floatにNaN/InfがあるFFT畳み込みは汚染しうる。分母のprefix sumも長尺無音・スケール差の下では差分丸めを検証する。

**評価**：**Adaptive Dispatch P0**／Blocked FFT P1。

#### 3.3.3 OpenCV — 高速テンプレート照合でのブロックDFT設計

**方向性。** 高性能な画像処理・特徴抽出/認識のC++ライブラリ。CPU/最適化バックエンド/GPU等を用意し、大量の画素に対する相関と正規化を効率的に実行する。音声ではないが、**有効位置での正規化テンプレート照合**は1次元化するとAudioLensのNCCと非常に近い。[テンプレート照合の数式](https://docs.opencv.org/4.x/de/da9/tutorial_template_matching.html)

**確認できる実装。** [`modules/imgproc/src/templmatch.cpp`](https://github.com/opencv/opencv/blob/4.x/modules/imgproc/src/templmatch.cpp) は、相関結果を複数タイルに区切り、テンプレートのDFTをキャッシュ的に利用しつつ、入力側の部分DFT、cross-spectrum、inverse DFTを行う。正規化には積分和・二乗和や別の演算経路を用いる。**`TM_CCORR_NORMED`が非中心化NCCの式に該当**し、`TM_CCOEFF_NORMED`は平均を引くため異なる。

**AudioLensへの移植可能性。** `band_ncc` の全域を分割して相関曲線を生成し、全ラグの候補順位・背景・端点を変更しない。`band_gain`の短窓ではFFT16,384に対し、8192×2ブロック案を実測比較できる。試算では窓内の3 forward +2 inverse（8192）対2 forward +1 inverse（16384）を`N logN`モデルで比べると約0.774。ただし実時間23%削減という意味ではない。

**非互換・注意。** OpenCVの画像境界処理、複数チャネル正規化、float32/float64、パディングはAudioLensの既存規則とは違う。**タイル境界のラグを二重計算/欠落しない**こと、NCCの分母が窓位置と一致すること、tie/峰形状を完全に保持することが重要。

**評価**：**Blocked FFT P1**。長尺ワークスペースの減少効果も評価対象。

#### 3.3.4 FFTW — FFTカーネルの計画・バッチと実行効率

**方向性。** FFTを高効率に処理するためのCライブラリ。1D/ND、実数/複素、batched（`plan_many` / guruインターフェース）、stride、in-placeなど、多様な形状を処理し、planによる実行効率を追求する。メインの価値は**FFT演算そのものの高性能化**であり、NCCや局所M2という高水準推定器を実装することではない。[FFTW公式](https://www.fftw.org/)・[高度なreal transforms](https://www.fftw.org/fftw3_doc/Advanced-Real_002ddata-DFTs.html)

**関連する設計知見。** 多数の窓の同一FFT長・stride付きデータをひとまとめに処理する計画。`band_gain`では約47,600の小FFTが反復されるため、バッチ処理とメモリ配置（SoA/AoS、行優先・列優先）を組み合わせる実験は理にかなう。

**制約。** FFTの回数・フィルタbankの本数・NCCの数学的コストを自動的に減らすわけではない。FFTWのplan生成コスト、メモリ、thread数、既存Rust `al-math` FFT実装とのベンチが先。GPL/商用オプション等の**配布条件を確認すること**。FFTWのGitHubリポジトリは自ら通常のソース配布用cloneと誤解しないよう注意を記しているため、公式の配布・文書を優先する。

**評価**：Plan reuseは実装済み／**batch・workspaceの検証 P2**。

### 3.4 音声指紋・音楽イベント・時間伸縮のOSS

#### 3.4.1 Chromaprint — 「ほぼ同一録音」を短い特徴で識別する

**方向性。** AcoustIDのための音声指紋ライブラリ。コンパクトな指紋を生成し、録音の識別・重複判定・長時間監視をする。**一般目的の強頑健指紋ではなく、検索速度のために精度/頑健性をトレードオフする**と明記している。[公式README](https://github.com/acoustid/chromaprint)・[AcoustID説明](https://acoustid.org/chromaprint)

**具体コード。** [`src/fingerprint_matcher.cpp`](https://github.com/acoustid/chromaprint/blob/master/src/fingerprint_matcher.cpp) は、指紋整数列中の一致部分から時間オフセット候補をヒストグラム化し、候補の一致度を確認する。全PCMの各ラグ相関を計算しない。

**AudioLensへの用途。** BGM等が多量にあるときに、候補素材ID・候補オフセットを先に出すための前段。実際の定位の正しさは、既存のfull/band NCCや他の独立証拠で評価する。ゲームSEのような短い素材や大きく加工された録音には、仕様上の優位を保証しない。AcoustID公式配布ページには**2026-10-15公開のChromaprint v1.6.2**が記載されており、少なくとも調査日時点で配布更新の一次根拠がある。[公式ダウンロード](https://acoustid.org/chromaprint)

**重要なライセンス注意。** 自前コードはMITだが、[公式LICENSE.md](https://github.com/acoustid/chromaprint/blob/master/LICENSE.md)は内包するFFmpeg由来部分のため、**全体をLGPL-2.1と考えるべき**と説明している。FFTバックエンドのライセンスも影響し得る。

**評価**：多素材候補の前処理○／既存のbandNCC evidenceの代替×。

#### 3.4.2 audfprint — ランドマークに基づく部分音声の検索

**方向性。** Dan Ellisの音声ランドマーク照合方式を基にしたPythonの音声指紋実装。周波数時間平面のピークを用いて頑健なハッシュ列を作り、データベースから**雑音のある短いquery**を検索することに狙いがある。[README](https://github.com/dpwe/audfprint)

**ソース構造。** [`audfprint_analyze.py`](https://github.com/dpwe/audfprint/blob/master/audfprint_analyze.py) が信号の時間周波数ランドマーク、[`audfprint_match.py`](https://github.com/dpwe/audfprint/blob/master/audfprint_match.py) がハッシュ/候補照合、[`hash_table.py`](https://github.com/dpwe/audfprint/blob/master/hash_table.py) がデータベース表現、[`audfprint.py`](https://github.com/dpwe/audfprint/blob/master/audfprint.py) がCLIワークフローを扱う。

**AudioLensへの用途。** BGM/多数候補/ミックス音声中の短いソース候補提案や、動的に変わる capture からのsource候補検索。ただし周波数ピークが弱い音や極短のSEでは偽陰性が増え得る。ピークハッシュは真のgain、反転極性、全ラグ背景、残差を提供しない。

**保守状況の留保。** 公開GitHub `pushed_at` は2019年で、研究OSSの参照実装としては有用だが、現行ランタイムとの互換・再現性はローカルで検証が必要。

**評価**：多素材候補の前処理○／既存のbandNCC evidenceの代替×。

#### 3.4.3 madmom — 時間イベントの高機能MIRパイプライン

**方向性。** beat・onset・downbeat・テンポ、音符等の音楽イベント認識に特化。Processorの連結とNumPy配列に基づく研究アルゴリズム実装、Cythonでの高速化、学習済みモデルを組み合わせる。[README](https://github.com/CPJKU/madmom)・[導入](https://madmom.readthedocs.io/en/latest/introduction.html)

**AudioLensへの用途。** 途切れたイベント、オンセットの時間シフト、beat同期不良などの検出について、Cheap/Expensive featuresの参考にできる。**現行の帯域別正規化相関を高速化するカーネルではない**。

**重大なライセンス区分。** ソースコードは原則BSDで、同梱のモデル・データは**CC BY-NC-SA 4.0**として別管理される。[ライセンス説明](https://github.com/CPJKU/madmom/blob/main/README.rst)　学習済みモデルを組み込む場合は用途を慎重に確認する。

**評価**：音楽イベント解析の追加候補○／#71の直接高速化×。

#### 3.4.4 Rubber Band — 時間伸縮/ピッチ変化を「生成」する実装

**方向性。** テンポとピッチを独立に変更するDSPライブラリ。R2（Faster）/R3（Finer）のエンジンを使い分け、音質・CPUコストのトレードオフを利用者へ露出する。これは**変換器であり、入力2本の対応を検出するアラインメント推定器ではない**。[公式ミラーREADME](https://github.com/breakfastquay/rubberband)

**AudioLensへの用途。** デバッグfixtureとして、音声の非線形時間伸縮、長さの変化、ピッチシフト、トランジェント付近の加工を人工的に生成し、`locate`/tracking/M2がそれをどう観測するか試験する。アルゴリズムを直接組み込むより、独立テスト生成ツールとして使うと利点が大きい。

**注意。** 変換器が作るstretch/音質変化はAudioLensの推定器の正しさそのものを保証しない。GPL-2.0-or-laterまたは商用ライセンスで配布条件を検討する。[ライセンス説明](https://github.com/breakfastquay/rubberband)

**評価**：golden・ストレス試験○／#71カーネルの直接高速化×。

### 3.5 OSSを横断して見えるアルゴリズム設計原則

### F.1 最大の論点は「推定量の同値性」

**互換性維持を目指せるもの**：STUMPYのvalid sliding dot productの分子、OpenCVのブロックFFTとエネルギー正規化、SciPyのdirect/FFT/Overlap-Add選択、librosa/Essentiaのバッファ寿命管理。

**別推定器になるもの**：librosa `iirt/CQT/DTW`、Essentiaやaubioのband power、pyfarのSOS/FIR、ViSQOLのパッチ音質、pyroomacousticsのPHAT、fingerprint、madmomのイベント識別。

同じ「帯域ごとの相関」のラベルを使っていても、次は違う。

\[
\operatorname{NCC}_b(\tau)=\frac{\sum_t x_b[t]y_b[t+\tau]}{\sqrt{\sum_t x_b[t]^2\,\sum_t y_b[t+\tau]^2}}
\quad\neq\quad
\text{band power ratio},\ \text{Pearson},\ \text{PHAT},\ \text{spectral patch similarity}
\]

かつ `band_ncc` は**各帯域で正規化してから加算**する。分子のcross-spectrumだけ先に帯域加算して、最終的に単一の分母で割る案は一般には非同値。

### F.2 全長FFT帯域マスクをIFFTなしで置き換えられる範囲

- **可能な特殊条件**：等長信号・同一FFTグリッド・同一マスク・全区間・線形演算などが揃うならParseval/周波数領域の相互スペクトル積から全区間の非正規化内積やLSの分子・分母を構成できる。
- **AudioLens現状で不可な一般化**：source/captureの入力長によりFFTグリッドが異なる、フィルタ後に元長へ切り戻す、各窓だけを切り出す、lagが帯域・窓別で変わる、zero guard・端点除外がある。**時間窓との乗算と周波数マスクは可換ではない**。
- したがって、Essentia/aubioの帯域パワー演算にM2全体をすり替えることはできない。

### F.3 「軽量経路の追加」と「既存解析全件実行」の論理的緊張

AudioLensの目的は最終判定でなく**解析・証拠保存**。軽量な指紋、Melパワー、共通ラグ候補を生成しても、フル帯域M2と全域NCCを常に計算する契約なら、その計算時間は減らない。速度改善には**既存計算の数学的再構成**、または**仕様で許された条件付き省略（別モードで、欠損とreasonを保存）**のいずれかが必要。

### F.4 望ましい新規カーネルAPIの候補

```text
CorrelationMode = NonCenteredNcc | GccPhat | RawCrossCorrelation
CorrelationEngine = DirectSimd | ValidFft | BlockedFft
PlanKey = {mode, template_len, signal_len, fft_len, layout}

BandFilterMode = LegacySpectralMaskV1 | OtherExplicitlyVersionedFilter
BandProcessing = StreamingBandPair | MaterializeAllBands

outputs:
  complete correlation curve for required lag domain
  -> unchanged existing evidence selector / peak statistics
  band windows (gain, corr, lag, support, reason, start)
  -> unchanged existing tensors / feature records
```

機能分離を徹底し、planからPHATの意味を暗黙に変えず、`band_gain`の局所ラグ契約を維持する。**別推定器**を作る場合には `filter_version`、`estimator_id`、`lag_resolution`、`phase_response`、`edge_policy`、`support_definition`を記録する。

### 3.6 課題別の比較と優先順位

| 課題 | 第一候補（根拠） | 第二候補 | 保留/別Issueとすべきもの |
|---|---|---|---|
| `band_ncc` FFTが過大 | **STUMPY Valid FFT**（ソースに実例） | SciPy adaptive direct/FFT | GCC-PHATと共通にFFT長を短縮 |
| 全探索相関が巨大・メモリが重い | **OpenCV-style Blocked FFT** | FFTスペクトルpair間共有 | 近傍候補だけ見て全域evidenceを失う案 |
| `band_gain` 窓FFTが約47,600回 | **SciPy/OpenCV風窓FFTブロック比較** | FFTW batch・窓間再利用 | STOMPを無検証で標準化 |
| 26帯域×入力2本×chを保持 | **Streaming Bandpass**（AudioLensコード由来） | memoria-bounded workspace（librosa/Essentia） | Filterbankを変えるだけで互換と主張 |
| 低域の帯域計算を減らす | **librosa multirate**（ただし別推定器） | pyfar SOS/FIR別方式 | 元のM2と同一ID/値で置換 |
| ソース候補が多数・雑音が多い | **audfprint / Chromaprint候補提案** | librosa CQT/subsequence DTW | NCC全域証拠を削除 |
| ゲームの揺れ/局所欠落 | **ViSQOL局所パッチ・DPの発想** | librosa DTW、madmom onset | 共通ラグで帯域ごとの遅延を上書き |
| 追加診断用帯域特徴 | **Essentia/aubioのSTFT→band pool** | SciPy H1/coherence、pyfar octave | band powerからM2に換算したとみなす |

---

## 4. 改善案の詳細と適用条件

### A. NCCの有効ラグ専用FFT長（STUMPY型） — **最優先** [数学]

現在の`CorrelationPlan::new`は

\[
N_{\mathrm{old}}=\operatorname{nextpow2}(S+L-1)
\]

を採用する。テンプレートが信号内に完全に収まる`0<=lag<=S-L`だけを計算するNCCなら

\[
\boxed{N_{\mathrm{valid}}=\operatorname{nextpow2}(S)}
\]

でよい。逆相関の出力インデックスとFFTの正規化係数は既存実装との比較で確定する。

**成立理由**：反転テンプレートとの線形畳み込みの必要区間は`L-1 .. S-1`。FFT長`N>=S`の循環畳み込みで発生する折り返しはインデックス`0 .. L-2`にしか現れない。必要区間には影響しない。

**例（46秒、48kHz、ほぼ等長の2信号）**：`S≈2,205,000、L≈S`ならFFT長8,388,608から4,194,304に落とせる。一方`L=10,000`なら、元のFFT長が既に4,194,304なので変化しない。

**注意**：

- FFT要素数50%削減 ≠ 実行時間50%短縮。
- この証明は**NCCの非正規化分子**の有効ラグについて。分母は別途既存ルールで計算。
- **GCC-PHATの周波数ビン位相正規化はFFT長依存**なので、同じ短縮を適用してはいけない。`CorrelationPlan`をNCC/PHATで分離するか、計画キーにモードを含める。
- 分子の数学的同等性と`f64`演算のbit完全一致は異なる。タイ近傍のargmax・背景・候補順位まで回帰試験が必要。

### B. Direct SIMD / Valid FFT / Blocked FFTの適応dispatch — **最優先** [仮説]

`K=S-L+1`個のNCCラグを計算するとき：

\[
T_{\mathrm{direct}}=O(KL),\quad T_{\mathrm{FFT}}=O(N\log N).
\]

- `K`が非常に小さい：直接内積 + 局所二乗和。
- 多数ラグ：有効ラグ専用FFT。
- 大型FFTに必要なメモリを抑えたい、またはキャッシュ効率が有利：blocked FFT（Overlap-Add/Save系）。

既存の`CorrelationPlan`では`N=0`や短いsignalといったガードがあり、`CorrelationPool`も形状キーで再利用する。方式選択を加える場合は`(演算種別, L, S, FFT長, block形状)`をキー候補とする。形式だけでなく**合計保持バイト数**の予算を設ける。

### C. `band_gain`のStreaming Bandpass — **優先** [数学・試算]

`bandpass_many`はすでに**入力1本につきforward FFT 1回 + 各帯域IFFT**。ここからforward回数をさらに削っても大きな改善とは限らない。代わりに、`band_gain`でsource/captureを各帯域ごとに逐次実体化して使い切る。

```text
source FFT (once)          capture FFT (once)
         \                 /
          +-- band b mask -+
                 |
          2 x band IFFT
                 |
         windows / lag / LS
                 |
          Window[]だけ保存
                 |
          次のbandへ（波形解放）
```

**得られるもの**：全26帯域波形の同時保持を、原則として1帯域のsource/captureへ削減可能。未使用`coarse`再サンプルの除外。既存フィルタ・全窓の出力を維持しやすい。

**得られないもの**：26帯域×2のIFFT回数そのもの、約47,600回の窓NCCはこの変更だけでは減らない。forward FFTのworkspace・スペクトル2本・出力Tensor等は引き続き必要。

**実装注意**：帯域順、端点guard、全体の集計・400Hzアンカー、Tensorパディングの順序を維持する。`InputCache`を全stageで変えるのではなく、reference側の`BandpassSource`/`BandIterator`等を追加する方式が安全。

### D. `band_gain`局所NCCのBlocked FFT — **優先試作** [試算]

既定`W=4800、R=1920`では`S=W+2R=8640、K=3841`。現行の`CorrelationPlan`はFFT長16,384で、テンプレートforward、入力forward、inverseを各1回行う。

FFT長8,192なら、1タイルがカバーできる有効ラグは`8192-4800+1=3393`。全3,841ラグは2タイルで処理可能。テンプレートforwardを共有すれば：

| 方式 | 必要なFFT（1窓） | `N log2 N`の単純積算 |
|---|---|---:|
| 現行 | 16,384点×3 | `3×16384×14=688,128` |
| 2タイル | 8,192点×5（template 1、signal 2、inverse 2） | `5×8192×13=532,480` |

比は約**0.774**、単純演算規模で約**23%減**。[試算]

実行時間はFFT実装・バッファ複写・SIMD・メモリアクセス・plan数で決まるため、予測できない。正規化の分母・端点・全ラグ出力は既存`CorrelationPlan::ncc`と一致させる。

### E. `band_gain`の窓間共有・STOMP型更新 — **限定試作** [数学・試算]

固定ラグの局所内積

\[
D_t(\tau)=\sum_{i=0}^{W-1} x[t+i]y[t+i+\tau]
\]

には、1サンプル移動時の更新式がある。

\[
D_{t+1}(\tau)=D_t(\tau)-x[t]y[t+\tau]+x[t+W]y[t+W+\tau].
\]

隣接窓間の再利用は理論上成立する。しかし、`H=2400`とラグ3841個で全サンプルを更新すると、1帯域・1chで`2,205,000×3,841≈84.7億`の時刻・ラグ更新（各更新に通常2積）となる。26帯域・2chでは約4,404億更新規模。[試算]

したがってSTOMP型の逐次更新を**既定モードの主たる高速化策に決める根拠は乏しい**。lag幅が小さくhopが短いワークロードに限定してベンチする。また、1窓を2半窓へ分割して相関を共有する方法は成立するが、`R`が大きいため各半窓相関のFFT長が十分小さくならず、結合コストが上回る場合がある。

### F. `band_gain`の「全信号上のprefix局所相関」 — **実装前に演算量評価が必須** [仮説]

分母の局所エネルギーはprefix二乗和で共有できる。一方、**分子はラグごとに異なる積`x[t] y[t+lag]`の時系列**である。全ラグの積を作ってprefix化するだけでは`O(NR)`の処理となり、既定形状では非常に大きい。Issueに記載された「窓NCCを全信号相互相関に再構成」という構想を、**全信号の相互相関を1本求めれば各窓相関も得られる**と解釈してはならない。

### G. 入力FFTのpair間再利用 — **再利用頻度を測ってから** [仮説]

`InputCache`は時間波形を共有するが、NCC用FFTスペクトルを入力ID・帯域・FFT長で共有しているわけではない。同じcaptureを多数のsourceと比較する場合、capture側forward FFTに重複が生じる可能性がある。

ただしFFT長に依存したキャッシュなので、巨大な複素スペクトルを多数保持するとメモリが膨らむ。**ヒット率、シリアル化・ロックの負担、ピークRSS、利用後解放**を定量化して採否を決定する。既存`MAX_SHAPES=8`は個数上限でありメモリバイト上限ではない。

### H. Pruned FFT / sparse-spectrum演算 — **研究枠** [仮説]

1/3オクターブ低域では非ゼロFFTビンが少ない。非ゼロビンのみ計算するpruned IFFTや周波数帯域間の演算共有は、理論的には機会がある。しかし汎用FFT実装の最適化・データ配置が強力で、必ずしも速くない。**同じ半開FFTビンマスクを再現できる**方式だけを互換候補とし、IIR/SOSなど別のフィルタ応答への置換とは区別する。

### I. librosa型multirate・pyfar型SOS/FIR・ViSQOL型局所パッチ — **新推定器・追加特徴量枠** [変更]

- multirate：低域に低レートを割り当て、フィルタ・相関を低サンプル数で実行。
- IIR/SOS/FIR：既存の矩形スペクトルマスクと異なる周波数・位相応答をもつフィルタで帯域を分ける。
- ViSQOL型：全体整列の結果を起点にパッチ単位で周辺探索し、時間揺れに対応する。
- Essentia/aubio型：既存STFTから低コストの帯域エネルギー・パワー比を出し、M2やNCCとは**別フィールド**として保存する。
- クロススペクトル`H1(f)=S_xy(f)/S_xx(f)`で伝達ゲインを推定する方法も既存STFT活用の候補。ただし局所ラグ選択・極性・top-half medianを持つM2とは別物。

音声解析機能を豊かにする価値はあるが、Issue #71の**互換性を維持した高速化**と同じPRに混在させない。

---

## 5. 数学的に注意すべき非同値変換

### 5.1 帯域ごとのNCC正規化をスペクトル上で合成できない

現行の

\[
\sum_b \frac{D_b(\tau)}{\sqrt{E_{x,b}E_{y,b}(\tau)}}
\]

と

\[
\frac{\sum_b D_b(\tau)}{\sqrt{(\sum_b E_{x,b})(\sum_b E_{y,b}(\tau))}}
\]

は一般に**一致しない**。周波数ビンのクロススペクトルを先に足してからIFFTするのは、分母を含む現行の帯域別NCC曲線とは異なる。帯域数・重み付けや極性にも影響する。

### 5.2 source / captureは異なるFFTグリッドでフィルタされ得る

`bandpass_many`は入力全区間を各々`next_power_of_two(len)`へゼロ埋めし、半開区間ビンマスクを適用して元長へ切り戻す。sourceとcaptureが異なる長さならFFTグリッドが異なる場合がある。**共通FFTグリッドで`conj(X)Y|M|²`を一度に計算**する方法は、既存の別々の帯域時間波形へのフィルタと同じであると保証できない。

### 5.3 時間窓の切り出しと周波数マスクは一般に可換でない

`band_gain`は**全長信号にフィルタを掛けてから**、band別・window別に切り出して相関を計算する。窓を先に切り出して帯域化すると、境界応答が変わる。全信号のクロススペクトル1回だけでは時間窓ごとの局所ラグ・gainを再現できない。

### 5.4 Pearson / PHAT / パワー比 / M2は別の統計量

- `corr0`：mean subtractionを伴わない非中心化正規化相関。
- Pearson/z-normalized：平均値を除去する。
- GCC-PHAT：周波数の位相情報を重視するスペクトル重み付け。
- 帯域パワー比：ラグ・極性・局所最小二乗条件を持たない。
- `H1`：クロススペクトル/自己スペクトルによる線形伝達推定。
- M2：選択した局所ラグでのLSゲイン、相関・support・極性で選別・集約する。

指標が近い数値を示しても、同値とはならない。

---

## 6. トレードオフ比較（契約・性能・汎用性・保守性）

凡例：**◎** 強い適合、**○** 適合、**△** 条件付き、**×** 通常は不適合。速度欄は**予測であり実測ではない**。

| 案 | 主対象 | 速度の期待 | ピークメモリ | 契約同等性 | 汎用性/保守性 | 規模 | 推奨 |
|---|---|---|---|---|---|---|---|
| A. Valid FFT NCC | locate | ○〜◎（FFT長境界次第） | 削減 | ◎（NCCのみ） | ○、PHAT分離が必要 | 中 | **先行試作** |
| B. Direct/FFT適応dispatch | locate/reference | ○（形状次第） | 縮小可能 | ◎ | ◎（基盤API化） | 中 | **先行試作** |
| C. Streaming Bandpass | reference | CPU不明〜○ | **大幅削減** | ◎ | ○（入力準備の責務分割） | 中 | **先行実装候補** |
| D. 局所Blocked FFT | reference | ○（要実測） | ○ | ◎ | ○（plan/workspace複雑化） | 中〜大 | **実測比較** |
| E. STOMP型更新 | reference | △（狭lag時） | 実装依存 | ◎ | △（専用経路） | 中〜大 | 条件付き |
| F. 全ラグprefix積 | reference | ×の懸念 | 増加懸念 | ◎ | △ | 大 | 非優先 |
| G. スペクトルキャッシュ | locate | ○（ヒット率依存） | **増加** | ◎ | △（寿命・容量管理） | 中〜大 | 計測後 |
| H. Pruned FFT | reference/filter | 不確実 | 不確実 | 条件付き◎ | △ | 大 | 研究枠 |
| I1. multirate / SOS / FIR | locate/reference | ○〜◎の可能性 | 削減可能 | **×（別推定器）** | ○（版管理すれば） | 大 | 別Issue |
| I2. ViSQOL型段階整列 | reference/expensive | ○〜◎の可能性 | 実装依存 | **×（探索を制限するなら）** | ○ | 大 | 別Issue |
| I3. スペクトル集約・H1 | reference特徴追加 | 既存M2の高速化ではない | 小 | **別特徴量** | ◎ | 小〜中 | 別Issue |
| FFTW型batch FFT | reference | △〜○ | ワークスペース依存 | ◎ | △（基盤実装依存） | 中〜大 | 後段 |
| 既存Issueのbandpass plan化 | reference/filter | △〜○ | バッファ再利用 | ◎ | ◎ | 小〜中 | Cと併せ評価 |

---

## 7. 推奨アーキテクチャ

### 7.1 数値契約を変えない基盤の分離

```text
al-math
  NccEngine
    DirectNccKernel        // 同じnon-centered NCC
    ValidFftNccKernel      // valid出力に必要な最小FFT長
    BlockedFftNccKernel    // FFTタイル化
    DispatchPolicy         // shape / 実測 / memory budget

  BandpassPlan
    common forward spectra // sourceとcaptureは独立
    band mask + inverse    // 既存spectral maskを維持
    iterative band output  // 長尺波形を全帯域保持しない

al-locate
  CorrelationPool          // worker所有のcontext、バイト上限を検討
  BandNcc                  // 各bandでNCC正規化→加算
  Existing evidence        // 候補/背景/幅/曖昧性を保存

al-analysis
  band_gain
    StreamingBandIterator
    windows()              // exact output semantics
    select()/summarize()   // 変更しない
    records/tensors        // 変更しない
```

`GccPhatPlan`はValid FFT用のNCCモードから分離する。APIを汎用化しすぎず、`ncc`/`gcc_phat`が明確に別契約であることを型・planキーに残す。

### 7.2 将来の別推定器はデータ上でも別物として扱う

```text
Exact/compatible route
  spectral-mask-v1 -> full-band signals -> band NCC / M2

Optional/new route
  multirate/IIR/FIR/CQT/ERB/DTW/H1 -> independent new features
```

後者を導入するならアルゴリズム名・バージョン・帯域定義・FFTグリッド・サンプリングレート・位相応答・端点処理を出力メタデータに明記し、既存`ref.band_gain_m2`に黙って上書きしない。

---

## 8. 段階的な実施計画

### Phase 0 — ベースライン再現と形状監査（最初に実施）

- 既存実サンプルについて`band_ncc`各呼出の`L/S/K/N`、有効帯域数、source/capture/chの組み合わせを記録する。
- `band_gain`について帯域数・窓数・guard分布・FFT長・`coarse`再サンプル・ピークRSSを確認する。
- `busy`・wall-clock・FFT数・allocated bytes・peak RSS・cache hit/missを分離して記録する。
- 同じ環境、releaseビルド、ウォームアップ・複数反復、入力データを固定して比較する。

### Phase 1 — Valid FFT NCCを独立PRとして試作

- `N>=S`に短縮できる非中心化NCC専用プランを追加する。
- 現行GCC-PHAT planと明示的に分ける。
- `CorrelationPlan`基準の全有効ラグ曲線・candidate evidenceをgolden比較する。
- 短尺・等長・長短不均衡・FFT境界付近を重点測定する。

### Phase 2 — Direct/FFT/Blocked比較とdispatch

- 共通のshapeベンチハーネスで3方式を比較する。
- 小さい`K`への直接SIMD、巨大FFT形状へのBlocked FFTを測る。
- 閾値はCPU・FFTバックエンドや入力形状に依存するため、固定値の採用前に実測表を残す。
- メモリ予算・planサイズ上限を加える。

### Phase 3 — Streaming Bandpassを独立PRとして試作

- 同じ全区間FFT・同じhalf-open帯域マスク・同じ切り戻しで、帯域を逐次生成する。
- `band_gain`固有経路でcoarse計算を省略する。
- source/captureの2chを含めたピークメモリとwall/busyを測る。
- TensorのNaN/0配置、レコード順、帯域guardを比較する。

### Phase 4 — `band_gain`局所NCCのBlocked FFT比較

- 現行FFT16,384と、2×FFT8,192案を比較する。
- テンプレートFFTは窓内タイル間で共有する。
- `W/H/R`の設定を変えた場合も比較し、常に一方が有利とは仮定しない。
- 有利なshapeのみ選択するdispatchを検討する。

### Phase 5 — OSS由来の別推定器・別特徴量の研究

- librosa型multirate、pyfar型fractional-octave、ViSQOL型局所パッチ整列、既存STFTからの帯域エネルギー/H1を**別issue**で調査する。
- 比較条件は、周波数ごとに独立した遅延・極性反転、dropout、強いエフェクト、時間伸縮、低相関素材、複数音源が混在するキャプチャを含める。
- 既存M2に取って代わるのではなく、解釈可能な補助特徴としてまず比較する。

---

## 9. 契約・数値・性能の受入試験

### 9.1 正確性・境界 [必須]

| テスト | 必須観測 |
|---|---|
| 有効ラグ | `0..S-L`の全点、両端・長さ1・等長・極短窓 |
| エネルギー | 無音・片側0・極小正値・gain分母`1e-20`境界 |
| 符号 | 正負極性のlag/corr/gain、反転音声 |
| tie | ほぼ同点・周期音・広いpeak・浮動小数点誤差 |
| locate evidence | 候補順位、`peak_to_background`、`peak_width`、2位比、曖昧性、reject reason |
| reference windows | `start`、`guard`、`search_edge`、`silence`、`low_correlation`、`polarity_unsupported`、`supported` |
| reference output | 全帯域、coherence、top-half極性選択、400Hz anchor、median/MAD、`support_count` |
| dataset | `ref.band_*`、全Tensor（NaN/0含む）、レコード・中間成果物のshapeと値 |
| 実行経路 | scalar / runtime-dispatch、worker並列時、plan/cache再利用後 |
| GCC-PHAT | plan分離後にも**既存値と同じ**演算設定を維持 |

**評価方法**：相関曲線の絶対・相対誤差だけでなく、最大ピーク・tie群・順位・validity・最終alignmentへの波及を検証する。十分離れたピークでは位置完全一致を期待する一方、同点許容差内のほぼ同点は既存曖昧性契約で扱う。閾値は`#42`以降の既存scalar/runtime-dispatch基準に従い、決定前に現実の実数誤差分布を記録する。

### 9.2 性能 [必須]

少なくとも次を**同一入力・同一環境**で反復測定する。

- 小さな`K`、大きな`K`、`L≈S`、`L≪S`、FFT長の段差直前・直後。
- bandpass帯域数：1、5、26。入力長：短尺、中尺、約46秒、より長尺。
- 窓条件：`W=4800,H=2400,R=1920`に加え、`R`が小さいケース。
- source×captureのpair数、ch数、worker数。キャッシュ共有条件。
- CPU busy・wall・peak RSS・計算回数・割当数・キャッシュヒット率・出力サイズ。

**成功基準を二層に分ける**：

1. **互換性**：特徴量と証拠が許容誤差・既存契約内で一致する。
2. **有効性**：対象ワークロードで処理時間やメモリが実際に減る。悪化する形状ではfallbackが働く。

特にStreaming Bandpassはメモリ節約そのものが成果となるため、CPU時間の改善と別に評価する。

---

## 10. 決定が必要な事項と未解決の問い

1. **#71のスコープ**：互換性維持型の性能改修のみとし、音声専用の新推定器は別Issueにするか。→ **推奨：分離**。
2. **NCC/PHATのplan API**：同一`CorrelationPlan`が両者を扱う現状から、演算契約ごとのplanへ分割するか。→ **推奨：型または明示モード分離**。
3. **FFT長の短縮効果**：実ジョブの`L/S`がどの程度`nextpow2(S+L-1)>nextpow2(S)`条件を満たすか。→ **未計測**。
4. **Blocked FFTの局所NCC速度**：FFT長半減×複数ブロックは理論上候補だが、実装全体の高速化率は未知。→ **microbench必須**。
5. **キャッシュ容量**：ワーカー数・帯域・大FFT workspaceに対し何バイト許容するか。→ **RSS計測後決定**。
6. **音声専用別経路**：multirate/IIR/ViSQOL/H1を何の診断目的で、どうメタデータに記録するか。→ **別設計議論が必要**。
7. **#54との関係**：stage内並列化に先立ってメモリを圧縮すべきか。→ **ピークRSS次第**。カーネル最適化と並列化は独立だが、メモリ上限で相互作用する。

---

## 11. 最終提言

**短期**：#71の主実装候補を **(1) Valid FFT NCC、(2) Direct/FFT/Blocked適応dispatch、(3) Streaming Bandpass、(4) `band_gain`の局所Blocked FFT試作** の4系列に整理する。いずれも既存M2やlocate evidenceを維持することを目標にし、個別の前後比較と回帰試験を行う。`band_gain`の単純な「窓NCC全体を1回の全長相関に変える」発想は数学的に不足があり、実装方針に確定しない。

**中期**：メモリ削減と実測による適応方式で負荷が下がった後、FFTスペクトルのpair間キャッシュ、バッチFFT、狭lag専用のSTOMP更新を条件付きで検討する。

**長期**：librosa/pyfar/ViSQOL等の知見から、低レート・多重解像度・階層整列・スペクトル伝達推定を**追加の解析方式・特徴量**として研究する。低相関帯域や独立の帯域遅延を隠さないことを優先し、既存M2の意味を黙って変更しない。

**結論**：現時点で最も根拠が強い新規改善は、STUMPYと同系の**Valid FFT NCC**であり、最も設計上の改善余地が大きいのは`band_gain`の**Streaming Bandpass**である。局所NCCの**Blocked FFT**は次に試すべき有力候補だが、効果はまだ仮説である。音声専用OSS由来のmultirateや段階的整列は価値が高い一方、#71の互換性維持型最適化と混同しない。

---

## 12. 参考文献・ソース実装へのリンク

**ライセンス・方向性の参照**： [librosa README](https://github.com/librosa/librosa) / [Essentia Introduction](https://essentia.upf.edu/introduction.html) / [Essentia license](https://essentia.upf.edu/documentation.html) / [aubio Overview](https://aubio.org/) / [pyfar](https://pyfar.org/) / [ViSQOL README](https://github.com/google/visqol) / [pyroomacoustics summary](https://pyroomacoustics.readthedocs.io/en/stable/) / [STUMPY](https://stumpy.readthedocs.io/en/latest/) / [Chromaprint license](https://github.com/acoustid/chromaprint/blob/master/LICENSE.md) / [madmom README](https://github.com/CPJKU/madmom/blob/main/README.rst) / [Rubber Band README](https://github.com/breakfastquay/rubberband).


### 12.1 AudioLens

1. [Issue #71](https://github.com/scarecrow173/audio-lens/issues/71)
2. [reference band_gain計測資料](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/reference-analysis-analyze-pair-band-gain.md)
3. [band_gain.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-analysis/src/band_gain.rs)
4. [methods.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/methods.rs)
5. [prepared.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/prepared.rs)
6. [stage.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/stage.rs)
7. [pool.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/pool.rs)
8. [correlation.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/correlation.rs)
9. [filter.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/filter.rs)
10. [Issue #40調査](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/issue-40-locate-pair-plan-cache.md) / [Issue #53調査](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/issue-53-local-lag-fft.md)
11. [source-localization仕様](https://github.com/scarecrow173/audio-lens/blob/main/docs/specs/source-localization.md) / [reference-analysis仕様](https://github.com/scarecrow173/audio-lens/blob/main/docs/specs/reference-analysis.md)

### 12.2 相関・FFT・時系列検索

12. [STUMPY — `stumpy/sdp.py`（Valid FFT sliding dot product、pyFFTW cache）](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/sdp.py)
13. [STUMPY — `stumpy/stump.py`（行列プロファイル/対角線更新）](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/stump.py)
14. [SciPy — `choose_conv_method`公式ドキュメント](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.choose_conv_method.html)
15. [SciPy — `oaconvolve`公式ドキュメント](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.oaconvolve.html)
16. [SciPy — `_signaltools.py`ソース](https://github.com/scipy/scipy/blob/main/scipy/signal/_signaltools.py)
17. [OpenCV — `templmatch.cpp`](https://github.com/opencv/opencv/blob/4.x/modules/imgproc/src/templmatch.cpp)
18. [OpenCV — Template Matching公式ドキュメント](https://docs.opencv.org/4.x/de/da9/tutorial_template_matching.html)
19. [FFTW — Advanced Real-data DFTs / Plan Many](https://www.fftw.org/fftw3_doc/Advanced-Real_002ddata-DFTs.html)

### 12.3 音声・音響・音楽解析

20. [librosa — `iirt`公式ドキュメント](https://librosa.org/doc/main/generated/librosa.iirt.html)
21. [librosa — `core/spectrum.py`（iirt、STFTバッチ処理）](https://github.com/librosa/librosa/blob/main/librosa/core/spectrum.py)
22. [librosa — `filters.py`（multirate filterbank）](https://github.com/librosa/librosa/blob/main/librosa/filters.py)
23. [librosa — `sequence.py`（DTW）](https://github.com/librosa/librosa/blob/main/librosa/sequence.py)
24. [Essentia — アルゴリズム一覧](https://essentia.upf.edu/algorithms_reference.html)
25. [Essentia — `ERBBands`公式ドキュメント](https://essentia.upf.edu/reference/streaming_ERBBands.html)
26. [Essentia — `BarkBands`公式ドキュメント](https://essentia.upf.edu/reference/std_BarkBands.html)
27. [Essentia — `crosscorrelation.cpp`](https://github.com/MTG/essentia/blob/master/src/algorithms/standard/crosscorrelation.cpp)
28. [aubio — `filterbank.c`](https://github.com/aubio/aubio/blob/master/src/spectral/filterbank.c)
29. [pyfar — fractional-octaveフィルタ公式ドキュメント](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html)
30. [pyfar — `fractional_octaves.py`](https://github.com/pyfar/pyfar/blob/main/pyfar/dsp/filter/fractional_octaves.py)
31. [Google ViSQOL — リポジトリ](https://github.com/google/visqol)
32. [ViSQOL — `alignment.cc`](https://github.com/google/visqol/blob/master/src/alignment.cc)
33. [ViSQOL — `comparison_patches_selector.cc`](https://github.com/google/visqol/blob/master/src/comparison_patches_selector.cc)
34. [ViSQOL — `gammatone_spectrogram_builder.cc`](https://github.com/google/visqol/blob/master/src/gammatone_spectrogram_builder.cc)
35. [pyroomacoustics — `sync.py`（GCC/PHAT/TDOA）](https://github.com/LCAV/pyroomacoustics/blob/master/pyroomacoustics/sync.py)
36. [Chromaprint — `fingerprint_matcher.cpp`](https://github.com/acoustid/chromaprint/blob/master/src/fingerprint_matcher.cpp)
37. [audfprint — リポジトリ](https://github.com/dpwe/audfprint)
38. [SciPy — `csd`公式ドキュメント](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.csd.html)
39. [madmom — README/ライセンス](https://github.com/CPJKU/madmom/blob/main/README.rst)
40. [Rubber Band — 公式mirrorとR2/R3・ライセンス](https://github.com/breakfastquay/rubberband)
41. [Chromaprint — LICENSE.md（同梱コードの注意）](https://github.com/acoustid/chromaprint/blob/master/LICENSE.md)
42. [Essentia — 設計思想・Standard/Streaming](https://essentia.upf.edu/introduction.html)
43. [librosa — multirate filterbank API](https://librosa.org/doc/main/generated/librosa.filters.semitone_filterbank.html)
44. [pyfar — fractional-octave SOSと再構成FIRの仕様差](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html)

---

> **注記**：この報告書は、既存Issue・計測資料・ソースコード・OSS公開資料から導いた技術検討である。OSS由来の演算方式をAudioLensへ組み込んだ結果や、新しい性能改善の実測値は含まない。ソースのバージョンと測定環境を固定した再現実験を、実装採否の条件とする。
