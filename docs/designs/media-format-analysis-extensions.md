---
id: "DESIGN-034DlwOAFNKWY4VRafyYzo"
type: "design"
status: "approved"
title: "メディア形式拡大・正規化アーキテクチャと解析精度拡張"
created: "2026-10-10"
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
    - "docs/specs/media-ingest.md"
    - "docs/adr/mp4-demux-decode.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/media-format-analysis-extensions.md"
  references: []
  defers: []
  deferred-by: []
---

# メディア形式拡大・正規化アーキテクチャと解析精度拡張

## Context

approved plan `docs/plans/media-format-analysis-extensions.md` の #36 フェーズ1、
#38、#39、#41 を実装可能な責務境界に分解する draft design。
本書は承認を代行せず、現行の MP4 demux-only ADR を変更しない。

現行 `al-media/src/stage.rs` は `detect_format` 後に WAV/AVI/MP4 を直接分岐し、
`MuxObservations` と `ingest_decoded` で成果物を登録する。WAV は decode 不能で
入力を失敗させる一方、AVI/MP4 は利用可能なタイミング・コンテナ観測を残し、
音声や映像の unavailable を明示する。`video.rs::extract_features` は DIB decode、
全画面輝度・差分・イベント、thumbnail 作成を同じループで行う。

`al-analysis/src/reference.rs::analyze_pair` は受理済み alignment を入力とし、
現在の `ref.band_gain_m2` は帯域 PSD 比の平方根である。
`ReferenceAnalysisStage` はペア単位の records/tensors を書き、キャプチャ間を
集約しない。`avsync.rs` は映像 tick を `start_tick` からの秒へ変換し、
音声 onset と決定的な最近傍照合を行う。これらの観測・来歴契約を設計の起点とする。

## Scope

### In Scope

- #36: `MediaDecoder` と registry、既存 WAV/AVI/MP4 の native backend 移行。
- #38: 1/3oct 帯域のラグ補償 LS ゲイン、相関上位半分の頑健な代表値。
- #39: job 宣言による複数 ROI の輝度系列・ステップと avsync 接続。
- #41: 同じソースの受理済みペアからキャプチャ間経路差を集約・永続化。

### Out of Scope

- ffmpeg/symphonia 等の依存導入、主要 codec の実 decode 対応の追加。
- MP4 ADR の承認・変更、ゲーム側原因の診断、既存 raw 観測の上書き。
- viz UI の新規実装。表示側は本設計で保持した系列・観測を後続作業で利用する。

## Components and Boundaries

### MediaDecoder と registry

`al-media` に `MediaDecoder` を置き、概念上 `detect(bytes) → probe(facts) →
decode(request)` を統一する。probe はコンテナ・stream・codec・クロック・
能力(audio/video/timing)を返し、decode は要求 stream ごとに decoded または
理由付き unavailable を返す。壊れた入力・I/O・能力不足を区別し、能力不足だけを
将来の fallback 選択条件とする。入力破損を別 backend の成功で隠さない。

registry は利用可能な backend と決定的な優先順位を所有し、native を最初に試す。
既存 `detect_format` の magic-byte 判定を native adapter に集約し、拡張子で
推測しない。WAV/AVI/MP4 の parser を書き直さず adapter から呼び出す。
MP4 は現在どおり probe/demux/timing のみで、codec decode は unavailable。
登録と能力選択を分離することで、将来の純 Rust 音声 backend、次に明示有効化された
ffmpeg backend を追加できるが、今回の registry に未実装 backend を登録しない。

共通出力は既存 `DecodedAudio`、タイミング/メタデータ/完全性観測、
映像抽出結果を一つの ingest 結果にまとめる。`MuxObservations` の役割をこの
境界に移し、stage は hash 検証・辞書注釈・成果物登録を担当する。
辞書は生観測を抑制しない。部分 ingest の可否は共通結果に基づく一つの stage
ポリシーで決め、backend ごとの成果物登録や caller の補償処理を作らない。

正規化とは型と座標系の統一を意味する。元の decoded PCM と各チャンネルを保持し、
暗黙の resample/downmix はしない。必要な変換は設定・元 rate/layout・変換後
rate/layout・処理順を一箇所で記録する。音声/映像クロック、PTS/DTS の有無、
packet 長、gap/overlap を保持し、公称 FPS で実時刻を作らない。
provenance は backend ID、実装/実行バージョン、設定、要求能力、試行順と理由、
fallback 有無、入力 hash を記録する。後段は backend を分岐条件にしない。

### 共通フレーム特徴抽出と ROI

DIB の pixel decode を native の責務とし、top-down の luma plane とサイズ、
frame tick/timebase/start、必要な RGB preview 入力を共通特徴抽出へ渡す。
`VideoExtraction` の features/thumbs を返す境界は維持する。これにより将来の
圧縮 codec も同じ ROI・全画面・thumbnail パイプラインを利用できる。

`VideoFeatureConfig` に `rois: Vec<Roi>` を追加し、job に名前、x/y/w/h、
輝度ステップ閾値を宣言する。座標は元画像の左上原点・pixel 単位・半開矩形。
名前の一意性、正のサイズ、有限の非負閾値を config 境界で検証し、frame サイズが
分かった時点で矩形の範囲と加算 overflow を検証する。暗黙 clip はせず設定エラーを
明示する。ROI 未指定は空配列で従来の全画面抽出・照合を維持する。

一度 decode した luma plane から全 ROI の平均輝度を求め、共通 frame_timestamps
に対応する名前別系列と採用矩形・閾値を `video_features` に保存する。
連続 frame の平均差の絶対値が閾値以上なら ROI 名付き step event を作る。
初回 frame は step としない。欠落/不連続を跨ぐ差を正常な step としない。
映像 decode 不可は既存の `VideoFeatureArtifact::Unavailable` を使う。

avsync は ROI ID を保持したまま `rebase_events` の同一クロック変換を適用する。
名前別に全画面/ROI の照合系列を分離し、一つの ROI が別 ROI の候補を奪わない。
系列内では現在の一対一・最近傍・決定的 tie-break を維持する。
ROI 名、frame、元 tick、audio onset、delta、confidence/support を出力し、
字幕出現を音声原因の診断と呼ばない。

### reference 帯域ラグ補償ゲイン

受理済み alignment の重なり領域を、同じ 1/3oct 境界と同じ bandpass で source/
capture とも処理する。帯域境界生成と filter 適用は reference 内で一箇所に定義し、
チャンネル別結果を保持する。既存フルバンド lag 設定 `lag_max=512` と独立に、
帯域用の窓 0.1 秒・hop 0.05 秒・最大 lag ±0.04 秒を job config に持ち、
実 rate から sample 数に変換して保存する。48kHz の探索幅は ±1920 samples。

各窓で有効な overlap に対する正規化相関の絶対値 `|correlation|` を最大化する
lag を探索し、同点は
|lag| が小さいもの、次に符号の固定順で決める。lag の正方向は capture が遅れる
方向に統一し、補償した source x と capture y で `g = Σ(x*y)/Σ(x*x)` を求める。
filter 過渡と境界不足を除外し、無音・分母不足・探索端の peak・低相関は理由付きで
記録する。低相関判定にも `|correlation|` を使う。選択に使う絶対値とは別に、
選択 lag の signed correlation と signed LS gain を保持し、dB は |g| > 0 の場合だけ
導出する。`y=-x` の真の lag は相関 -1 でも選択対象となる。
これは `crates/al-locate/src/methods.rs::peaks_from_correlation` と
`crates/al-locate/src/consensus.rs` の絶対値選択・符号保持と整合する。

帯域ごとに数値計算可能な窓を signed correlation と LS gain がともに正の通常極性群、
ともに負の反転極性群へ分ける。ゼロまたは符号不整合の窓は理由付きで保持し、
代表値から除外する。各群を `|correlation|` 降順で並べ、同点を時間順で固定して
群内の上位 ceil(N/2) の signed gain の中央値を、それぞれ独立した代表値とする。
N は各群の窓数であり、反転極性の値を通常極性の代表値に混ぜない。
空の群は unavailable とし、他方の群の値で補完しない。
各群の極性、選択数、群内窓数、全窓数、signed correlation とその絶対値、coherence、
MAD、lag、validity を残す。上位半分でも `|correlation|` が低ければ高信頼へ昇格させず、
低相関の値には下限的解釈というメタデータを付ける。
400Hz 相対ゲインも同じ推定器の同じ極性群で支持された 400Hz 帯域から計算する。
対応する 400Hz 群が未支持なら相対ゲインは unavailable とする。

`ref.band_gain_m2` をこの LS 推定の正本とし、旧 PSD 比は別の明示的な
`ref.band_gain_psd_ratio` 二次観測として保持する。algorithm/provenance に
推定方法と全設定を記録し、同じ ID に二つの意味を混在させない。
window×band の lag/gain/correlation/support は `NamedTensor`、代表値は
`FeatureRecord` とし、alignment_id、channel、freq_range を引き継ぐ。
低 support の PSD 比を周波数応答の根拠に使わない。

### キャプチャ間経路校正

ペア計測 `analyze_pair` に他の capture を読ませず、`ReferenceAnalysisStage` が
全ペアを計測した後、writer.finish 前の job 集約として校正を実行する。
対象は job の明示的な baseline/target capture 組で、source_input_id と pinned
content hash が同一の受理済み alignment を join する。ファイル名近似では join
しない。チャンネル対応・rate・周波数 grid・計測条件が整合するもののみ使用する。

同じ source の重複区間はまず source 内で集約し、一つの長い録画や多数の窓が
source 間の中央値を支配しないようにする。各 source の `H_target/H_baseline` を
複素比として計算し、十分な coherence/support と非ゼロ分母の bin だけを使う。
grid が異なる場合は今回無理に補間せず unsupported とする。

source ごとの `20*log10(|H_target/H_baseline|)` と帯域ゲイン比を記録し、
共通 source 全体の magnitude dB 中央値/MAD/support を経路差観測とする。
位相は複素比の実部/虚部とともに保持し、±π の折返しを算術中央値で扱わない。
位相集約が支持できない bin は unavailable として magnitude と分けて示す。
共通 source なしは `validity=unsupported`、低 coherence/分母不足は各理由付きで
出力し、ゼロ dB の正常値で穴埋めしない。

capture 比較の識別子・baseline/target input ID・各 source/alignment の集合を
一つの比較コンテキストに定義し、FeatureRecord と tensor の lineage/provenance
から参照する。現行の単一 input/alignment 用フィールドに複数 ID を連結しない。
dataset 表現の追加は実装時にこの責務の一箇所で定義し、既存ペア結果を保持する。
表示/後段解析は raw 差と校正後差を並べ、校正に使った共通 source と support を
示す。校正値を raw transfer_h や R0〜R3 に書き戻さない。

## Data and Control Flow

実装順序の前提ゲートとして、backend 選定 ADR(TASK-036a)を accepted にしてから
#36 フェーズ1の既存 WAV/AVI/MP4 の native 移行へ進む。codec backend の依存導入時
だけのゲートにはしない。MP4 demux-only 決定の維持または supersede/amend の要否も
その ADR で確定し、本書の draft 記述だけで現行決定を変更しない。

1. ingest: pinned hash 検証 → registry/native probe/decode → 共通 PCM・frame・
   timing 結果 → 全画面/ROI 特徴 → provenance/manifest 登録。
2. locate: 既存 PCM とクロックによる alignment を受理し、reference へ渡す。
3. reference: ペアごとに既存 transfer/residual と帯域 LS 系列を計測する。
4. reference 集約: 明示 capture 組の共通 source を join して経路差を追加する。
5. avsync: 音声 onset と名前別 ROI event を各 stream 原点に rebase して照合する。
6. dataset/export: 生系列・窓系列・代表値・invalidity・来歴を正本に保持する。

## Non-Functional Constraints

- 再実行では registry 順、lag tie-break、ROI 順、source 集約順を固定する。
- bandpass をペア/チャンネル/帯域ごとに再利用し、FFT 相関で lag 探索のコストを
  抑える。時間領域の小さい oracle と一致させ、線形相関の zero padding を守る。
- ROI は縮小 thumbnail から測定せず元 frame を使い、複数 ROI でも decode は一回。
- tensor は既存 `write_tensor` と chunk budget を利用し、job 共通 dataset session
  に保存する。無効値は既存 null/validity の表現を用いる。
- リリース前方針に従い、意味変更後の成果物は再生成する。互換層は導入しない。

## Risks and Trade-offs

- 0.1秒窓は低域の周期数が少ない。support を記録し、短窓の低域を確実な推定としない。
- 周期信号の lag 多峰性や探索端 peak は不確実性を残す。探索幅を広げるだけでは
  受理済み alignment の誤りを修復できない。
- 共通 source のゲーム内 mix 自体が変わる場合、差は純粋な録画経路差とは限らない。
  複数 source の一致/MAD と計測条件を提示し、原因確定や自動除去をしない。
- codec decode の拡張は再現性・配布コストを伴う。native 移行と backend 導入を分ける。

## Deferred Design Concerns

- backend 選定 ADR の accepted 化と MP4 ADR の supersede/amend | 現行決定を維持 |
  TASK-036a のレビュー時、native 移行開始前の必須ゲート | 承認なしの移行・codec 対応拡大。
- #41 比較コンテキストの永続化スキーマ | 現行 `al-dataset` の `FeatureRecord` は
  単一の `input_id` / `alignment_id` と `lineage`、manifest の `DatasetArtifact` は
  `role` / `path` / `provenance` を持ち、比較専用契約は未定 |
  #41 実装開始前に比較コンテキストの role/path、安定識別子、FeatureRecord/tensor
  からの参照 field、再読込スキーマと参照検証を一箇所で確定 |
  再読込時に baseline/target と source/alignment 集合を復元できないリスク。
- symphonia/ffmpeg backend と codec coverage | plan のフェーズ1外 |
  依存承認と後続計画の成立時 | 主要キャプチャの PCM は引き続き unavailable。
- ROI/経路差の viz view | 解析データの契約を先に固定 |
  後続 export/UI 作業時 | 生系列の目視比較に留まる。

## Verification Notes

以下は実装時の検証項目であり、本書作成によるテスト成功を意味しない。
正本は `docs/test-specs/media-format-analysis-extensions.md` の TASK-036a〜e。

- [ ] accepted backend ADR(TASK-036a)と MP4 ADR の関係を確認し、native 移行前にレビューする。
- [ ] native registry 経由で既存 WAV/AVI/MP4 ingest/golden を通し、部分 unavailable、
  timing、PCM/channel、hash と backend provenance を確認する。
- [ ] ROI の既知輝度、複数矩形、top/bottom-up、範囲外設定、VFR/欠落、未指定時の
  既存結果、既知 onset 差の avsync を確認する。
- [ ] 帯域別既知 gain/lag、rate 差、無相関高域雑音、無音、極性、上位半分中央値、
  400Hz 比、FFT oracle 一致と低 support validity を確認する。
- [ ] 同じ既知 lag の `y=x` / `y=-x` が絶対値相関で選ばれ、signed correlation/gain を
  保持すること、極性混在窓の群別上位半分集約と低相関判定が絶対値を使うことを確認する。
- [ ] 同一 source の既知 -5dB 経路差、複数 source の外れ値、位相折返し、分母ゼロ、
  grid 不整合、共通 source なしの unsupported、既存ペア観測保持を確認する。

## References

- [approved plan](../plans/media-format-analysis-extensions.md)
- [reference-analysis spec](../specs/reference-analysis.md): 帯域・複素伝達・残差契約。
- [test-spec](../test-specs/media-format-analysis-extensions.md)
- [ingest design](ingest-av.md)、[video design](video-features.md)、
  [dataset design](dataset-schema.md)、[MP4 design](mp4-ingest.md)。
- GitHub issues [#36](https://github.com/scarecrow173/audio-lens/issues/36)、
  [#38](https://github.com/scarecrow173/audio-lens/issues/38)、
  [#39](https://github.com/scarecrow173/audio-lens/issues/39)、
  [#41](https://github.com/scarecrow173/audio-lens/issues/41)。

チェーン解決の上流は front matter の media-ingest spec と MP4 ADR 各一件に限定し、
その他の仕様・計画・設計は本節の補助参照として扱う。
