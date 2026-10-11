---
id: "DESIGN-034AasyGL3izivzyzZjS0j"
type: "design"
status: "approved"
title: "映像フレーム特徴と A/V 同期設計"
created: "2026-10-04"
updated: "2026-10-11"
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
  depends-on:
    - "ingest-av.md"
    - "analysis-stages.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines:
    - "ingest-av.md"
  refined-by: []
  derives-from:
    - "../specs/video-av-timing.md"
    - "../specs/candidate-event-model.md"
    - "../discovery/video-av-feature-gaps.md"
    - "../adr/avi.md"
    - "../adr/implementation-stack.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 映像フレーム特徴と A/V 同期設計

## Context

spec `../specs/video-av-timing.md` はシーンチェンジ・輝度変化・
モーション系列と音声↔映像イベント同期を要求するが、現状は
コンテナ宣言メタデータとタイミング観測まででフレーム内容を
扱わない。ADR `../adr/avi.md` に基づき、非圧縮 AVI
(DIB/rawvideo)に限定してフレームデコードと映像特徴・
A/V 同期を設計する。

## Scope

### In Scope

- 非圧縮 AVI の `vids` フレームデコード(BI_RGB 系、24/32bpp、
  bottom-up / top-down)。height の符号で行順序を判別し、
  24bpp 行の DWORD パディングを正しく扱う。フレームを運ぶ
  チャンク(db/dc)と非フレームチャンクを区別し、タイムスタンプ
  系列とフレーム index のずれを防ぐ
- フレーム単位の粗い特徴: 平均輝度系列・frame-to-frame 変化量
  (モーション/アクティビティ)・シーンチェンジ候補タイムスタンプ
- 映像イベント時刻系列の永続化。生成するイベント種別を列挙:
  `scene_change`(差分閾値超過)・`luminance_step`(輝度階段、
  spec のフラッシュ例に対応)・`motion_onset`(差分の連続上昇
  開始)。タイムスタンプはストリームクロック由来、
  公称 FPS 演算に依らない
- 音声オンセット/イベントと映像イベント時刻の汎用照合
  (差分・確度・分布・ドリフト傾き)。照合前に両系列を各々の
  宣言時計を秒に変換し、音声 PCM 開始を共通原点にする
  (下記 Data and Control Flow)
- コンテナレベルのオフセット(auds/vids の `start_tick` 差)は
  既存 `StreamTiming` の per-stream 値から導出可能なため、
  専用レコードは追加せず導出元事実として永続化済み値を使う
- 部分 ingest 契約(下記): `auds` が欠損・デコード不可でも
  映像観測を記録して ingest を完結させる
- 映像証拠が利用不能な場合の explicit な `decode/input
  unavailable` または not-applicable 記録

### Out of Scope

- 圧縮コーデックのフレームデコード(別決定)
- ゲーム固有の映像検出器
- 音声アラインメント結果との統合判定
- 生フレームの dataset 永続化(容量・決定性の観点で粗い系列のみ)

## Components and Boundaries

- `al-media::video` コンポーネント: AVI `vids` チャンクを
  フレーム単位でデコードし、平均輝度とフレーム差分を算出する。
  デコードと同じパスで粗い特徴を生成するためフレームを
  ストアしない
- `al-media` ingest 成果物: `video_features`(フレームごとの
  輝度・変化量・シーンチェンジ候補フラグ・イベント種別)を既存
  `StreamTiming` / `TimingObservation` と同じクロックで永続化
- 部分 ingest 契約: 現行 `IngestStage` は `auds` デコード失敗を
  `StageError` に伝播し、成果物が一切登録されない。本設計では
  コンテナ観測(metadata・タイミング)は decode の可否と独立に
  記録し、PCM のみ `Option` として扱う。`auds` が欠損・圧縮・
  デコード不可でも `video_features` とメタデータ/タイミングは
  永続化され、decoded PCM は省略される。1 入力の失敗は他入力の
  記録を巻き込まない(per-input isolation)
- `al-analysis::avsync` コンポーネント(cheap stage 内):
  ingest の映像イベント時刻系列と cheap の音声オンセット
  (`trans.onset_times`)を照合する。音声側・映像側イベント系列が
  そろわない場合は該当レコードを validity で明示する
- 境界: `al-media` はコンテナとフレームだけを知り、音声解析を
  知らない。`al-analysis` は ingest 成果物と cheap 特徴だけを
  読み、コンテナを直接読まない。`ingest-av.md` の
  「内容解析は解析ステージの責務」との整合: 本設計が ingest に
  置くのはフレームデコードと不可分の粗い特徴(輝度/差分/
  シーンチェンジ候補)であり、意味的なイベント照合は
  `al-analysis` に置く。この境界明確化をもって `refines`
  関係を宣言する

## Data and Control Flow

- ingest: `vids` が非圧縮ならフレームごとに輝度・差分を算出し
  `video_features` として永続化(圧縮なら decode 不可を記録)
- cheap: 音声オンセット系列を既存どおり算出。onset 時刻は
  PCM 開始からの秒のまま扱う。選択された音声 stream の
  `start_tick / timebase_hz` は ingest metadata の decoder 選択結果と
  `StreamTiming` から読む(secondary stream の原点と取り違えない)
- avsync(cheap stage 内): 各音声イベントに対し設定可能な探索窓内
  の最近傍映像イベントを対応付け、ペアごとに
  `avsync.audio_time` / `avsync.video_time` / `avsync.delta` /
  confidence を記録。映像は `event_tick / video_timebase_hz -
  audio_start_tick / audio_timebase_hz` で音声 PCM 開始を共通原点とする。
  両 stream の共通シフトのみ相殺し、非ゼロ `dwStart` が表す実相対
  offset は delta に保持する(#28 判断承認: 2026-10-11)。時計が不明・
  不正なら `ComputationFailed` とし、ゼロ原点を補わない。対応ペア列から
  median/MAD と線形フィットによる drift 傾きをサマリ化
- すべて秒単位のストリームクロック時刻で扱い、nominal FPS を
  時刻計算に使わない

## ROI 輝度観測(#39)

- `config.ingest.video.rois` は `[{"name":"字幕", "rect":
  {"x":0,"y":900,"width":1920,"height":180}}]` の形で指定する。
  矩形は表示上の左上原点の整数ピクセル座標、右端・下端は含まない。
  DIB の行順序・パディングを解釈した後の top-down luma を使うため、
  bottom-up と top-down で同じ位置を観測する。
- 名前は空白だけを禁止し、同一設定内で一意にする。矩形の幅・高さは
  正、座標の加算は overflow 不可。形式・名前の不備は ingest 設定
  エラー、フレーム外の矩形は当該映像の explicit unavailable とする。
  黙って clip したり全フレームへ fallback したりしない。
- `VideoFeatures.roi_luminance` は ROI 定義(`roi.name`・`roi.rect`)と
  フレームごとの `luminance` を保持する。各系列の index は既存の
  `frame_timestamps` と一致する。全フレームの `luminance`・`diff` と
  既存イベントは ROI 指定時も算出し、未指定時は従来の観測を維持する。
- ROI 平均輝度の絶対差が既存の `luminance_step_threshold` 以上なら、
  `luminance_step` イベントを同じ stream-clock timestamp で追加する。
  `VideoEvent.roi_name` は ROI イベントの名前を保持し、全フレームの
  イベントでは null。イベントを avsync の既存最近傍照合へ渡し、
  ペアの FeatureRecord `params.roi_name` に同じ名前を保持する。
- cheap stage は `video.roi_luminance` FeatureRecord を ROI ごとに
  出力し、`params.roi` に定義、`params.frame_times` に時刻 tensor の
  参照を記録する。輝度 tensor は `[1, frames]`、単位は
  `luma_0_255`、パスは `features/cheap/<input>/video/roi_<index>/luminance`。
  名前はパスへ埋め込まない。実際のフレーム時刻を stream origin から
  秒へリベースした `[1, frames]` tensor を `video.frame_times` として
  出力する。nominal FPS による時刻復元は行わない。
- ROI 観測は音声のデコード可否とは独立して dataset へ出力する。
  tensor とレコードは入力および ingest 映像成果物の lineage を持つ。
  ROI 未指定時は新しい ROI レコード・tensor を出力しない。
- ROI 名を観測・イベント・照合ペアに共通のキーとして残し、後続の
  marker 取り込み(#31)で名前から対応付けできるようにする。
  marker の読込形式や意味的な照合規則は本変更では導入しない。

## Non-Functional Constraints

- フレームは特徴算出後に保持しない(追加分のメモリ確保を増やさ
  ない)。なお現行 ingest は入力ファイル全体を `fs::read` で
  バッファしており、本 WS はその I/O モデルを変えない。
  全入力バッファリング自体は別 ADR で扱う既知のスケーリング
  制限として記録する
- シーンチェンジ閾値は設定可能、かつ採用値を成果物に記録
- 音声解析は映像証拠がなくても完結する(映像欠如は
  ブロッカーではなく validity)

## Risks and Trade-offs

- 粗い輝度/差分ではモーション検出が粗い → 「coarse」特徴として
  明示し、精緻検出は将来の高コスト解析に委ねる
- 最近傍照合は誤対応しうる → 対応確度(距離・窓占有率)を記録し、
  一意性のない対応は confidence を下げる
- `al-media` で特徴を出すと ingest が「分析っぽい」処理を持つ
  → 輝度/差分のような decode 直結の粗い観測に限定し、意味的な
  照合は `al-analysis` に置くことで責務を分離する

## Deferred Design Concerns

- 圧縮コーデック対応 | 外部依存の要否が未決 | MP4/コーデック
  方針の決定時 | 実用キャプチャでの映像特徴欠如が続く
- ユーザー提供の映像マーカー入力形式 | 需要未確認 | 要求発生時 |
  手動マーカーとの照合ができない。照合機構そのものは
  タイムスタンプ付きイベント系列なら入力を選ばないため、
  保留するのはマーカーのファイル形式だけである

## Verification Notes

- [ ] 非圧縮 AVI のシーンチェンジ・輝度・モーション系列が
  タイムスタンプ付きで記録される
- [ ] 音声イベントと映像イベントの差分・分布・ドリフトが記録される
- [ ] 映像がなくても音声解析が完結する
