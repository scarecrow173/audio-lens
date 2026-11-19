---
id: "EVID-9mNf4kQpRwTsHv2zXcDe"
type: "evidence"
status: "approved"
title: "検証記録: メディア取り込み・時刻情報・AV 同期"
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

# 検証記録: メディア取り込み・時刻情報・AV 同期

`al-media` のデコード・タイミング・AV 同期系に関する検証の事実記録。

## WAV デコードと厳密構造検査

- `cargo test -p al-media`: WAV デコード検証で 8-bit unsigned offset(`raw − 128`)・24-bit sign extension・PCM s16/s24/s32/float32/64 の変換を固定。
- 構造拒否: RIFF サイズ不整合・block alignment/byte rate 不一致・ch/rate 0・奇数バイト padding 欠落・data 末尾余剰バイト・`fmt `/`data` チャンク重複・フレーム境界未満の data chunk をそれぞれエラーとして検証。
- 正規化検証: 整数 PCM は `[-1,1]` 正規化せず `sample_format` でスケール保持する契約を、metadata round trip と scale 適用経路で確認。

## AAC/MP4 fixture 検証

- ffmpeg 8.1.2 で生成した静的 fixture で `cargo test -p al-media --test aac` pass。stereo/mono/2 オーディオトラック/PCM MP4/無ペイロード `mp4a` の 5 ケースで、bit-exact でなくメタデータ・フレーム数・デコーダ試行・フォールバック・アーティファクト生成を検証。
- `cargo clippy --workspace --all-targets -- -D warnings`・`cargo fmt --all -- --check`・`cargo test --workspace` 全 pass。

## AVI/タイミング観測

- AVI 構造解析で `strh` ヘッダの `rcFrame`・`HASINDEX`/`idx1` 整合性・DIB stride 揃え・stream index 範囲を検証。実機 AVI で luma 系列抽出が成立することを `real_avi_metadata_produces_stream_time_views` 等で確認。
- `TimestampSource`/`SequenceTimestamps` で CFR 非仮定の観測(フレーム間隔・ppm ドリフト・不連続・重複/ドロップ候補・クロック種別)を検証。local median による孤立ギャップ/持続 VFR の区別をテスト。

## AV 同期・ユーザーマーカー

- `cargo test -p al-analysis --test avsync`: 7 件 pass で `VideoEventS` 経由の matching・`clock=video_stream` 前提・`dwStart`/stream origin 正規化を固定。
- `analysis.avsync.markers` から `VideoEventS` への変換・通常 matching 合流・`user_marker` スキーマ・config/lineage の params 永続化を検証。ターゲットテストは pass、workspace 全体は本変より前のスライスで pass。

## 部分 ingest への契約移行検証

- 構造破損は stage 全体 fail-fast を維持しつつ、局所 PCM 破損を明示的に扱う経路を回帰で固定。config パース失敗は silent `{}` ではなく明示エラー、辞書は job config スナップショットで resume 不変を検証。

## SHA-256 入力 pin

- `JobDefinition::create` で `source_sha256` を manifest `declared_input` params に記録することを確認。resume/再実行時に一致検証し、struct literal 迂回でも runner 入口で必須性を検査する経路をテスト。

## 未検証・制約

- 実 AAC ペイロードの bit-exact デコード検証は行わず、メタデータ・フレーム構造・フォールバック挙動を検証した。
- 実コンテナ規模の性能・ストリーム境界挙動は fixture ベースで、全 real-world AVI/MP4 の網羅ではない。
- `dig.quant_step_est` は高ビット深度で感度低下(別 record に記録)。
