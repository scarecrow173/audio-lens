---
id: "DESIGN-034PdHI4tDTk3iOLaSJO54"
type: "design"
status: "approved"
title: "Ruviform media I/O ingest 移行(Issue #100)"
created: "2026-10-31"
updated: "2026-10-31"
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
    - "../adr/media-decoder-backend.md"
    - "../adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
    - "ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "ingest-av.md"
    - "mp4-ingest.md"
  refines:
    - "../adr/media-decoder-backend.md"
    - "../adr/audio-fallback-pipeline-native-container-facts-symphonia-decode.md"
  refined-by: []
  derives-from:
    - "../specs/media-ingest.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform media I/O ingest 移行(Issue #100)

## Context

Issue #100 は media ingest の container/codec I/O を `al-media` から
Ruviform の media-I/O backend(`ruviform-media-io` +
`ruviform-io-symphonia` + `ruviform-io-avi`)へ移す。依存は
`e467ccba8e1cf9d18982387827b61275b4bf3f4c`(Issue #151 観測契約 +
Issue #154 backend)に pin する。

責務境界は「decode は Ruviform、AudioLens 固有の観測契約と解析判定は
AudioLens」。`IngestResult`/`StreamProbe`/`StreamTiming`/`Mp4Facts`/
integrity observation/Manifest provenance の形は維持し、VFR・AV drift・
重複/欠落候補などの判定は Ruviform に移さない。

## Scope

### In Scope

- 切替ゲート 1(WAV)と 2(AVI/MP4+AAC): PCM/packet の decode を
  Ruviform backend へ移す
- `NativeAudioFrame` の非破壊(native scale f64)経路による元 PCM・
  bit depth・sample format の保持
- backend 名・Ruviform revision・選択規則・fallback 試行の
  provenance 記録
- MP4 `elst` media-time offset の適用(AAC priming 除去)
- AAC 実 fixture(`tests/fixtures/aac_lc_*.mp4`)による
  `ruviform-io-symphonia`(Symphonia 0.6)実 decode 検証 —
  Ruviform Issue #154 で fixture 範囲外とされた残件
- AudioLens 側の直接 `symphonia = "0.5.5"` 依存の除去
  (0.5.x/0.6.x の重複バージョン共存を残さない)

### Out of Scope

- ゲート 3(動画フレーム/タイミング・複数ストリーム)の移行。
  AVI 映像特徴は native DIB 経路を維持し、provenance に
  `video_path = "native_dib"` と記録する
- `wav.rs`/`avi.rs`/`mp4.rs` の汎用 parser 撤去(Issue #104 で
  追跡。詳細は Deferred Design Concerns)
- 解析アルゴリズム・Dataset 形式の刷新、legacy 出力の互換 reader
- VFR/AV drift 等の AudioLens 固有判定の Ruviform 側への移動

## Components and Boundaries

- `crates/al-media/src/ruviform_backend.rs` — `RuviformDecoder`
  (`MediaDecoder` impl、`id = "ruviform"`、`version` = pin した
  Ruviform revision)。`RuvDemux` enum が Symphonia/AVI demuxer を
  抱え、`Packet` feed → `make_native_decoder` / `make_decoder` →
  `Frame::NativeAudio` / `Frame::Audio` の収集を担う
- `DecoderRegistry::ruviform()` — ingest が使う registry。
  backend は `RuviformDecoder` 単独。旧 `SymphoniaDecoder` と
  AudioLens→symphonia 直接依存は削除
- `NativeDecoder` — probe/decode の事実観測側として残る。
  `RuviformDecoder::decode` は `request.audio = false` で
  `NativeDecoder::decode` を呼び、container facts・video・timing・
  映像特徴を従来どおり取得する
- `wav.rs`/`avi.rs`/`mp4.rs` — AudioLens の永続観測契約
  (skipped boxes・edit list・sync samples・per-chunk facts)を
  Ruviform の現行観測面(`StreamFacts`/`ContainerInfo`)がまだ
  提供しないため残置(#104)

## Data and Control Flow

- probe: `NativeDecoder::probe` で `StreamProbe` 群を構築し、
  Ruviform demuxer の `facts.decodable` で各 stream の `decode`
  フラグを更新する(AAC 等 native が decode 不能とした stream も
  Ruviform 経由で decodable になりうる)
- decode: container facts/timing/video は native 観測経路のまま
  (audio 要求を落とした `fact_request`)。音声は `ruviform_audio`
  が「最初の decodable audio stream」を選び、全 packet を集めて
  decoder へ送る
- PCM 系 codec(`native_format` が `CodecFormat::Audio`)は
  `make_native_decoder` で native scale f64 を得て、
  `SampleFormat` を integer 幅で写像する。正規化しない
- AAC 等 native PCM 形を持たない codec(`native_format` が
  `NotDeclared`)は `make_decoder` の canonical f32 frame 経路で
  decode し `SampleFormat::Float32` と記録する(旧 symphonia
  fallback と同じ表示契約)
- MP4: Ruviform 側は `elst` media_time を適用しないため、
  `Mp4Facts` の `edit_media_time_ticks`/`duration_ticks` を
  media timescale → sample rate で換算して adapter が先頭 trim +
  playable 長へ切り詰める
- `provenance.streams` は native の出力順契約を維持する: 選択
  audio → 選択 video → 未選択 stream(probe 順)。`integrity` の
  `decode_unavailable` も `soun` trak 順が video 観測より先に並ぶ
- 失敗の型分け: Ruviform `ErrorKind::Unsupported` →
  capability(unavailable)、`InvalidData`/`InvalidValue`/
  `TimeOverflow`/`UnexpectedEos` → malformed、`Io` → I/O 失敗。
  decode 不能は ingest 全体を失敗させず integrity/provenance に
  記録する
- 暫定比較経路: Ruviform decode が失敗した入力に限り native
  decoder を試行し、成功時は `settings.ruviform.audio_fallback`
  に理由を記録して採用する(#104 で撤去判断)

## Non-Functional Constraints

- 整数 PCM は元尺度のまま f64 interleaved で保持し、暗黙の
  f64→f32 縮約・`[-1,1]` 正規化を導入しない
- lossless に元 PCM/codec metadata を再現できない backend は
  `unsupported`(capability unavailable)とし、成功扱いしない
- Manifest/provenance に使用 Ruviform revision・backend 名・
  settings(`ruviform.rev`/`io`/`video_path`/`audio_fallback`)を
  残す
- headless 維持(`check-cli-headless` 対象に GUI 依存を入れない)

## Risks and Trade-offs

- Symphonia は `ruviform-io-symphonia` 内の 0.6.1 のみとし、
  AudioLens 側 0.5.5 直接依存は除去した(#99 で本 Issue への
  判断送りだった重複バージョン問題を解消)
- AAC は非可逆 codec のため canonical f32 経路を取る。native PCM
  契約(元尺度保持)は PCM 系 codec にのみ適用し、AAC は
  `Float32` 明示で誤った「lossless 成功」を報告しない
- facts 観測と payload decode の二重走査(native parser +
  Ruviform demux)は移行期の意図的コスト。恒久的な二重保守に
  しないため撤去境界を下記に固定する

## Deferred Design Concerns

- `wav.rs`/`avi.rs`/`mp4.rs` の汎用 parser 撤去(Issue #104):
  Ruviform の観測面が AudioLens 永続契約の facts(skipped box、
  elst、sync samples、chunk map)を提供次第置き換える
- ゲート 3: AVI 映像フレーム(`native_dib`)/複数ストリーム選択・
  raw PTS/DTS の `StreamTiming` 統合。Ruviform `Packet` の
  pts/dts/provenance は現行 `StreamTiming` モデルへ未接続
- Ruviform 側の観測面 gap(`StreamFacts` が未提供の box 詳細、
  native path での elst trim 非適用)は Ruviform Issue 起票案
  として報告する

## Verification Notes

- [x] WAV/AVI/MP4 synthetic fixture が `DecoderRegistry::ruviform()`
  経由で decode できる(`tests/aac.rs`・`tests/ingest.rs`・
  `tests/registry.rs`)
- [x] 実 AAC-LC bitstream(stereo/mono/two-track、ffmpeg 生成
  fixture)が Ruviform backend で decode され、elst trim 後の
  frame 数が一致(12000/12000/9600 @48kHz)
- [x] PCM 元尺度保持: `pcm_s16le` 等の codec 名・
  `SampleFormat::PcmS16` 等の写像・整数値完全一致を単体テストで
  確認
- [x] unavailable/corrupt/Io の区別と部分 ingest が保持される
- [x] `mise run fmt` / `check` / `lint` / `test` / `build` /
  `verify-toolchain` / `check-cli-headless` が pass
- [x] #98 baseline(`tools/baseline` の `small` run)比較:
  `decoded.f64le`・golden JSON・`artifacts.index` の payload 面は
  完全一致。残差分は `manifest.normalized.json` の decoder
  provenance(`backend`/`version`/`attempts` が `native`→
  `ruviform`、`settings.ruviform` 追加)のみで、Issue #100 の
  provenance 記録要件による意図的差分
- [ ] ゲート 3 後に variant baseline run との再比較(#104)
