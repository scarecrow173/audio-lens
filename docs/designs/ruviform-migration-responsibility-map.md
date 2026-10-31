---
id: "DESIGN-034P3nlejLy074yTmYSAcJ"
type: "design"
status: "approved"
title: "Ruviform 移行の処理単位責務マップ"
created: "2026-10-30"
updated: "2026-10-31"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/95"
    - "https://github.com/scarecrow173/audio-lens/issues/97"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../plans/ruviform-foundation-migration.md"
  depends-on:
    - "../adr/ruviform-foundation-migration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ruviform-foundation-migration.md"
    - "../adr/ruviform-foundation-migration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform 移行の処理単位責務マップ

## Context

ADR `../adr/ruviform-foundation-migration.md` で採用された「処理
単位の責務分割」を、モジュール/処理ごとの移行仕様へ落とす正本。
各行は crate 名ではなく処理単位を正とし、現オーナー→新オーナー、
移す汎用計算/残す固有処理、API の意味・precision・RT 契約、
ライブラリ依存、性能・検証方法、切替順序を定義する。段階の実施
順序・rollback・二重実装撤去条件は
`../plans/ruviform-foundation-migration.md` が担う。

用語:

- **汎用**: 別用途のツールがそのまま使える処理。Ruviform が所有。
- **固有**: AudioLens のゲーム音声デバッグ解析の意味論を含む処理。
  AudioLens が所有。
- **f64 契約**: 解析経路は終端 f64。暗黙の f64→f32 変換は禁止。
- **元 PCM 契約**: 整数 PCM は整数値のまま(unsigned 8bit は -128
  オフセット)、float32 は f64 拡張、float64 は元値。[-1,1] 正規化
  を I/O 境界で強制しない。NaN/Inf をゼロへ置換しない。
- **時刻契約**: raw PTS/DTS・time base・原点・推定値/明示値の区別・
  欠落を保持。欠測は欠測のまま。公称 FPS 等から時刻を捏造しない。
- **RT 契約**: AudioLens 解析パイプラインは全体として非 RT。
  Ruviform の RT/f32 API は変更しない。
- **AL#nnn / R#nnn**: audio-lens / ruviform の GitHub Issue 番号。

## Scope

### In Scope

- `al-math`、`al-media`、`al-locate`、`al-analysis`、`al-job`
  vs `ruviform-exec`、`al-core`、`al-dataset`、`al-viz`、
  `al-observability`、`al-cli`、新 GUI アプリの責務境界。
- GUI/CLI 入口・内部 headless サービス・解析データ
  query/data port・UI 表示 cache・永続 Analysis Dataset・可搬
  export の境界。
- 各行の切替に対応する Issue 番号と検証方法。

### Out of Scope

- 各処理の Ruviform 側 API の具体形状(#150〜#157 の詳細設計)。
- GUI の画面設計・IPC スキーマ詳細(#105 の詳細設計)。
- 実装コード変更・Golden/性能の実測(#98 以降が担当)。
- `docs/plans/ruviform-foundation-migration.md` が持つ段階順序・
  rollback・撤去条件の詳細(ここでは切替単位の所在のみ示す)。

## Components and Boundaries

### 判定基準(ADR の再採)

別用途のツールがそのまま使える処理は Ruviform、AudioLens の解析
判断を含む処理は AudioLens。迷う処理は後述の「境界判定記録」に
記録する。

### 1. `al-math` — 数値・DSP カーネル(最終的に溶解)

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| f64 FFT / Real FFT・plan / workspace (`fft.rs`) | al-math | Ruviform #150 | 全量を移す(残さない) | split re/im 表現・scale・符号・配置・padding の数値規約を明文化。f64・非 RT | pulp(SIMD)→Ruviform 側 SIMD 規約(fearless-simd)に従う | 逆変換 round-trip・境界形状・実サンプル Golden・busy/wall time 比較(#98 基準) | AL#101 |
| NCC / GCC-PHAT / 相互相関・overlap 相関・plan/workspace (`correlation.rs`) | al-math | Ruviform #152 | 全量を移す | 計算範囲・normalization・support 数・極性を関数契約化。ゼロ分散/無音/短窓/端点/同率最大/NaN の出力意味論を明示。f64・非 RT | pulp→Ruviform SIMD | scalar 参照との差分・境界 fixture・`al-math` 同形状入力での correctness/性能比較 | AL#101 |
| 帯域フィルタ・`bandpass` / `bandpass_many` / `BandFilter` / `spectral_envelope` (`filter.rs`) | al-math | Ruviform #153 | 全量を移す | 窓・hop・normalization・端点・帯域中間情報共有を契約化。`spectral_envelope` の fft_size 非ゼロ 2 冪 assert は Ruviform 側 API に引き継ぐ。f64・非 RT | pulp→Ruviform SIMD | `al-analysis::stft`/`band_gain` 経由の Golden・#71 関連の性能比較 | AL#101 |
| リサンプラ (`resample.rs`) | al-math | Ruviform #150 | 全量を移す | 位相・端点・出力長・品質パラメータを契約化。f64・非 RT | pulp→Ruviform SIMD | 既知信号の Golden・歪み/遅延の差分 | AL#101 |
| ベクトル演算 `axpy`/`dot`/`corr0`/`rms`/`mag_sq`/`sub_scaled`/`sum_sq`/`scale` (`vec.rs`) | al-math | Ruviform #150 | 全量を移す | f64 プリミティブ。reduction の順序差は許容誤差規約(`simd-reduction-lane-fma`)として引き継ぐ | pulp→Ruviform SIMD | 等価性マトリクス・許容誤差内差分 | AL#101 |
| linalg(QR/LeastSquares 等、`linalg.rs`) | al-math | Ruviform #150 | 全量を移す | 列指向 API・許容差を契約化。f64・非 RT | pulp→Ruviform SIMD | `caller-side-optimizations` 契約の差分 | AL#101 |
| 独自 SIMD 算術基盤(pulp 採用決定) | al-math | Ruviform 側 SIMD 層 | pulp 利用は残さない | n/a(契約移動なし — SIMD 許容差分は既存 ADR で継承) | pulp 撤去、Ruviform fearless-simd 系へ | #104 の撤去監査 | AL#104 |

### 2. `al-media` — メディア I/O(汎用 I/O は Ruviform、ingest 意味論は AL)

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WAV parser(RIFF、PCM tag1/tag3) | al-media (`wav.rs`) | Ruviform #154 backend | パース・PCM 抽出を移す | 元 PCM 契約。format/bit depth/channel layout 未取得は unknown。非 RT | 純 Rust | 8/16/24/32bit int・f32/f64 fixture、bit depth/尺度の正確性 | AL#100(WAV を最初の切替) |
| AVI demux・WAVEFORMATEX・DIB decode | al-media (`avi.rs`/`dib.rs`) | Ruviform #154 backend | demux/フレーム decode を移す。新境界: Ruviform decode(#154)がフレーム/luma を供給し、AL 側 `video.rs` が特徴抽出を行う。`extract_features` の呼出は `avi.rs` から Ruviform decode 出力経由へ再配線される | 元 PCM 契約・時刻契約。stream 局所破損の区分を保持。非 RT | 純 Rust | 非圧縮 AVI fixture・破損/部分失敗の区分維持 | AL#100(WAV の次) |
| MP4 demux・sample table・PTS/DTS・PCM 抽出 | al-media (`mp4.rs`) | Ruviform #154 backend | demux/timing 抽出を移す | 時刻契約。codec 別 unavailable の区分。非 RT | 純 Rust | MP4 fixture・track/PTS/DTS 観測の網羅性 | AL#100(WAV の次) |
| Symphonia AAC decode | al-media (`symphonia.rs`) | Ruviform #154 backend(io-symphonia) | decode backend を移す | 元 PCM 契約: decoder が生成した値をそのまま保持する(再スケール・正規化・再合成しない)。復号できない codec は明示的 unsupported。Ruviform 側 symphonia 等 decoder version の違いによる出力差は #98 比較で「decoder 差分バケット」として明示分類する(bit-exact 必須ではないが変更理由を記録)。非 RT | symphonia 0.5→Ruviform 側の backend 版決定(#99/#154) | AAC-LC MP4 fixture・fallback 試行記録 | AL#100(AVI/MP4+AAC 段階) |
| `MediaDecoder` trait・registry・fallback 合成機構・format sniffing (`decoder.rs`/`format.rs::detect_format`) | al-media | Ruviform media-io(backend capability) | registry/選択機構・汎用 format 検出(`detect_format`)を移す(#154 scope) | detect/probe/decode 分割・結果区分(unknown/unsupported/malformed/IO)・capability 表現。非 RT。fallback merge の機構は Ruviform へ移すが、merge ポリシーは AL ingest 意味論に残す: audio-only merge(native container facts 優先・上書きしない)・container が Malformed の場合は fallback しない・per-attempt provenance(`attempts`/`fallback`/`fallback_codec`)。capability 合成 hook で AL ポリシーを適用する | n/a(AL 側追加依存なし — backend 依存は Ruviform #154 側で管理) | backend capability table・未対応明示失敗 | AL#100 |
| コンテナ/ストリーム/codec/時刻の基礎観測供給・`observe_timing` 候補計算 (`timing.rs`) | al-media | Ruviform #151 観測契約 | raw 観測事実の供給と `observe_timing` の候補計算(`interval_stats`/`drift_ppm`/`discontinuities`/`duplicates`/`drop_candidates`、閾値ヒューリスティック含む)を汎用観測計算として移す | 時刻契約 + 推定値/明示値・取得不可の区別。ヒューリスティック値は golden で固定する契約 | n/a(AL 側追加依存なし) | VFR/原点ずれ/重複欠落 PTS/複数 track fixture | AL#100 |
| Ingest 意味論:`IngestResult`・`Unavailable` 分類・partial-ingest ポリシー・integrity observation・辞書適用・Manifest/Provenance 記録 | al-media (`stage.rs`/`metadata.rs`/`dictionary.rs`/`video.rs`) | **残す** al-media(ingest adapter 化) | 固有(汎用 I/O を除く全量)。`video.rs`(`VideoFeatureConfig`/`VideoEvent`/`VideoThumbs`/`extract_decoded_features`)は `video_features` 成果物の抽出意味論・検出チューニングとして AL に残す | 現行契約維持: WAV audio decode 不可は input failure、AVI/MP4 は独立有効観測を保存して継続。decode unavailable / corrupt / I/O 失敗の区別 | al-dataset・al-job | 既存 ingest/golden 観測値の維持・#100 検証項目 | AL#100 で段階切替、AL#104 で残量整理 |
| VFR・AV drift・重複/欠落の**解釈**(異常候補の意味付け) | al-media/al-analysis | **残す** al-media/al-analysis | 固有。`observe_timing` の候補計算は Ruviform #151 へ移し(上行)、AL に残るのは候補の「意味解釈」(VFR/AV drift/欠落が game-audio デバッグで何を示唆するか、capture dictionary 適用)のみ | 観測主義: 断定せず候補として記録 | n/a(AL 側追加依存なし) | 既存 golden/integrity 観測値 | AL#100/#103 |
| capture-artifact 辞書 | al-media (`dictionary.rs`) | **残す** al-media | 固有(ゲーム環境固有) | 現行スキーマ維持 | n/a(変更なし — 現行依存のみ) | 既存辞書テスト | 変更なし |

### 3. `al-locate` — ソース定位(意味論は AL、カーネル呼出を Ruviform へ)

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `dsp.rs` のカーネル呼出(re-export 層 + `conv`/`fft`) | al-locate→al-math | Ruviform #150/#152/#153 | 呼出先を Ruviform へ置換。`conv`(overlap-save convolution、plan/workspace を持つ実カーネル)は汎用 DSP として Ruviform 側(#150 系の f64 plan/workspace または #152)へ配置する(境界判定記録)。`fft` は thin wrapper として Ruviform f64 FFT へ置換。`deinterleave` 等の小さな glue は Ruviform buffer/layout API(#151 提供)へ寄せ、等価物が無いもののみ AL 側に残す | 呼出側は f64 契約を要求。decode 境界では native scale を保持するが、`al-locate` は `decode.rs::format_scale` で [-1,1] へ正規化した `normalized_pcm`(amplitude_scale: `decoded-float-normalized`)を生成してカーネルへ渡す。この正規化層は AL 側の解析ポリシーとして残り、scale-sensitive な出力(spectral_envelope・residual energies・least_squares 係数)の契約はこの層の存在を前提とする。ADR の元 PCM 契約は I/O 境界を規定するもので解析側正規化を否定しない | Ruviform f64 API | 同 fixture/形状で旧新実装の Golden・差分・性能 | AL#101 |
| 複数 `LocateMethod` の探索・合議・argmax 曖昧性 (`methods.rs`/`consensus.rs`/`candidate.rs`) | al-locate | **残す** al-locate | 固有(全量) | 合議・曖昧性の証拠レベル契約(既存 ADR)維持。f64 | n/a(AL 側追加依存なし) | Locate 合議・argmax 同率の Golden | 変更なし(意味論) |
| loop/skip/gap/drift の piecewise time map・window anchor (`tracking.rs`) | al-locate | **残す** al-locate | 固有 | time map の現行契約維持 | n/a(AL 側追加依存なし) | tracking/anchor Golden・#76 性能比較 | 変更なし(意味論) |
| ミキサー推定・residual search・候補採否 (`mixer.rs`/`residual.rs`) | al-locate | **残す** al-locate | 固有 | 残差音声の現行契約(`spec-residual-contract-finalization`)維持 | n/a(AL 側追加依存なし) | residual/mixer Golden | 変更なし(意味論) |
| `normalize.rs`/`prepared.rs`/`pool.rs`(run pool)/`result.rs`/`stage.rs` | al-locate | **残す** al-locate | 固有 | run pool の plan 所有境界(`locate-plan-run-pool`)は Ruviform plan/workspace 契約に合わせて再配線。f64 | Ruviform plan/workspace | 共有 cache 監査・性能比較 | AL#101/#102 |
| `decode.rs`(decode adapter) | al-locate | **残す**(切替) al-locate | 固有 adapter。取得元を Ruviform I/O 型へ置換 | 元 PCM 契約・時刻契約を下流へ保証 | Ruviform media-io | ingest→locate の e2e golden | AL#100 |

#101 の切替面には facade 経由に限らず `al_math::` の直接 import も
含まれる: `pool.rs`/`prepared.rs`/`normalize.rs`/`mixer.rs` が
`al_math::` を直接 import しており(facade 迂回)、`lib.rs` は
`al_math::linalg::{condition_number, least_squares}` を公開
re-export している。これら直接 import と公開 re-export も #101 の
切替対象面に含める(Pre-release 規約で API 変更可・成果物は再生成
前提)。

### 4. `al-analysis` — 3 段階解析(数値カーネルは Ruviform、意味論は AL)

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 複素 STFT・PSD/Power/Magnitude・CSD・coherence (`stft.rs`) | al-analysis(経由 al-locate::dsp) | Ruviform #153 | 数値カーネルを移す。中間複素スペクトルの Dataset 保存形式は AL の責務 | 窓・hop・normalization・zero padding 契約化。f64・非 RT | Ruviform #150 plan | `stft.rs` との correctness/性能比較・中間スペクトル golden | AL#101 |
| `band_gain`・`band_ncc` の演算部 | al-analysis/al-locate | Ruviform #153/#152 カーネル + **残す** stage 意味論 | 帯域分割・帯域相関・帯域ゲインの演算を移す。各 stage の端点除外・support・意味付けは残す | f64 契約。帯域中間情報共有 API(#153) | Ruviform band/corr kernel | #71 の構造最適化を含む実サンプル前後比較 | AL#101/#102 |
| Cheap / Reference / Expensive stage 制御 (`cheap.rs`/`reference.rs`/`expensive/`) | al-analysis | **残す** al-analysis | 固有(3 段階の適用制御・candidate/manual/always-run) | 現行契約維持 | n/a(AL 側追加依存なし) | availability/validity/confidence/provenance の維持 | 変更なし(意味論) |
| `FeatureRecord`・Validity・Confidence・Support・Provenance・診断非断定の観測モデル (`records.rs`) | al-analysis | **残す** al-analysis | 固有(全量) | 現行契約維持 | al-dataset | golden・#103 回帰 | 変更なし(意味論) |
| `events.rs`(候補イベント)・`session.rs`・`calibration.rs`・`avsync.rs` | al-analysis | **残す** al-analysis | 固有 | 現行契約維持 | n/a(AL 側追加依存なし) | golden・e2e | 変更なし(意味論) |
| 映像特徴の record 投影(scene/luminance/motion、`video.rs`) | al-analysis | **残す** al-analysis | 固有。`al-analysis/src/video.rs` は `VideoFeatureArtifact` を dataset record に投影するのみで切替対象ではない。フレーム供給元の切替配線は al-media 側(§2)で起きる | f64 契約 | n/a(record 投影のみ — フレーム供給は al-media 経由の Ruviform decode) | 既存 video 特徴 golden | 変更なし(意味論) |

### 5. `al-job` vs `ruviform-exec` — 実行制御の境界

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 不変 JobDefinition/JobInput・Stage 依存順・再開・部分失敗・Manifest・ArtifactRegistry・hash | al-job | **残す** al-job | 固有(全量)。Ruviform へ Job/StageRunner を移さない。依存順の語彙 `PIPELINE_ORDER` は al-core の `StageKind` が所有(§6 al-core 行参照) | 不変ジョブ・安定 id・再現性契約(既存 ADR)維持 | al-core・al-dataset | 再開/部分失敗/ハッシュ golden(#103) | 変更なし(意味論) |
| チャンク駆動オフライン実行(進捗・キャンセル・ワーカープール) | ruviform-exec(既存) | Ruviform(既存)#155 で利用契約を整備 | 移さない(Ruviform 側既存責務) | 逐次参照等価の決定性契約。チャンク独立が呼出側契約 | ruviform-exec | #54 で stage 内並列化適合を実測評価して採否 | AL#54(独立評価、#99 後) |
| headless 実行のライフサイクル/エラー/進捗の汎用契約 | ruviform-exec 周辺 | Ruviform #155 | 移さない(Ruviform 側拡張) | 汎用。業務固有の resume/retry は含めない | n/a(Ruviform 側拡張 — AL 側依存なし) | CLI サンプル・正常/失敗/キャンセル検証 | AL#105 で利用 |

### 6. 共通・永続化・観測・入口

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `al-core`: InputId/JobId/StageKind/InputRole/hash/b64 等の語彙型 | al-core | **残す** al-core | 固有(全量) | `StageKind`/`PIPELINE_ORDER` は Visualization 撤去(#106)で更新。`StageKind::Export`(PIPELINE_ORDER 末尾)は **enum variant を維持**し、実装を `al_viz::stage::ExportStage` から al-dataset 実装へ再配線する(#106 scope)。manifest/provenance の stage 記録と `al run --stages export` の意味は維持。新 `al export` サブコマンド(#105/#106)は同じ headless service 経由で同実装を呼ぶ薄い入口であり、二重実装にしない。Pre-release 規約により既存成果物は再生成前提(互換維持は行わない)だが、stage 語彙自体は維持する | n/a(語彙型のみ — AL 側追加依存なし) | 型契約・シリアライズ golden | AL#106 で StageKind 更新 |
| `al-dataset`: manifest・表・chunked f64 tensor・中間複素 STFT・WAV/f64 WAV・Validity/Confidence/Resolution/Provenance・reader/writer | al-dataset | **残す** al-dataset | 固有(全量)。Ruviform バージョン/backend/precision/変換履歴を provenance に追加記録 | 永続スキーマは AL 責務。Pre-release: 旧形式互換 reader は必須としない(再生成) | serde/保存基盤(現行) | #103 の回帰・reader 回帰・provenance 追跡可能性 | 変更なし+AL#103 拡張 |
| 可搬な解析データ export(`al-viz::package`/`ExportStage`) | al-viz | **移す** al-dataset( export 専用モジュールとして再配置。`GENERATOR_ID`/package schema は #106 で更新) | 表示機能ではない可搬性責務のみ移す。静的 HTML 添付は削除。`StageKind::Export` の variant は維持し、`al_viz::stage::ExportStage` から al-dataset 実装へ再配線する(§6 al-core 行参照) | `analysis/` の可搬コピー・入力参照の相対化・manifest/hash・portability 検証を維持 | al-job manifest | #106 の portability/security 検証 | AL#106 |
| `al-viz` の表示専用部:`VisualizationStage`・`lod`/`tiles`/`timeline`/`views`/`summary`・`report.html`/`report.rs`・visualization JSON/テンソル・viz golden | al-viz | **廃止**(#106) | 撤去対象。再実装しない | n/a(撤去対象 — 契約を移さない) | n/a(撤去対象) | 撤去前に比較用参照資料を保存(#98 Golden 優先順位に従う) | AL#106(#105 検証後) |
| `al-observability`: sink init/format/routing/activation | al-observability | **残す** al-observability | 固有(アプリ契約)。`ruviform-observability` 委譲は別判定 | logging 契約(既存 ADR)維持 | tracing-* | 現行検証(cargo tree/初期化箇所) | 変更なし(#104 監査対象) |
| `al-cli`: run/resume/inspect 入口 + `--stages export` の stage 経路 | al-cli | **残す** al-cli | 固有。`--stages` 表記・コマンド・終了コード・stdout/stderr を所有。現状の export 入口は `--stages export`(al-viz `ExportStage` への stage 経路)であり、実 `export` サブコマンドは #105/#106 の新規作業 | UI 非依存を強制。WebView/GPU/ウィンドウ依存を持たない | headless サービス(後述) | GUI なしで run/resume/inspect/export が動く・headless ビルド | 継続+AL#105/#106 で更新 |
| `al-testkit`: fixture・signal・media 生成 | al-testkit | **残す** al-testkit | 固有。Ruviform 型への fixture 更新のみ | n/a(契約変更なし — fixture のみ) | n/a(変更なし — 現行依存のみ) | #98 baseline の材料供給 | 随時 |

### 7. GUI/CLI・headless サービス・表示 cache・export の境界(#105/#106)

| 処理 | 現オーナー | 新オーナー | 移す汎用計算 / 残す固有処理 | API 意味・precision・RT 契約 | ライブラリ依存 | 性能・検証方法 | 切替順序 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GUI アプリ本体(仮称 `al-gui`) | —(新規) | **新設** AudioLens(#105) | 固有: 画面構成・操作意味・AudioLens の FeatureRecord/イベント/Provenance 表示契約 | GUI は薄いプレゼンテーション層。表示丸め/quantization は元解析データの精度を改変しない(R#157 の区別を遵守) | Ruviform GUI host(#156)・表示 primitive(#157)・headless サービス | #105 完了条件(同一 Dataset を再解析なし閲覧・GUI/CLI 同一結果) | AL#105 |
| 内部 headless 解析サービス(仮称 `al-service`) | —(新規。現状は al-cli が StageRunner 直呼び) | **新設** AudioLens(#105) | 固有: run/resume/cancel/progress/inspect のジョブ操作、解析データ query/data port、構造化 status/error | UI 非依存。GUI/CLI 双方の唯一の解析入口。再解析なしに永続 Dataset を読む | al-job・al-dataset・al-locate・al-analysis・al-media、内部で ruviform-exec 契約を利用可 | #105 で GUI/CLI 同一結果・同一 Job を検証 | AL#105 |
| 解析データ query/data port | なし(al-dataset::reader のみ) | **新設** al-service 内(AL 所有)。Ruviform #157 は汎用 port 契約を提供 | 固有: Analysis Dataset の時間/周波数範囲・解像度指定の読出し。Ruviform 側は任意 source に対する汎用取得 port のみ | 元解析値の精度・信頼度・欠測理由を保持。部分読出しを必須にし全量読込を要求しない | al-dataset::reader + Ruviform #157 port | #103/#105 で巨大 Dataset の部分読出しを検証 | AL#105 |
| UI 表示 cache(LOD・タイル・ビューポート用中間表現) | al-viz(永続 Visualization Dataset として実現) | **実行時 cache**: al-gui 内一時 cache + Ruviform #157 の汎用 LOD/タイル計算 | 固有: cache のキー・無効化・アプリ寿命。汎用: LOD/min-max/peak/RMS・タイル生成(#157) | 表示用途の丸めは可、元データ改変は不可。永続物として保存しない | Ruviform #157 | 長尺データの応答・メモリ計測(#105) | AL#105 |
| 永続 Analysis Dataset | al-dataset | **残す** al-dataset(上表 6 参照) | 固有(全量) | 変更なし | n/a(変更なし — 現行依存を維持) | #103 回帰 | n/a(切替なし — 上表 6 の通り変更なし) |
| 静的 HTML レポート・Visualization Stage | al-viz | **廃止**(#106)。GUI/CLI が代替検査経路 | 撤去対象 | n/a(撤去対象 — 契約を移さない) | n/a(撤去対象) | #105 稼働・検証後に撤去 | AL#106 |
| 可搬 export CLI 入口 | al-viz(ExportStage) | **移す** al-cli(export コマンド)+ al-dataset(実装) | 可搬性責務のみ。HTML 添付は削除。新 `al export` サブコマンド(#105/#106)は headless service 経由で同じ al-dataset 実装を呼ぶ薄い入口であり、二重実装にしない | 現行の相対化・manifest/hash・portability 検証を維持 | n/a(headless service 経由 — 直接依存なし) | #106 検証 | AL#106 |

## Data and Control Flow

移行後の依存方向(上位が下位に依存。Ruviform → AudioLens の
逆依存は存在しない):

```text
al-cli ─┐
        ├─> al-service ─> al-job ─> stage ─> al-locate/al-analysis ─┐
al-gui ─┘        │             │                                   ├─> Ruviform カーネル(#150/#152/#153)
        └─> Ruviform GUI host (#156)・表示 primitive (#157)          └─> Ruviform media-io + backend (#151/#154)
al-gui/al-cli ─(query/data port 経由)─> al-service ─> al-dataset(永続 Analysis Dataset + 可搬 export)
al-cli/al-service/al-gui ─> al-observability(init はプロセス入口のみ)
```

- `al-cli`/`al-gui` は `al-service` のみを解析入口として呼び、
  `StageRunner` を直接抱えない(#105 で構成)。
- `al-gui` は永続化層を持たず、表示に必要な範囲を query/data
  port 経由で取得し一時 cache に展開する。
- query/data port は AL(al-service)が所有・実装する(AL→port→AL)。
  payload は汎用表現(quality/availability 等の列挙値)とし、AL の
  Validity/Confidence/欠測理由は port 内でマッピングする。
  Ruviform API に AL enum を持ち込まない。
- `al-viz`・Visualization Stage・静的 report は #106 で撤去され、
  パイプライン上の `StageKind::Visualization` も消える。

## Non-Functional Constraints

- 精度: f64 契約・元 PCM 契約・時刻契約は API 境界の必須要件。
  暗黙の f64→f32、[-1,1] 正規化強制、欠測の捏造を認めない。
- 性能: 各切替は #98 baseline と同一入力で wall/busy-time・メモリを
  比較し、退行は修正または明示承認。暫定 adapter は目的・削除条件・
  Issue を付記する。
- 依存方向: AudioLens → Ruviform の一方向。AudioLens 固有型
  (Job/InputRole/StageKind/Validity/Confidence/FeatureRecord/
  イベント意味)を Ruviform 公開 API に持ち込まない。
- CLI/headless: `al-cli` と解析コアは WebView/GPU/ウィンドウ/
  フロントエンドアセットに依存しない。GUI backend は al-gui 側の
  optional deps に隔離。
- 互換性: Pre-release 規約により旧永続形式・CLI 出力の互換層は
  要求しない。変更は再生成で扱い、比較は #98 の保存済み Golden で。

## Risks and Trade-offs

- Ruviform 側の観測契約(#151)が AudioLens の必要事実を欠くリスク:
  責務マップの「API 意味」列を #151 詳細設計の入力要件として相互
  参照し、欠落は Ruviform Issue で対処(AL 側 adapter で補完
  しない)。
- `al-locate::dsp` の facade を残したまま二重委譲が固定化する
  リスク: facade の呼出先変更(#101)と二重実装撤去(#104)を別
  ゲートで管理し、facade の存否は #102 の責務分離で再判定。
- `al-service` 新設が過剰になるリスク: GUI/CLI 共通入口が無ければ
  Job/Dataset 呼出が二重化する。boundary が必要なのは #105 で確定
  済みであり、crate 名は仮称として詳細設計で再確認する。
- 可搬 export の帰属先判断: `al-dataset` を仮置きとし、#106 の
  棚卸しで portability 検証との整合を確認して確定する。

## Deferred Design Concerns

- `al-service`/`al-gui` の crate 名・公開 API・IPC adapter |
  #105 の詳細設計 | #105 着手 | 仮称のまま実装に入ると rename が
  波及する。
- `ruviform-exec` OfflineDriver の stage 内利用 | #54 の独立性能
  評価 | #99 後の実測 | 未評価採用は決定性契約違反リスク。
- `al-observability` の Ruviform 委譲 | 移行目的ではない | #104
  監査 | 未評価委譲は logging 契約の揺れ。
- 永続スキーマの新形式(Parquet/Zarr 等) | #103 の追加位置づけで
  明示的に別スコープ | 別 Issue | 移行に混入すると範囲が発散。

## 境界判定記録

「別用途のツールがそのまま使えるか」と「AudioLens の解析判断を
含むか」で迷った処理の判定結果を記録する。

- `al-media/src/video.rs` の分割(§2): Ruviform decode(#154)が
  フレーム/luma を供給し、AL 側 `video.rs` が `video_features`
  成果物の抽出意味論・検出チューニングを担う。特徴抽出は AL に
  残す。
- `observe_timing` の配置(§2): 候補計算
  (`interval_stats`/`drift_ppm`/`discontinuities`/`duplicates`/
  `drop_candidates`、閾値ヒューリスティック含む)は汎用観測計算と
  して Ruviform #151 へ配置する。ヒューリスティック値は golden で
  固定する契約。AL に残るのは候補の意味解釈のみ。
- `dsp.rs::conv` の配置(§3): overlap-save convolution
  (plan/workspace を持つ実カーネル)は汎用 DSP として Ruviform 側
  (#150 系の f64 plan/workspace または #152)へ配置する。
- `StageKind::Export` の扱い(§6): enum variant は維持し、実装を
  `al_viz::stage::ExportStage` から al-dataset 実装へ再配線する
  (#106 scope)。stage 語彙と `al run --stages export` の意味は
  維持し、新 `al export` サブコマンドは同実装を呼ぶ薄い入口と
  する。

### AL#102 判定(解析意味論と汎用カーネルの責務分離)

#101 切替後に残った「意味を持たない演算」の境界を確定した。

- `al-locate::dsp` facade の存否(§3): **暫定 adapter として維持**。
  Ruviform の検証 `Result` を旧 `al-math` と同じ panic 契約へ
  写像する責務を一箇所に集約する。撤去条件は AL#104 — 全呼出側
  が `ruviform_numeric` を直接呼ぶ構成へ移行し panic 契約の帰属
  が再評価された時点。`deinterleave` は AL の入力境界処理として
  恒久残留(`decode.rs::PcmView` の scale 除算 fused 版は decode
  adapter の責務として別実装を維持 — 取得経路が異なり責務も別)。
- Welch 蓄積・H1 伝達関数(§4): `reference.rs::fir_ir` と
  `expensive/channel.rs` の帯域 L/R coherence 蓄積は
  `ruviform_numeric::xspec::CrossSpectrum` へ寄せた(per-bin 加算
  規約が同一で bit-exact。`transfer_h1_into` の `pxx > 0.0`
  ゼロガードも同一)。`channel.rs` は第三オクターブ帯選択と
  `|coherence|` 平均を AL 側に残し、蓄積のみ全 bin 化
  (per-bin 独立のため `band_bins` の値は不変)。
- `expensive/channel.rs::csolve`(§4): 複素 Gaussian 消去法ソルバ。
  `ruviform-numeric` の公開 API に複素線形ソルバが存在しないため
  AL に残留(汎用だが委譲先なし)。Ruviform 側への移管は契約提案
  として IR に記録。
- `cheap.rs` の IPD lag sweep・`analyze_pair` の `corr0` スキャン
  (§4): `OverlapCorrPlan` への置換は FFT 分子経路のため直算
  `corr0` と FP 差分が生じ、#102 の挙動不変条件を満たさない。
  直算 sweep は AL に残留。Ruviform に scalar 直算 overlap-NCC
  経路が提供されれば移行可能(契約提案として IR に記録)。
- スペクトル統計・OLS・argmax/同率・状態機械スキャン
  (`cheap.rs`/`reference.rs`/`expensive`)(§4):
  centroid/flatness/rolloff/entropy/flux/bandwidth/slope・
  true-peak 補間・clip/zero/stuck/discontinuity 走査・median/
  peak 選択・onset 検出等は「基礎スペクトル統計」としては汎用だ
  が、Ruviform 側に対応 API がなく AudioLens の観測仕様
  (端点除外・support 数え・window 分割・閾値)と一体のため AL
  に残留。`cheap.rs::analyze_input` は measure 系関数
  (`measure_level`/`measure_digital`/`measure_spectral`/
  `measure_stereo`/`measure_transient`)へ演算を分離し、
  `FeatureRecord` 変換(validity/confidence/support 写像・
  tensor 出力)のみを本体に残す層構造とした。これにより汎用
  演算の塊が「Ruviform 契約待ちの measure 層」として可視化
  される。`reference.rs::analyze_pair` ・ `expensive` 各 family
  `run` は feature 単位の計算+emit ブロック構造を維持 —
  各ブロックの計算が帯域選択・validity gating・median/argmax
  選択の意味論集約であり、残る inline 演算は AL 意味論側と
  判定。同型の measure 分離は AL#104 の監査対象。
- `residual.rs::rms`(§3): 複数チャンネル合成エネルギー
  (Σsum_sq/Σlen)は `dsp::rms`(単一チャンネル mean)とは別意味論
  の AL ローカル helper として残留。
- `normalize.rs`/`prepared.rs` の Ruviform 型混入(§3):
  `ResamplePlan` の内部パラメータを `normalized_pcm`
  永続スキーマ/cache 鍵へ直列化している。永続契約は AL 側の
  責務(#98 baseline 互換)のため現行維持とし、Ruviform 側の
  安定 ID/バージョン化されたパラメータ表現があれば移行する
  契約提案として IR に記録。

## Verification Notes

- [ ] 本表が棚卸し対象の全 crate・全境界(GUI/CLI/headless/
      query/cache/Dataset/export)を網羅し、各行が 6 項目を満たす。
- [ ] f64・元 PCM/PTS・RT/非 RT・汎用/固有の記述が ADR と一致する。
- [ ] 各行の切替順序が Issue 番号と対応し、
      `../plans/ruviform-foundation-migration.md` の DAG と矛盾しない。
- [ ] Ruviform に残るべきでないもの(AudioLens 固有意味論)が
      「残す」列に明示的に配置されている。
