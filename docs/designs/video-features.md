---
id: "DESIGN-034AasyGL3izivzyzZjS0j"
type: "design"
status: "approved"
title: "映像フレーム特徴と A/V 同期設計"
created: "2026-10-04"
updated: "2026-10-04"
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
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/video-av-timing.md"
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
  bottom-up / top-down)
- フレーム単位の粗い特徴: 平均輝度系列・frame-to-frame 変化量
  (モーション/アクティビティ)・シーンチェンジ候補タイムスタンプ
- 映像イベント時刻系列の永続化(タイムスタンプはストリーム
  クロック由来、公称 FPS 演算に依らない)
- 音声オンセット/イベントと映像イベント時刻の汎用照合
  (差分・確度・分布・ドリフト傾き)
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
  輝度・変化量・シーンチェンジ候補フラグ)を既存
  `StreamTiming` / `TimingObservation` と同じクロックで永続化
- `al-analysis::avsync` コンポーネント: ingest の映像イベント
  時刻系列と cheap の音声オンセット(`trans.onset_times`)を
  照合する。音声側・映像側イベント系列がそろわない場合は
  該当レコードを validity で明示する
- 境界: `al-media` はコンテナとフレームだけを知り、音声解析を
  知らない。`al-analysis` は ingest 成果物と cheap 特徴だけを
  読み、コンテナを直接読まない

## Data and Control Flow

- ingest: `vids` が非圧縮ならフレームごとに輝度・差分を算出し
  `video_features` として永続化(圧縮なら decode 不可を記録)
- cheap: 音声オンセット系列を既存どおり算出
- avsync(cheap stage 内): 各音声イベントに対し設定可能な探索窓内
  の最近傍映像イベントを対応付け、ペアごとに
  `avsync.audio_time` / `avsync.video_time` / `avsync.delta` /
  confidence を記録。系列が複数ある場合は対応ペア列から
  median/MAD と線形フィットによる drift 傾きをサマリ化
- すべて秒単位のストリームクロック時刻で扱い、nominal FPS を
  時刻計算に使わない

## Non-Functional Constraints

- フレームデコードはストリーミング(全フレームをメモリに持たない)
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
  手動マーカーとの照合ができない

## Verification Notes

- [ ] 非圧縮 AVI のシーンチェンジ・輝度・モーション系列が
  タイムスタンプ付きで記録される
- [ ] 音声イベントと映像イベントの差分・分布・ドリフトが記録される
- [ ] 映像がなくても音声解析が完結する
