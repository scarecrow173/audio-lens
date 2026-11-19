---
id: "KNOW-pV9f2kLmWbHh8xQ7yRzT"
type: "knowledge"
status: "approved"
title: "メディア取り込みと時刻情報: WAV/AVI/MP4・SHA-256・タイミング観測"
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

# メディア取り込みと時刻情報

`al-media` クレートは、入力メディアのデコード・メタデータ・タイミング観測・ハッシュ化を担い、`IngestStage` を通じて `decoded.f64le` + `metadata.json` を出力する。本書は WAV/AVI/MP4 のデコード契約、SHA-256、時刻情報観測、および「部分 ingest」への契約進化をまとめる。

## WAV デコード契約

対応形式は PCM(u8/s16/s24/s32)と IEEE float32/64 で、出力は f64 へ変換する。重要な設計判断として、整数 PCM は `[-1,1]` へ正規化せず、ソースのスケールを `sample_format` メタデータで保持する —— NCC/PHAT はスケール不変で、絶対スケールが必要なのはスペクトル包絡と残差エネルギーだけであり、normalize は locate 側で一元化する。

8-bit PCM は unsigned offset-binary(`raw - 128`)で、24-bit はリトルエンディアン 3 バイトの明示 sign extension を行う。malformed 構造は厳密に拒否する: RIFF サイズ不整合、block alignment/byte rate 不一致、チャンネル・サンプルレート 0、奇数バイト チャンクの padding 欠落、data chunk 末尾の余剰バイト、`fmt ` または `data` チャンクの重複、フレーム境界未満の data chunk。padding 欠落は data 直前に限らず、RIFF 階層全体の整合性として検査する。

## コンテナ(AVI/MP4)とデコーダ委譲

AVI は RIFF 構造を直接解析する。`strh`(stream header)の `rcFrame` が frame rate/scale として扱われ、`HASINDEX`/`idx1` の整合性、DIB の stride 揃え、stream index 範囲を検証する。video 系(キャプチャ映像)は luma ROI/全フレーム輝度系列を抽出し `video_events` として解析層へ渡す。

AAC/MP4 は ffmpeg 静的フィクスチャで検証する方針を採用した。実行時に ffmpeg に依存しないよう、AAC-LC MP4 を `ffmpeg 8.1.2` で生成してテストフィクスチャとしてコミットした。フィクスチャは stereo/mono/2 オーディオトラック/PCM MP4/「`mp4a` 構造はあるが AAC ペイロードを持たない」ケースを網羅し、bit-exact ではなくメタデータ・フレーム数・デコーダ試行・フォールバック・アーティファクト生成を検証する。デコード可能な AAC は ruviform 側の symphonia 経路に委譲する。

## SHA-256 と入力ピン留め

SHA-256 は外部依存を増やさず `al-core::hash` に実装した。`JobDefinition::create` で入力パスを絶対化し、内容ハッシュ `source_sha256` を manifest の `declared_input` params に記録する。manifest だけで入力バイト列を特定可能にすることが目的で、Ingest 側は resume/再実行時にこのハッシュを再検証する。ハッシュが一致しない入力は実行前に拒否する —— 呼出し側が struct literal で `create` を迂回しても runner 入口で `source_sha256` 必須性を検査する。

## タイミング観測

`TimestampSource` と `SequenceTimestamps` 抽象で、コンテナ由来と合成の両経路からフレーム時刻を得る。`avsync` は CFR を仮定せず、フレーム間隔・ドリフト(ppm、nominal interval 指定時のみ)・不連続・重複/ドロップ候補・クロック種別を観測する。孤立した大ギャップと持続的 VFR 変化は local median ベースで区別し、全体統計に流されない。

`stream_time` 観測は `VideoEventS` レコードとして永続化され、AV 同期判定は `clock=video_stream` を必須とする。`dwStart` とストリーム原点 semantics は matching の前に正規化する。ユーザー供給マーカーは `analysis.avsync.markers` 経由で既存 `VideoEventS` に変換して通常 matching へ合流させる —— ゲーム固有検出器を入れず generic `user_marker` スキーマを維持する設計。

## 部分 ingest への契約進化

初期契約は「どの decode 失敗も stage 全体を失敗させる」だったが、メディア実態に合わせて部分 ingest へ進化した。構造的破損(コンテナ自体の不整合)は依然 fail-fast、しかし局所的 PCM 破損やコンテナ内の非音声ストリーム問題は明示的に扱い、取得可能な部分のみで ingest を成立させる。契約変更の所有権はタスク単位で明示し、消費側で二重実装しない規則とした。実装上の注意として、「失敗=stage 全体」から「失敗の局所化」への移行は、失敗を握り潰して「観測なし」に変換しないよう、失敗種別と範囲を manifest 側に記録する。

config パース失敗は silent `{}` fallback ではなく明示エラーにする。辞書設定は job config にスナップショットし、resume 時に元の辞書ファイルへ依存しない。

## 実装上の教訓

- `sample_format` 欠落を暗黙補完すると永続データが偽造される。`format_scale` でスケール決定は open 時に行い、欠落は `InvalidData` で失敗させる(pre-release 再生成前提)。
- `path.split_once('/')` による artifact role 分解は role が `/` を含む(`features/cheap`)と破壊される。manifest の `artifact.role` を正本とする。
- `audio.sample_rate`/`channels` は `audio` オブジェクトのネスト構造で、`source_reference` ではなく `source_asset` が正しい `InputRole` 名。
- 8-bit unsigned offset と 24-bit sign extension は音声データの変換として最も誤りやすい箇所で、テストが byte 境界・符号境界の両方を固定する。

関連 Issue/PR: #28(コンテナエッジケース)、#30(fixture fidelity)、#31(avsync マーカー)、#32(残差 PCM)、#37(サンプルレート正規化)、#40(InputCache 経路)、#97/#103/#105(service 層への移設)。
