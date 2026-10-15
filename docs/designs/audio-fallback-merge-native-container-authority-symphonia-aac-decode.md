---
id: "DESIGN-034Gs3MhX5g3FwTsySzAiT"
type: "design"
status: "approved"
title: "audio fallback merge: native container authority + symphonia AAC decode"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source:
    - "docs/adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
    - "docs/specs/symphonia-aac-lc-audio-decode-in-mp4.md"
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
    - "docs/adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# audio fallback merge: native container authority + symphonia AAC decode

## Context

`DecoderRegistry::decode` は単一 backend 解決 + `IngestResult` 構築 +
provenance 記録を担う。ADR で「resolve した backend が音声を生成
しない場合に限り、残り backend を detect 順に試行し `audio` のみ
merge する」が確定済み。ここではその合成経路と `SymphoniaDecoder`
の内部構造を固定する。

## Scope

### In Scope

- `DecoderRegistry::decode` の audio fallback 分岐と merge。
- `SymphoniaDecoder`(新規 `symphonia.rs`): MP4 検出、最初の
  decode 可能な AAC-LC soun track の選択、packet 列の decode、
  trim(priming/delay)適用、`DecodedAudio` 生成。
- `IngestStage` 側の registry 構成(`DecoderRegistry::native()` に
  `register(SymphoniaDecoder)`)。
- AAC fixture(ffmpeg で生成・静的コミット)に基づくテスト。

### Out of Scope

- 映像デコード、他コンテナ/他コーデック、任意 stream 選択、
  metadata schema の変更。

## Components and Boundaries

- `DecoderRegistry`(decoder.rs): fallback 合成の唯一の所有者。
  「resolved backend が音声を生成しない」を判定し、merge 規則と
  attempts/fallback 記録を担う。backend 自身は互いを知らない。
- `SymphoniaDecoder`(symphonia.rs): `MediaDecoder` trait 実装。
  - `detect`: `detect_format(source) == Some(MediaFormat::Mp4)`
    のみ(symphonia の demuxer 選択でなく既存 format 判定に倣う)。
  - `probe`: 最小構成 — native probe を置き換えない。`MediaProbe`
    を返す必要はあるが fallback 経路では registry 側が native probe
    を使うため、symphonia probe は `default_track`/`tracks` の
    stream 列挙に限定する(simple probe)。
  - `decode`: symphonia で packet を読み、AAC-LC track を選択して
    decode し、`IngestResult{audio, provenance.streams}` を返す。
    track の選択は track order で最初の decoder 生成可能な track
    (AAC のみを対象)。symphonia の `track.id` は MP4 の trak 順
    index(tkhd ではない。0.5.5 で実測確認)であり、
    `StreamResult.stream` にはこれを記録する。
- merge(registry): fallback 結果の `audio` を採用する際、
  `result.mp4.facts.tracks` で fallback の track ID を native
  track へ逆引きする — tkhd `track_id` 一致を優先、なければ
  moov 順 `track_index` 一致(いずれも `handler == "soun"` 必須)。
  対応不能なら棄却し、symphonia の失敗として記録する。
  採用時は `provenance.streams` の該当 audio entry を
  `decoded: true, unavailable: None` に更新し、AAC priming を
  除いた `DecodedAudio` を `result.audio` に設定する。
- `Mp4Track.edit_media_time_ticks`(native、mp4.rs): `elst` の
  最初の非空 entry の `media_time`(media timescale ticks)を記録
  する新規フィールド。AAC encoder delay は symphonia/isomp4 が
  `trim_start`/`delay` に反映しないため(0.5.5 実測)、container
  fact を native が記録し、merge 時に `media_time` フレームを
  先頭から除去 + `duration_ticks - media_time` に truncate して
  priming/padding を除去する。単位は media timescale →
  sample_rate で換算する。

## Data and Control Flow

1. `DecoderRegistry::decode`: resolve → probe(native)→ decode。
   `request.audio && result.audio.is_none()` なら fallback 分岐へ。
2. 残りの登録 backend を `detect` 順に走査。detect しないものは
   skip(attempts にも記録しない — 試行していないため)。
   fallback 対象 backend には `backend.probe(source)` +
   `backend.decode(source, &probe, request)` を通常通り呼び、
   **結果の採用は `audio` のみ** — merge 後の `result.streams`
   (probe facts)・`mp4`/`timing` は native の値を維持する
   (symphonia 側 probe は backend 内部整合のためだけに使う)。
3. symphonia `decode` 成功(音声あり): `result.audio` を設定、
   `provenance.fallback = true`、`attempts` に symphonia の成功を
   追記、該当 audio `StreamResult` を `decoded` に更新、
   `fallback_codec` は native が観測した codec 名のまま保持。
4. symphonia が失敗/音声なし: `attempts` に失敗(reason 付き)を
   追記、`fallback` は false のまま、native の Unavailable/
   integrity 契約を維持したまま返す。
5. `SymphoniaDecoder::decode` 内部:
   a. `Cursor` を `MediaSourceStream` 化し `probe().format()` で
      isomp4 demuxer を得る。
   b. `format.tracks()` を順に走査し `codec_params.codec ==
      CODEC_TYPE_AAC` の最初の track を選択、`make_decoder`。
   c. `format.next_packet()` ループで選択 track の packet を
      `decoder.decode`(`FormatOptions::enable_gapless` 有効;
      demuxer が `trim_start`/`trim_end` を載せた場合は decoder が
      除去する。isomp4 では現状 elst を packet trim に反映しない
      ため、AAC priming は merge 側の `edit_media_time_ticks` が
      担う — 上記 merge 項参照)。
   d. `DecodedAudioRef` を `SampleBuffer::<f64>` 経由で interleave、
      `DecodedAudio{channels, sample_rate, bits_per_sample: 32,
      sample_format: Float32, codec: "aac"}` に積算。
   e. 先頭 packet の pts と適用した trim(front/back 合計)を
      `provenance.settings["symphonia"]` に記録(監査用)。
6. 部分観測: track が見つからない / decoder 生成失敗 /
   packet エラーは `IngestResult{audio: None}` + stream 単位の
   `Unavailable` で返し、Err(fatal)にしない — container が
   `Malformed` で native probe 自体が落ちる場合のみ fatal。

## Non-Functional Constraints

- determinism: 同一入力 → 同一 `decoded.f64le`(trim/decode 順が
  packet 順で一意)。
- symphonia 依存は `default-features = false` + `aac`/`isomp4`
  のみ(他 codec/format を引き込まない)。gapless は feature では
  なく `FormatOptions::enable_gapless`(既定 true)で明示する。
  pin = `0.5.5`。
- 追加の runtime dependency / subprocess なし(純 Rust のみ)。
- `DecodedAudio.samples` は symphonia の f32→f64 変換値で、
  `[-1,1]` 正規化スケール(AAC の native decode 形式)。
  `sample_format = Float32`・`bits_per_sample = 32` とする。

## Risks and Trade-offs

- symphonia `track.id` と native `track_index`/`track_id` の対応:
  0.5.5 実測で `track.id` は moov 順 index(tkhd ではない)。
  merge は `track_id`(tkhd)一致を優先し、なければ `track_index`
  一致にフォールバックする。対応不能なら棄却して native の
  Unavailable を維持する(誤った stream を decoded と記録しない
  方が安全)。
- `elst` edit list: symphonia/isomp4 は elst を packet trim に
  反映しないため、AAC encoder delay は native が記録した
  `edit_media_time_ticks` で merge 時に除去する。native timing は
  sample table 由来で不変。trim 後の frames 数は fixture テストで
  検証する。
- symphonia の API 変更リスク: pin 版で固定し、trait 境界内に
  閉じ込める。

## Deferred Design Concerns

- H.264/H.265 映像デコード | ffmpeg subprocess backend(次フェーズ)|
  音声 AAC-LC が安定した後 | VideoThumbs が Unavailable のまま残る
- symphonia による他 codec(MP3/FLAC/Opus 等)・他コンテナ対象化 |
  本 Issue のスコープ外 | AAC-LC 経路の検証後 | fallback は MP4/AAC
  に限定される
- 任意 stream 選択(`audio_stream`/`video_stream`) | 既存の
  native phase 1 規則と同様に未提供 | ユーザー要件発生時 |
  複数音声 track は先頭のみ選択

## Verification Notes

- [ ] AAC-LC MP4 fixture で `decoded.f64le` が生成され、
  `metadata.json.audio.codec == "aac"`・sample_rate/channels が
  source と一致する。
- [ ] trim 適用後の frames 数が期待値と一致し、先頭 pts が
  settings に記録される。
- [ ] PCM track だけの MP4 では attempts が1件・fallback=false。
- [ ] AAC 不能(破損/非 AAC-LC)で attempts が2件・fallback=false、
  audio なしの metadata が出力される。
- [ ] `cargo test -p al-media` / `--workspace` / clippy が clean。
