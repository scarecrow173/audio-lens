---
id: "SPEC-034Gs0Ej9pXCBUDbXO8456"
type: "spec"
status: "approved"
title: "symphonia AAC-LC audio decode in MP4"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source:
    - "docs/discovery/symphonia-aac-lc-decoder-backend.md"
    - "https://github.com/scarecrow173/audio-lens/issues/36"
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
  related:
    - "../adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/symphonia-aac-lc-decoder-backend.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# symphonia AAC-LC audio decode in MP4

## What Should Be Built

MP4 コンテナ内の AAC-LC(`mp4a` sample entry、`esds` の
AudioSpecificConfig を持つ)音声トラックを、symphonia(純 Rust)で
実デコードし、既存の `decoded.f64le`/`metadata.json` artifact 契約で
`decoded_pcm` を生成する。

## Why Now

ゲームキャプチャの主要コンテナは MP4 で、音声はほぼ AAC-LC。
現状の native backend は PCM sample entry のみ抽出可能で、AAC 入力は
`audio` 節なしの metadata のみになってしまい、解析 pipeline の主対象が
 ingest 段階で落ちている。

## Users and Value

- 解析ツール利用者: MP4/AAC 入力をそのまま job に投入できる。
- 下流 stage(locate/analysis): `decoded_pcm` artifact 契約は
  変更なしで AAC 由来 PCM を取得できる。

## Scope

### In Scope

- `DecoderRegistry` の audio fallback: resolved backend(native)が
  音声を生成しなかった場合、登録済みの他 backend(detect 順)で
  音声のみ decode を試行する。
- `SymphoniaDecoder`: `MediaDecoder` trait 適合、MP4 の AAC-LC
  track decode → `DecodedAudio`(interleaved f64、`Float32`、
  `codec: "aac"`)。
- AAC priming/delay の除去(`Packet::trim` 由来の front/back)と
  先頭 PTS の観測保持。
- 複数 `soun` track がある場合の選択規則は native と同一
  (最初に decode 可能な track のみ)。
- provenance: `attempts` に backend ごとの試行結果、`fallback` フラグ、
  `fallback_codec`、stream 単位の `decoded`/`unavailable` を維持。
- MP4 の native 抽出(timing/`mp4` facts/PCM tracks)と symphonia
  音声の部分統合。

### Out of Scope

- 映像デコード(H.264/H.265 等)。`VideoThumbs`/`VideoFeatureArtifact`
  は引き続き Unavailable で理由を記録する。ffmpeg subprocess
  backend は次フェーズ。
- AAC 以外のコーデック(MP3 単体、FLAC、Opus、ALAC 等)や
  MP4 以外のコンテナでの symphonia 適用。
- 任意 stream 選択(`audio_stream`/`video_stream` の explicit
  selection は従来通り未提供)。
- decoded.f64le のフォーマット変更 / 部分読み取り。

## Requirements

- `detect` は MP4 のみ返す(他コンテナは native のみが handle)。
- native が音声を返した場合(PCM track)は symphonia を呼ばない。
- symphonia fallback が音声を返したとき:
  - `IngestResult.audio` に `DecodedAudio` が入る。
  - `provenance.attempts` に native と symphonia の2件が記録される。
  - `provenance.fallback == true`。
  - audio stream の `StreamResult.decoded == true`、
    `unavailable == None`。
  - `streams`/`timing`/`mp4`/`video`/`video_features`/`video_thumbs`
    は native の結果をそのまま保持(上書きしない)。
  - `fallback_codec` は native が観測した圧縮 codec 名を保持する。
- symphonia も音声を返せない場合: native の Unavailable/integrity
  契約を維持したまま、`attempts` に symphonia の失敗を追記し、
  `fallback` は立てない。
- `decode` の結果は bit-identical に deterministic(同一入力→同一
  `decoded.f64le`)。AAC デコード結果自体は symphonia の出力に従う。
- 既存の PCM/WAV/AVI 経路は変更しない。

## Acceptance Criteria

- [ ] AAC-LC in MP4 の fixture で `decoded.f64le` が生成され、
  `metadata.json.audio.codec == "aac"`・sample_rate/channels が
  source と一致する。
- [ ] AAC priming/delay が除去され、frames 数が trim 済みの
  期待値と一致する(既知 fixture で検証)。
- [ ] PCM track を持つ MP4 は従来通り native のみで decode される
  (attempts は1件、fallback=false)。
- [ ] AAC decode 不能(破損/非 AAC-LC)でも stage は失敗せず、
  audio なし metadata + integrity observation が残る。
- [ ] 複数 soun track の MP4 では最初の decode 可能 track のみが
  選択され、他 track は `unavailable` で記録される。
- [ ] `DecoderRegistry::decode` が audio なし MP4 で2件の attempt を
  provenance に記録する。
- [ ] `cargo test -p al-media`(fixture ベース)がパスし、既存の
  `metadata.json` schema が変わらない。

## Deferred Design Concerns

- H.264/H.265 映像デコード | 別フェーズ(ffmpeg subprocess backend) |
  音声 decode が安定した後 | MP4 映像の VideoThumbs が
  Unavailable のまま残る
- MP3/FLAC/Opus 等の他コーデック symphonia 対象化 | 本 Issue の
  スコープ外 | AAC-LC 経路が検証された後 | fallback が MP4/AAC に
  限定される

## Sources and Relations

- `docs/discovery/symphonia-aac-lc-decoder-backend.md`
- Issue #36
