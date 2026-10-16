---
title: "AudioLens Issue #71 — OSS詳細調査：プロジェクトの目的・設計思想・実装・アルゴリズム適用性"
date: "2026-10-17"
repository: "https://github.com/scarecrow173/audio-lens"
issue: "https://github.com/scarecrow173/audio-lens/issues/71"
status: "技術調査（外部実装をAudioLensへ導入した実測ではない）"
---

# AudioLens Issue #71 — OSS詳細調査報告書

**調査日**：2026-10-17（日本時間）  
**対象**：`locate::band_ncc` の帯域別全域 NCC、および `reference::band_gain` の1/3オクターブ帯域別局所ラグ補償付き最小二乗ゲイン（M2）。  
**調査手段**：各OSSの公式説明・リポジトリ・具体的ソースファイル・API文書の照合。外部 OSS のベンチ結果を AudioLens の性能改善として転用しない。  
**姉妹文書**：`audiolens_issue71_algorithm_research_report_detailed.md` は本調査を統合した全体報告書。

## A. 調査上の問題設定と結論の読み方

この比較で大切なのは、同じ「帯域」「相関」「位置合わせ」という名前でも、OSSごとの**解く問題・統計量・出力の意味**が異なること。

| 軸 | AudioLens `band_ncc` | AudioLens `band_gain` | 他OSSでよく扱われる別問題 |
|---|---|---|---|
| 入力 | ソース波形とキャプチャ波形 | アライン済みのソース/キャプチャ | 音楽特徴・単一音声・マイクアレイ等 |
| 帯域 | 数帯域の全長 spectral mask | 26帯域程度の1/3オクターブ spectral mask | Mel/Bark/ERB/CQT、IIR/SOS、FIR |
| 相関 | 各帯域の**非中心化NCC**を先に正規化し、全有効ラグについて加算 | 各帯域・各短時間窓の非中心化NCCと局所ラグ選択 | Pearson、PHAT、パッチ類似度、指紋ハッシュ |
| 推定量 | 全域のピーク・背景・競合・幅・曖昧性 | 最適ラグでの `dot(x,y)/sum_sq(x)` と頑健な集計 | スペクトルパワー比、H1、MOS、音楽構造 |
| 重要な不変量 | 全域証拠・符号・方法独立性 | 帯域別遅延、極性、support、guard、低相関の扱い | 音質/分類/識別が主目的のOSSでは保証されない |

**判定表現**：

- **[実装確認]**：公開ソースの具体的な演算・構造に根拠がある。
- **[数理的に適用可能]**：前提を守れば既存推定量を計算できる。浮動小数点bit一致までは主張しない。
- **[別推定器]**：既存の `band_ncc` / M2 と異なる統計量・フィルタ・証拠になる。
- **[研究課題]**：高速化・有効性はまだ実測によって証明されていない。

### A.1 現行の確認済み性能状況

[Issue #71](https://github.com/scarecrow173/audio-lens/issues/71) と[計装資料](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/reference-analysis-analyze-pair-band-gain.md)によると、約46秒・48kHzのサンプルで `locate band_ncc` は計174秒 busy/16呼出、`reference band_gain` は約23秒/2ch pair。`band_gain` の主因は1chあたり全長26帯域のフィルタ約4.35秒、26帯域×約915窓のNCC約6.92秒。前処理・窓演算・入出力は区別して評価する必要がある。

**注意**：この数字はAudioLensの既存計測。以降のOSSの説明は、この計測を再現したものではない。

## B. OSSの位置付けと成熟・依存関係

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

## C. 音声・音楽・音響を中心とするOSSの詳細

### C.1 librosa — MIR向けの柔軟な分析・実験プラットフォーム

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

### C.2 Essentia — 大規模な音楽解析を構成するC++アルゴリズムエンジン

**プロジェクト全体の方向性。** Music Technology Group（UPF）発の音声・音楽解析ライブラリ。Spectral/Temporal/Tonal/Rhythm/Music descriptors、入出力、集計、学習モデル統合などの**大きなアルゴリズムカタログ**を備える。公式の設計方針は、正確性、使いやすさ、コード保守性、性能、アルゴリズム範囲の順で重視。研究の対話的試作と大規模バッチ抽出の両方を想定し、C++本体とPython/JSなどのインターフェースを提供する。[設計思想](https://essentia.upf.edu/introduction.html)・[全体](https://essentia.upf.edu/)

**特に参考になるアーキテクチャ。**

- **Standard mode**：呼出側がデータ確保と処理タイミングを制御する命令的API。データコピーを抑え、特定カーネルの性能検証がしやすい。
- **Streaming mode**：Source/Sinkを接続して有向グラフ化し、スケジューラに段階間のデータ送受信を任せる。同じ演算実装を可能な限り共有しながらパイプラインを組む。AudioLensのCheap/Reference/Expensiveという段階、特徴量共通中間表現、バッファ寿命の分離に非常に近い**設計参照**。[Streaming architecture](https://essentia.upf.edu/streaming_architecture.html)
- `BarkBands`、`ERBBands`、`EnergyBand`などでは、入力スペクトルから帯域強度/エネルギーをまとめて計算する。[アルゴリズム一覧](https://essentia.upf.edu/algorithms_overview.html)・[ERBBands](https://essentia.upf.edu/reference/streaming_ERBBands.html)
- [`CrossCorrelation`](https://github.com/MTG/essentia/blob/master/src/algorithms/standard/crosscorrelation.cpp)は対象ラグの直接積和で、巨大な全域相関や `band_gain` の約47,600回のFFTを削減する完成済み手法ではない。

**AudioLensへの移植可能性。** FFTを共有する複数の「スペクトル集約だけで良い」特徴量を1パスへ合成し、M2等の波形・位相を要する特徴とは計算パスを分ける。**入力データ取得/共通スペクトル/窓フレーム/FFT/特徴・recordsへの分配をDAGとして明示する**点で、EssentiaのStreaming設計は重要。ただし `band_gain` の既存 `Window.reason` や `support` は別途実装する。

**非互換・限界。** `ERBBands` や `BarkBands` はエネルギー表現を返し、帯域ごとの全域NCCや局所LSゲインには直結しない。**AGPL-3.0**はコード移植・直接依存時の重要条件。設計から得られたパターンを独自に実装する方針と、ソース流用は明確に区別する。[ライセンス](https://essentia.upf.edu/documentation.html)

**評価**：**パイプライン設計◎**／スペクトル特徴最適化◎／NCC・M2カーネルの直接改善△。

### C.3 aubio — 因果的・低遅延の音楽イベント検出

**プロジェクト全体の方向性。** Cで書かれた低遅延の音楽イベント検出ライブラリ。onset、pitch、beat/tempo、MFCC、相対的に軽量なFFTや音声フィルタを備える。開発者は因果的（未来サンプルに依存しない）な演算に重きを置き、ライブ処理/楽器/エフェクト用途を重視する。AudioLensの**オフラインかつ全証拠保存**とは優先順位が異なる。[プロジェクト紹介](https://aubio.org/)・[マニュアル](https://aubio.org/manual/latest/index.html)

**具体コード。** [`src/spectral/filterbank.c`](https://github.com/aubio/aubio/blob/master/src/spectral/filterbank.c) の `aubio_filterbank_do()` は、入力FFTの振幅ベクトル（必要なら冪）をフィルタ係数行列 `fmat_vecmul` で複数帯域へ射影する。帯域数ぶん全長IFFTをしない理由は、**出力がエネルギー/スペクトル特徴で、時間波形ではないから**。`src/spectral/fft.c`、`src/pitch/`や`src/onset/`は軽量フレーム処理の参考。

**AudioLensへの移植可能性。** 既に存在するSTFTからMel/Bark系の複数帯域特徴をまとめて計算し、オンセット候補をCheap featuresへ追加して異常検知の窓を提案すること。リアルタイムデータ構造は将来のストリーミング対応を検討するときに参考になる。

**非互換・限界。** フィルタバンク係数が与える帯域パワーを、各帯域のNCCやラグ補償LSゲインに置き換えてはいけない。イベント検出は既存数値を再現する性能最適化には直接寄与しない。ライセンスはGPL系。

**評価**：Cheap features拡充○／低遅延DSP設計○／#71の支配カーネル置換△。

### C.4 pyfar — 音響測定でのフィルタの意味・再構成性・追跡可能性

**プロジェクト全体の方向性。** 音響測定・研究を重視するPythonエコシステム。Signal/TimeData/FrequencyData、Filters、座標・指向性表現、評価・可視化・音響機器との接続を統一的に扱う。文書化、テスト、データ構造の意味を重視する点は、AudioLensの構造化結果保存と相性が良い。[公式サイト](https://pyfar.org/)・[API体系](https://pyfar.readthedocs.io/en/stable/api_reference.html)

**2種類の重要なフィルタバンク。**

1. [`fractional_octave_bands`](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html)：IEC 61260-1:2014 に対応する条件で設計された**Butterworth SOSの-3dBカットオフ**を持つエネルギー保存指向のフィルタ群。有限次数やNyquist近傍には例外がある。
2. [`reconstructing_fractional_octave_bands`](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html)：**-6dB、線形位相FIR、`n_samples/2`の遅延**という別設計。加算再構成（振幅保存）を意識する。両者は「fractional octave」という名前でも測定結果が一致しない。[実装](https://github.com/pyfar/pyfar/blob/main/pyfar/dsp/filter/fractional_octaves.py)

**AudioLensへの移植可能性。** `ref.band_gain_m2` と `ref.band_*` に対して、**フィルタ方式ID、振幅応答、群遅延、サンプルレート、端点処理、guardの設計を別々に記録する**べきことが明確になる。既存の矩形spectral maskを残し、音響規格に近い帯域測定は**別feature family**として実装・比較するのが望ましい。

**非互換・限界。** SOSの位相/過渡・FIRの遅延は現在の矩形マスクと異なる。帯域ゲインや相関のvalidityが変わり得る。「フィルタバンクの形式が一般的」という理由だけでM2の出力上書きはできない。

**評価**：音響計測の仕様設計◎／1/3oct別経路◎／互換性維持の直接高速化△。

### C.5 Google ViSQOL — リファレンス音声の知覚的劣化評価

**プロジェクト全体の方向性。** ViSQOL (Virtual Speech Quality Objective Listener) は、参照音声と劣化音声の**時間周波数構造**の類似性から、知覚音質（MOS-LQO）の予測を出すフルリファレンス評価器。音声/音楽等のモードでモデル、推奨サンプルレート、内部処理が異なる。公式READMEによればaudio modeは48kHzを期待し、多チャンネル入力は比較時にmonoへdownmixする。speech modeは16kHzを期待する。**出力の目的は知覚品質予測で、発生した音声バグのメカニズムを分解して記録するAudioLensとは違う。** 公式READMEはコーデックやVoIP劣化評価を主要用途とし、他用途では性能に限界があると明記する。[README](https://github.com/google/visqol)

**ソース確認されたパイプライン。**

- [`src/alignment.cc`](https://github.com/google/visqol/blob/master/src/alignment.cc)：全体の音声包絡（upper envelope）相互相関から粗い全体整列。
- [`src/gammatone_spectrogram_builder.cc`](https://github.com/google/visqol/blob/master/src/gammatone_spectrogram_builder.cc) と [`src/gammatone_filterbank.cc`](https://github.com/google/visqol/blob/master/src/gammatone_filterbank.cc)：Gammatone/ERBの周波数方向へ時間フレームのフィルタ信号を集約し、帯域パワーで比較可能な表現へ。
- [`src/comparison_patches_selector.cc`](https://github.com/google/visqol/blob/master/src/comparison_patches_selector.cc)：参照パッチごとに探索窓内の候補パッチを比較し、累積類似度とbacktraceで**時間順序・途中スキップ**を扱う（DTWに近いDP）。
- 音質スコア学習/マッピングとパッチ表現の生成は上記とは別ステップ。

**AudioLensへの移植可能性。** 素材位置の粗整列→時間・帯域の局所比較→異常区間で精密解析という段階化。`band_gain` が毎窓±40ms全探索を繰り返すコストを減らす**別経路**として、共通ラグ候補/前窓予測の周りから段階的に探索する案はある。

**重大な制約。** AudioLensは**帯域ごとに独立した遅延**も異常として保存する。ViSQOL風に全帯域共通の整列結果へ強制固定すると、周波数依存遅延、位相異常、途切れの証拠が消える。またViSQOLはスコアを出すために多くの特徴を集約し、低相関帯域のM2正当性を保証しない。公式audio modeのmono downmixは、AudioLensが重視するsource→L/R独立の異常解析の代用にならない。現在のgammatone実装はフレームごとにフィルタ状態を初期化する部分があり、そのカーネルが特に高速とは言えない。

**評価**：段階化・局所整列の設計◎／互換性維持のM2高速化△／**別の知覚品質参考指標**○。

### C.6 pyroomacoustics — 室内音響シミュレーションとマイクアレイDSPの研究基盤

**プロジェクト全体の方向性。** 複数音源・マイク・反射を伴う室内音響のシミュレーションと、beamforming、DOA（到来方向）推定、適応フィルタ、分離、ノイズ除去等を**比較可能な参照実装として提供する**ことが主目的。C++の伝搬計算とPythonの実験用APIで構成する。[公式Summary](https://pyroomacoustics.readthedocs.io/en/stable/)

**関連ソース。** [`pyroomacoustics/sync.py`](https://github.com/LCAV/pyroomacoustics/blob/master/pyroomacoustics/sync.py) の `tdoa()` と `correlate()` はFFTでcross-spectrumを作り、オプションのPHAT正規化を施して lag を求める。STFTエンジンやadaptive filteringは独立モジュールに切り分ける。

**AudioLensへの移植可能性。** ゲームのcaptureが複数ch/残響/クロストークを含む場合、遅延推定とチャネル混合・gain・残差推定を別の推定器として並立させる指針になる。現行のGCC-PHATやmix解析との整合チェックに使える。

**非互換・限界。** PHATはスペクトル振幅で重みを正規化する別の統計量。`band_ncc`の各帯域非中心化NCCや、M2のゲイン・supportを計算するわけではない。室内伝搬を想定した典型例は、ゲームのデジタルキャプチャ障害とそのまま一致しない。

**評価**：チャネル/遅延設計○／基準実装とのA/B評価○／#71直接の高速化△。

## D. 高速相関・時系列検索・FFTに特化したOSSの詳細

### D.1 STUMPY — Matrix Profile とSliding Dot Productの高速アルゴリズム

**プロジェクト全体の方向性。** あらゆる時系列に対する近傍部分系列・motif・discord・異常発見・セグメンテーションを高速に求めるPython/Numbaのライブラリ。`mass`（query-to-seriesのdistance profile）や `stump`（Matrix Profile）を中心に、高性能計算をアルゴリズム化している。音響専用ではないが、AudioLensの**スライディング相関を高速に計算したい**という問いには、汎用音声特徴OSS以上に直接的な先例がある。[公式概要](https://stumpy.readthedocs.io/en/latest/)・[API](https://stumpy.readthedocs.io/en/stable/api.html)

**具体実装。**

- [`stumpy/sdp.py`](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/sdp.py) は `n=len(T)`、`m=len(Q)` に対して `next_fast_len(n)` でFFT長を選び、反転したqueryとTの**循環畳み込みのvalid部分 `[m-1:n]`**を切り出す。全線形畳み込みのための`n+m-1`を使わない。
- 同ファイルには、直接演算（Numba）、SciPy直接/FFT自動選択、Overlap-Add、pocketfft、pyFFTWキャッシュ実装があり、**同一APIで実行方法を比較**する作りになっている。
- [`stumpy/stump.py`](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/stump.py) は窓相関を対角線上で更新する STOMP 系アルゴリズムを使用。隣接窓の相関の積和を引き継ぐ。

**適用の核心（数理的に適用可能）。** `band_ncc` の *分子* はNCC有効ラグの範囲で FFT長`nextpow2(S)`の循環相関として計算できる。既存の`CorrelationPlan::new`の`nextpow2(S+L-1)`より短くなるケースでは、FFT作業領域と演算量が減る。*分母*は現行どおりテンプレートエネルギーとスライディングキャプチャ窓エネルギーで計算する。

**境界条件。** STUMPYの通常のMatrix Profileは**z-normalized（平均を引くPearson系）**で、AudioLensの `corr0` は**平均を引かない非中心化NCC**。分子のSliding Dot Productだけを移植すべき。`gcc_phat`のFFT長を同様に短縮するとPHATの周波数重みが変わるため、GCC-PHATには流用しない。

**STOMPがM2に向かない理由。** M2のデフォルトはhop=窓長の半分・最大ラグ±1920で、各時刻/ラグの直積更新は概算`2.205e6×3841≈8.47e9`箇所/帯域/chとなる。ラグが非常に狭く、hopが極端に小さい設定でのみ比較対象。演算回数とメモリ帯域の実測が必要。

**評価**：**band_nccのValid FFT P0**／`band_gain`のSTOMP方式P3。

### D.2 SciPy — 演算形状から数値カーネルを選ぶ基盤

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

### D.3 OpenCV — 高速テンプレート照合でのブロックDFT設計

**方向性。** 高性能な画像処理・特徴抽出/認識のC++ライブラリ。CPU/最適化バックエンド/GPU等を用意し、大量の画素に対する相関と正規化を効率的に実行する。音声ではないが、**有効位置での正規化テンプレート照合**は1次元化するとAudioLensのNCCと非常に近い。[テンプレート照合の数式](https://docs.opencv.org/4.x/de/da9/tutorial_template_matching.html)

**確認できる実装。** [`modules/imgproc/src/templmatch.cpp`](https://github.com/opencv/opencv/blob/4.x/modules/imgproc/src/templmatch.cpp) は、相関結果を複数タイルに区切り、テンプレートのDFTをキャッシュ的に利用しつつ、入力側の部分DFT、cross-spectrum、inverse DFTを行う。正規化には積分和・二乗和や別の演算経路を用いる。**`TM_CCORR_NORMED`が非中心化NCCの式に該当**し、`TM_CCOEFF_NORMED`は平均を引くため異なる。

**AudioLensへの移植可能性。** `band_ncc` の全域を分割して相関曲線を生成し、全ラグの候補順位・背景・端点を変更しない。`band_gain`の短窓ではFFT16,384に対し、8192×2ブロック案を実測比較できる。試算では窓内の3 forward +2 inverse（8192）対2 forward +1 inverse（16384）を`N logN`モデルで比べると約0.774。ただし実時間23%削減という意味ではない。

**非互換・注意。** OpenCVの画像境界処理、複数チャネル正規化、float32/float64、パディングはAudioLensの既存規則とは違う。**タイル境界のラグを二重計算/欠落しない**こと、NCCの分母が窓位置と一致すること、tie/峰形状を完全に保持することが重要。

**評価**：**Blocked FFT P1**。長尺ワークスペースの減少効果も評価対象。

### D.4 FFTW — FFTカーネルの計画・バッチと実行効率

**方向性。** FFTを高効率に処理するためのCライブラリ。1D/ND、実数/複素、batched（`plan_many` / guruインターフェース）、stride、in-placeなど、多様な形状を処理し、planによる実行効率を追求する。メインの価値は**FFT演算そのものの高性能化**であり、NCCや局所M2という高水準推定器を実装することではない。[FFTW公式](https://www.fftw.org/)・[高度なreal transforms](https://www.fftw.org/fftw3_doc/Advanced-Real_002ddata-DFTs.html)

**関連する設計知見。** 多数の窓の同一FFT長・stride付きデータをひとまとめに処理する計画。`band_gain`では約47,600の小FFTが反復されるため、バッチ処理とメモリ配置（SoA/AoS、行優先・列優先）を組み合わせる実験は理にかなう。

**制約。** FFTの回数・フィルタbankの本数・NCCの数学的コストを自動的に減らすわけではない。FFTWのplan生成コスト、メモリ、thread数、既存Rust `al-math` FFT実装とのベンチが先。GPL/商用オプション等の**配布条件を確認すること**。FFTWのGitHubリポジトリは自ら通常のソース配布用cloneと誤解しないよう注意を記しているため、公式の配布・文書を優先する。

**評価**：Plan reuseは実装済み／**batch・workspaceの検証 P2**。

## E. 音声指紋・音楽イベント・時間伸縮のOSS

### E.1 Chromaprint — 「ほぼ同一録音」を短い特徴で識別する

**方向性。** AcoustIDのための音声指紋ライブラリ。コンパクトな指紋を生成し、録音の識別・重複判定・長時間監視をする。**一般目的の強頑健指紋ではなく、検索速度のために精度/頑健性をトレードオフする**と明記している。[公式README](https://github.com/acoustid/chromaprint)・[AcoustID説明](https://acoustid.org/chromaprint)

**具体コード。** [`src/fingerprint_matcher.cpp`](https://github.com/acoustid/chromaprint/blob/master/src/fingerprint_matcher.cpp) は、指紋整数列中の一致部分から時間オフセット候補をヒストグラム化し、候補の一致度を確認する。全PCMの各ラグ相関を計算しない。

**AudioLensへの用途。** BGM等が多量にあるときに、候補素材ID・候補オフセットを先に出すための前段。実際の定位の正しさは、既存のfull/band NCCや他の独立証拠で評価する。ゲームSEのような短い素材や大きく加工された録音には、仕様上の優位を保証しない。AcoustID公式配布ページには**2026-10-15公開のChromaprint v1.6.2**が記載されており、少なくとも調査日時点で配布更新の一次根拠がある。[公式ダウンロード](https://acoustid.org/chromaprint)

**重要なライセンス注意。** 自前コードはMITだが、[公式LICENSE.md](https://github.com/acoustid/chromaprint/blob/master/LICENSE.md)は内包するFFmpeg由来部分のため、**全体をLGPL-2.1と考えるべき**と説明している。FFTバックエンドのライセンスも影響し得る。

**評価**：多素材候補の前処理○／既存のbandNCC evidenceの代替×。

### E.2 audfprint — ランドマークに基づく部分音声の検索

**方向性。** Dan Ellisの音声ランドマーク照合方式を基にしたPythonの音声指紋実装。周波数時間平面のピークを用いて頑健なハッシュ列を作り、データベースから**雑音のある短いquery**を検索することに狙いがある。[README](https://github.com/dpwe/audfprint)

**ソース構造。** [`audfprint_analyze.py`](https://github.com/dpwe/audfprint/blob/master/audfprint_analyze.py) が信号の時間周波数ランドマーク、[`audfprint_match.py`](https://github.com/dpwe/audfprint/blob/master/audfprint_match.py) がハッシュ/候補照合、[`hash_table.py`](https://github.com/dpwe/audfprint/blob/master/hash_table.py) がデータベース表現、[`audfprint.py`](https://github.com/dpwe/audfprint/blob/master/audfprint.py) がCLIワークフローを扱う。

**AudioLensへの用途。** BGM/多数候補/ミックス音声中の短いソース候補提案や、動的に変わる capture からのsource候補検索。ただし周波数ピークが弱い音や極短のSEでは偽陰性が増え得る。ピークハッシュは真のgain、反転極性、全ラグ背景、残差を提供しない。

**保守状況の留保。** 公開GitHub `pushed_at` は2019年で、研究OSSの参照実装としては有用だが、現行ランタイムとの互換・再現性はローカルで検証が必要。

**評価**：多素材候補の前処理○／既存のbandNCC evidenceの代替×。

### E.3 madmom — 時間イベントの高機能MIRパイプライン

**方向性。** beat・onset・downbeat・テンポ、音符等の音楽イベント認識に特化。Processorの連結とNumPy配列に基づく研究アルゴリズム実装、Cythonでの高速化、学習済みモデルを組み合わせる。[README](https://github.com/CPJKU/madmom)・[導入](https://madmom.readthedocs.io/en/latest/introduction.html)

**AudioLensへの用途。** 途切れたイベント、オンセットの時間シフト、beat同期不良などの検出について、Cheap/Expensive featuresの参考にできる。**現行の帯域別正規化相関を高速化するカーネルではない**。

**重大なライセンス区分。** ソースコードは原則BSDで、同梱のモデル・データは**CC BY-NC-SA 4.0**として別管理される。[ライセンス説明](https://github.com/CPJKU/madmom/blob/main/README.rst)　学習済みモデルを組み込む場合は用途を慎重に確認する。

**評価**：音楽イベント解析の追加候補○／#71の直接高速化×。

### E.4 Rubber Band — 時間伸縮/ピッチ変化を「生成」する実装

**方向性。** テンポとピッチを独立に変更するDSPライブラリ。R2（Faster）/R3（Finer）のエンジンを使い分け、音質・CPUコストのトレードオフを利用者へ露出する。これは**変換器であり、入力2本の対応を検出するアラインメント推定器ではない**。[公式ミラーREADME](https://github.com/breakfastquay/rubberband)

**AudioLensへの用途。** デバッグfixtureとして、音声の非線形時間伸縮、長さの変化、ピッチシフト、トランジェント付近の加工を人工的に生成し、`locate`/tracking/M2がそれをどう観測するか試験する。アルゴリズムを直接組み込むより、独立テスト生成ツールとして使うと利点が大きい。

**注意。** 変換器が作るstretch/音質変化はAudioLensの推定器の正しさそのものを保証しない。GPL-2.0-or-laterまたは商用ライセンスで配布条件を検討する。[ライセンス説明](https://github.com/breakfastquay/rubberband)

**評価**：golden・ストレス試験○／#71カーネルの直接高速化×。

## F. OSSを横断して見えるアルゴリズム設計原則

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

## G. 課題別の比較と優先順位

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

## H. 実装採否の実証計画

**P0 — 同一推定量の構造改善**

1. Valid FFT：`N=nextpow2(S+L-1)` 対 `N=nextpow2(S)` のFFT長、時間、RAMを形状別比較。GCC-PHATは旧FFT長を固定。
2. Direct SIMD / FFT：`(L,K)` 格子を測り、1ラグ・数ラグ・長尺・短テンプレート・zero/periodic/negative/NaN等を検査。
3. `band_gain`：26帯域の一括 materialization と逐次 source/capture帯域生成・消費を比較し、レコード/tensorの全データ・peak RSSを確認。

**P1 — 選択的高速化**

4. `band_gain` 1窓`L=4800,S=8640,K=3841` の旧 FFT16384 対 FFT8192×2タイルを計測。エネルギー、gain、lag選択、window.reason、tieをA/B比較。
5. 長尺 `band_ncc` のOpenCV式ブロック計算で、全域peak/background/ambiguous/evidenceが一致することを検証。

**P2 — 別推定器・追加特徴量**

6. librosa iirt/pyfar SOSの低域節約効果は、原M2と比較する**別feature_id**で検証。周波数レスポンス、群遅延、窓境界、極性反転、低相関帯域・0エネルギーを条件として再現。
7. ViSQOL/DTW/音声指紋の候補生成に対して、ゲームのSE/BGM/mix/dropout/pitch/速度揺れの偽陽性・偽陰性と速度を測定。full NCCは証拠基準として保持。
8. madmom onsetやRubber Bandの生成波形で、現行のCheap/Reference/Expensiveとtraceabilityを評価。

**測定軸**：CPU busy、wall-clock、peak RSS、FFTs回数とサイズ、allocation bytes、SIMD dispatch、channel-pairs、cache hit、入力帯域と長さ、候補数、全テンサ同等性。既存実サンプルの同じreleaseビルドとCPU環境を用い、単一・複数ワーカを分けて計測する。

## I. 直接参照するべき一次資料（ファイル/公式文書）

### 現行AudioLens

- [Issue #71](https://github.com/scarecrow173/audio-lens/issues/71)
- [計測内訳](https://github.com/scarecrow173/audio-lens/blob/main/docs/discovery/reference-analysis-analyze-pair-band-gain.md)
- [band_gain.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-analysis/src/band_gain.rs) / [filter.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/filter.rs) / [correlation.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-math/src/correlation.rs)
- [locate methods.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/methods.rs) / [prepared.rs](https://github.com/scarecrow173/audio-lens/blob/main/crates/al-locate/src/prepared.rs)

### 音声・音楽・音響のOSS

- [librosaプロジェクト](https://github.com/librosa/librosa) / [iirt](https://librosa.org/doc/main/generated/librosa.iirt.html) / [コード](https://github.com/librosa/librosa/blob/main/librosa/core/spectrum.py) / [multirate filters](https://github.com/librosa/librosa/blob/main/librosa/filters.py) / [DTW](https://librosa.org/doc/latest/generated/librosa.sequence.dtw.html)
- [Essentiaの設計紹介](https://essentia.upf.edu/introduction.html) / [Streaming Architecture](https://essentia.upf.edu/streaming_architecture.html) / [全アルゴリズム](https://essentia.upf.edu/algorithms_overview.html) / [CrossCorrelation](https://github.com/MTG/essentia/blob/master/src/algorithms/standard/crosscorrelation.cpp)
- [aubio紹介](https://aubio.org/) / [マニュアル](https://aubio.org/manual/latest/index.html) / [filterbank.c](https://github.com/aubio/aubio/blob/master/src/spectral/filterbank.c)
- [pyfar全体](https://pyfar.org/) / [filter APIs](https://pyfar.readthedocs.io/en/stable/modules/pyfar.dsp.filter.html) / [fractional_octaves.py](https://github.com/pyfar/pyfar/blob/main/pyfar/dsp/filter/fractional_octaves.py)
- [ViSQOL README](https://github.com/google/visqol) / [alignment.cc](https://github.com/google/visqol/blob/master/src/alignment.cc) / [comparison patches](https://github.com/google/visqol/blob/master/src/comparison_patches_selector.cc) / [gammatone](https://github.com/google/visqol/blob/master/src/gammatone_spectrogram_builder.cc)
- [pyroomacoustics overview](https://pyroomacoustics.readthedocs.io/en/stable/) / [sync.py](https://github.com/LCAV/pyroomacoustics/blob/master/pyroomacoustics/sync.py)

### 相関・FFT・候補検索・試験

- [STUMPY API](https://stumpy.readthedocs.io/en/stable/api.html) / [sliding dot product](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/sdp.py) / [STOMP](https://github.com/stumpy-dev/stumpy/blob/main/stumpy/stump.py)
- [SciPy convolve](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.convolve.html) / [choose_conv_method](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.choose_conv_method.html) / [oaconvolve](https://docs.scipy.org/doc/scipy/reference/generated/scipy.signal.oaconvolve.html)
- [OpenCV matchTemplate](https://docs.opencv.org/4.x/de/da9/tutorial_template_matching.html) / [templmatch.cpp](https://github.com/opencv/opencv/blob/4.x/modules/imgproc/src/templmatch.cpp)
- [FFTW](https://www.fftw.org/) / [plan many](https://www.fftw.org/fftw3_doc/Advanced-Real_002ddata-DFTs.html)
- [Chromaprint](https://github.com/acoustid/chromaprint) / [matcher.cpp](https://github.com/acoustid/chromaprint/blob/master/src/fingerprint_matcher.cpp) / [ライセンス](https://github.com/acoustid/chromaprint/blob/master/LICENSE.md)
- [audfprint](https://github.com/dpwe/audfprint) / [audfprint_analyze.py](https://github.com/dpwe/audfprint/blob/master/audfprint_analyze.py)
- [madmom](https://github.com/CPJKU/madmom) / [READMEとモデルライセンス](https://github.com/CPJKU/madmom/blob/main/README.rst)
- [Rubber Band公式ミラー](https://github.com/breakfastquay/rubberband)

---

## J. 総合判断

1. **#71を既存数値契約のまま高速化する第一候補は、音声専用OSSの高級filterbankではなく、STUMPYのvalid-only sliding dot product、SciPyのadaptive dispatch、OpenCVのblocked DFT。** 実装・数学の対応が明確。
2. **音声専用OSSの最大の収穫はアーキテクチャ。** librosaのmultirate、EssentiaのStandard/Streaming分離、pyfarの明確なフィルタ特性、ViSQOLの段階的アライン・帯域/時間パッチの組み合わせは、AudioLensの長期的な機能と性能の両方に関係する。
3. **スペクトル帯域パワーだけでは現行M2を再現できない。** 整列・位相・極性・帯域ごとラグ・supportの情報が欠ける。新しい特徴量と既存特徴量を別のID/アルゴリズムで出力する。
4. **音声指紋は粗候補の追加証拠であって、全域NCCの代替ではない。** 速度優位だけを理由に、競合ピーク・背景・曖昧性・不一致の保存契約を削除しない。
5. **OSSのライセンス条件は導入方法ごとに評価する。** 読んで原理を学ぶことと、依存・移植・バイナリ/モデル配布は異なる。

以上は**新規ベンチマーク未実施の調査結論**。導入の可否と速度改善量は、同一CPU・release build・実サンプルと異常系fixtureで決定する。
