---
id: "IMPL-034ztWIx7Y7qsBZtBOnv43"
type: "impl"
status: "completed"
title: "avi container ingest and video fixtures"
created: "2026-10-03"
updated: "2026-10-03"
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
    - "../../designs/ingest-av.md"
    - "../../specs/media-ingest.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# avi container ingest and video fixtures

## Summary

動画メインの解析ユースケース向けに、レガシー AVI(RIFF 'AVI ')
コンテナのデマックスを `al-media` に自前実装し、決定的な合成
動画フィクスチャ(バグ動画 / 正常動画 / 非同一リプレイ動画 +
元音源)を `samples/` に追加した。

## Context

design `docs/designs/ingest-av.md` の Deferred Concern
「対応コンテナ / コーデックの初期セット」を実装フェーズで解決。
spec `media-ingest.md` の方針(未対応形式は `decode/input
unavailable` で明示失敗)に従い、依存ライブラリを追加せず
レガシー AVI のみを対象とした。

## Implementation

### Changes

- `crates/al-media/src/avi.rs`:
  - `decode_avi`: RIFF/AVI チャンク走査 → `hdrl`/`strl` の
    `strh`+`strf` を解析し、最初の `auds` ストリームの PCM を
    `decode_waveformatex` でデコード、最初の `vids` ストリームの
    宣言メタデータ(BITMAPINFOHEADER の codec/解像度/bpp +
    strh の frames/rate/scale)を `VideoInfo` として返す。
    `movi` LIST 内の `NNwb`(音声)/`NNdb`,`NNdc`(映像)チャンクを
    ストリーム番号で仕分けする。`movi` 欠落や音声ストリームなしは
    明示失敗
  - `write_avi` + `AviSpec`: CFR 非圧縮映像 + PCM 音声の AVI を
    書き出すフィクスチャ用ライター(avih/strh/strf/movi を生成)
- `crates/al-media/src/wav.rs`: `parse_fmt` / `decode_samples` /
  `FmtChunk` を `pub(crate)` 化し、WAVEFORMATEX→PCM 一括変換の
  `decode_waveformatex` を追加(AVI `strf` と共通の構造のため再利用)
- `format.rs`: `MediaFormat::Avi` と `RIFF....AVI ` シグネチャ検出
- `stage.rs`: ingest のデコードを `ingest_decoded` に共通化。
  `metadata.json` に `"video"` セクションを追加(VideoInfo、
  音声のみの入力では null)。container 文字列は `"wav"`/`"avi"`
- `crates/al-cli/examples/gen_samples.rs`: `to_s16le` /
  `video_frame` / `write_avi_file` / `bgm` / `gameplay_audio`
  を追加し、動画 3 本と元音源 1 本を生成
- `samples/`: `source-bgm.wav` + `video-gameplay-{normal,bug,run2}.avi`
- `.gitattributes`: 動画拡張子(mp4/mov/mkv/avi/webm/m2ts)を
  拡張子単位で LFS 管理に変更
- `tests/ingest.rs`: AVI ingest の結合テスト 2 件追加

### Decisions During Implementation

- **映像フレームはデコードしない**。ingest はコンテナ宣言値
  (codec fourcc・解像度・フレーム数・公称 fps)のみを記録し、
  映像内容解析は後段の責務とする(design の av-timing 責務分離に
  一致)
- **音声ストリームは `decode_waveformatex` に集約**。AVI の `strf`
  は WAV `fmt ` と同一構造のため、表現ドリフトを避けて変換処理を
  一箇所に保つ
- **ストリーム選択は先頭のみ**。`auds`/`vids` が複数ある場合は最初の
  1 ストリームを使う。fixture 用途では十分で、曖昧さを増やさない
- **圧縮コーデックは未対応のまま明示失敗**。`auds` が PCM/
  IEEE-float 以外なら `decode_waveformatex` が拒否し、MP4/H.264 等は
  シグネチャ検出の段階で `decode/input unavailable` になる
- **run2 の非同一性は音声側で作る**。BGM のオフセット・ゲイン・
  テンポ(約 1% の線形補間リサンプル)・ノイズ床・映像掃引速度を
  すべて変えて、locate がコンセンサス不成立で reject するケースを
  作った

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-media`: 43 tests pass(avi.rs 3 + ingest.rs 2 追加)
- `cargo clippy -p al-media --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean
- E2E(`samples/_runs/`):
  - `video-bug-vs-normal`: bug AVI から PCM を取り出して全ステージ
    完走。locate 受理(start=19198≈真値 19200)、bug 動画で
    `clipping` / `silence_dropout_candidate` 等を検出
  - `video-nonidentical`: bug 動画は受理・run2 は low confidence
    reject(非同一テイクにアライメントを強制しない)
  - `video-bug-only`: capture 単独でもイベント検出
- `metadata.json` の `video` セクションに codec/rawvideo・128×96・
  30 frames・10 fps が記録されることを確認

## Risks

- マルチ `auds`/`vids` ストリームや idx1 の PTS は扱わない
  (fixture 範囲外)
- 非圧縮 BGR24 のみを想定したライターのため、実機キャプチャ
  (H.264/AAC)は引き続き未対応

## Follow-ups

- MP4/圧縮コーデックの対応を行う場合は別 plan で扱う
- 映像フレームの内容解析(イベント検出器)は後段タスク
