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
  - チャンク走査は Result を返す `Chunks` イテレータに統一し、
    WAV パーサと同じ厳密さで検証する: RIFF 宣言サイズと実長の一致、
    各チャンクの `8+size+pad` 境界(切り詰め body・欠落 pad バイト・
    末尾ゴミはすべて `Malformed`)
  - `movi` パケットはストリーム番号付きで収集し、**最初の `auds`
    ストリームの `NNwb` のみ**を mux 順で連結する。他 `auds`
    ストリームのパケットは PCM に混入しない
  - `decode_waveformatex` の `InputUnavailable`(非 PCM/IEEE-float
    コーデック)は kind を変えずに伝播する(メッセージに AVI 文脈を
    付与)。ファイル破損(`Malformed`)と未対応コーデックを区別する
    ingest 契約を維持する
  - `StreamTiming`: レガシー AVI はパケット単位の PTS を持たない
    ため、各 `movi` パケットのストリーム時刻を mux 順の累積
    `strh` ユニット(映像フレームは 1 パケット = `dwScale` tick、
    `dwSampleSize` ストリームは `len*scale/sample_size` tick、
    `dwStart` を初期オフセット)から導出し、`crate::timing` の
    `observe_timing` で CFR 非依存の観測として記録する。
    `metadata.json` の `av_timing` セクションに永続化する
  - `write_avi` + `AviSpec`: CFR 非圧縮映像 + PCM 音声の AVI を
    書き出すフィクスチャ用ライター(avih/strh/strf/movi を生成)
- `crates/al-media/src/wav.rs`: `parse_fmt` / `decode_samples` /
  `FmtChunk` を `pub(crate)` 化し、WAVEFORMATEX→PCM 一括変換の
  `decode_waveformatex` を追加(AVI `strf` と共通の構造のため再利用)
- `format.rs`: `MediaFormat::Avi` と `RIFF....AVI ` シグネチャ検出
- `stage.rs`: ingest のデコードを `ingest_decoded` に共通化。
  `metadata.json` に `"video"` セクション(VideoInfo、
  音声のみの入力では null)と `"av_timing"` セクション
  (`StreamTiming` の配列、WAV 入力では空)を追加。
  container 文字列は `"wav"`/`"avi"`
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
  1 ストリームを使う。fixture 用途では十分で、曖昧さを増やさない。
  選択は `strl` 宣言順で解決し、選ばれた `auds` の `NNwb` パケット
  のみを mux 順で連結する(他ストリームの音声は観測されても
  PCM には混入しない)
- **パケットタイミングは mux 順由来の導出値**。レガシー AVI には
  明示的な PTS/DTS が存在しないため、タイムスタンプ系列は
  「`movi` 内のパケット出現順 + `strh` クロック」からの一次観測と
  して記録する。パケットの不規則なサイズ・宣言 `dwLength` と実
  フレーム数の差異はそのまま観測に現れるが、コンテナ内に刻まれた
  時刻そのものではない点に注意。`idx1` インデックスは解釈しない
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

- `cargo test -p al-media`: 58 tests pass(avi.rs 12 + ingest.rs 9 含む)
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

- `auds`/`vids` のデコード・メタデータ採用は先頭ストリームのみ
  (2 本目以降は `av_timing` で観測されるが音声 PCM 化しない)
- パケットタイミングは mux 順からの導出値であり、真の PTS では
  ない。idx1 / OpenDML (`AVIX`) / `rec ` リストは解釈しない
- 非圧縮 BGR24 のみを想定したライターのため、実機キャプチャ
  (H.264/AAC)は引き続き未対応

## Follow-ups

- MP4/圧縮コーデックの対応を行う場合は別 plan で扱う
- 映像フレームの内容解析(イベント検出器)は後段タスク
