---
id: "DISC-034GrYP6nZKwj0gFCaknA0"
type: "discovery"
status: "active"
title: "symphonia AAC-LC decoder backend"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source:
    - "remaining-issues-roadmap.md"
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

# symphonia AAC-LC decoder backend

## Exploration Goal

Issue #36「主要コーデックの実デコード対応」の第1段。MP4 コンテナ内の
AAC-LC 音声トラックを純 Rust の symphonia で実デコードし、
`decoded_pcm` artifact を生成可能にする。現状、MP4 の `soun` track
は PCM sample entry(`sowt`/`twos`/`lpcm`/`in24`/`in32`/`fl64` 等)
のみ抽出でき、`mp4a`(AAC)は `Unavailable::capability` +
integrity observation として記録され PCM は生成されない。

## Key Issues and Assumptions

- 確定済み(2026-10-14 の roadmap セッションで user 承認):
  - symphonia(純 Rust)依存追加は承認済み。
  - 第1対象は AAC-LC in MP4。映像デコード(H.264 等)は対象外 —
    `VideoThumbs`/`VideoFeatureArtifact` は Unavailable 継続で理由明示。
  - 部分結果統合: symphonia が音声を復号しても native が取得した
    映像・timing 情報(probe の `timing`/`mp4`/`streams`)を保持する。
  - provenance / attempts / unavailable 契約を維持。
- コード実態(確認済み):
  - `DecoderRegistry::resolve` は detect した最初の backend を1つ選ぶ
    単一解決。native は WAV/AVI/MP4 全てを detect するため、
    symphonia を単に register しても MP4 では到達しない。
    → fallback 合成経路を registry 側に設計する必要がある。
  - `DecoderProvenance` には `attempts: Vec<BackendAttempt>` と
    `fallback: bool` が既に存在し、複数 backend 試行の記録が
    設計済み。`IngestResult::fallback_codec` も既存。
  - `decoder.rs` の `decode()` 末尾で stream ごとの
    `StreamResult`(decoded/unavailable)を構築する。symphonia で
    音声が得られた場合、その stream の `decoded: true` と
    `unavailable: None` を反映する必要がある。
  - `IngestStage::ingest_input` は `DecoderRegistry::native()` を
    生成している。pipeline 構成の変更点はここ。
  - `ingest_decoded` は `mux.audio` の有無だけで `decoded.f64le` と
    `metadata.json` の `audio` 節を書くため、fallback で音声が
    得られた場合の artifact 生成は自動的に従う。
- AAC 固有の正確性要件(確定済み): PCM 値・チャンネル・
  sample_rate・AAC priming/delay・PTS・複数トラック選択を検証する。
  symphonia は `Packet::trim()`(iTunSMPB/elst 由来)を提供する;
  priming/余白の適用責任をどこに置くか要設計。
- fixture 戦略: al-testkit の `write_mp4` は構造 writer であり、
  有効な AAC bitstream は自力生成できない。実 AAC-LC payload は
  ffmpeg(mise 管理下に存在)で一度生成し、小さな fixture として
  コミットする方針が妥当(検証用資産、実行時依存ではない)。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. registry に audio fallback 合成(native 優先 → 非 PCM 音声は symphonia) | native の container/timing/video 観測をそのまま保持。attempts/fallback 既存フィールドに乗る | registry の単一 resolve 規約に部分合成ロジックを追加 | yes |
| B. symphonia を container 全体の backend にする | MP4 probe を symphonia に統一できる | native の MP4 facts 抽出(mp4.rs)と重複・出力形状の一致が難しい | no |
| C. ffmpeg subprocess で全コーデック一括 | H.264 等も将来まとめて解決 | 外部バイナリ依存・配布・セキュリティ面の複雑性。確定方針は次フェーズ扱い | no(次フェーズ) |

## Tentative Conclusions and Hypotheses

- (confirmed) `SymphoniaDecoder` を `MediaDecoder` trait に適合させ、
  `DecoderRegistry` に登録。`DecoderRegistry::decode` で resolved
  backend の `audio` が得られず、かつ音声が要求された場合に、
  残りの backend を detect 順に試行する audio-fallback を追加する。
- (confirmed) merge の正本は native `IngestResult`(streams/timing/
  mp4/video)とし、symphonia 側からは `audio`(DecodedAudio)のみを
  受け取る。`provenance.fallback = true`、2件目の `BackendAttempt`、
  audio stream の `StreamResult.decoded` 更新を行う。
- (hypothesis) `DecoderRegistry::native()` を pipeline 版
  (native + symphonia)に置き換えるか、IngestStage が明示的に
  backend を登録する形にするかは design で確定する。
- (hypothesis) AAC の出力は `SampleFormat::Float32`(f32→f64 昇格)、
  `bits_per_sample: 32`、`codec: "aac"` とする。symphonia の
  AudioBuffer(f32)を interleaved f64 に変換する。
- (hypothesis) AAC priming/delay は `Packet::trim()` の
  trim_front/trim_back で処理する。edit list の扱いは実装で検証。

## Open Questions

- (design で確定) fallback 試行の条件: 「audio 未取得」で足すか、
  Unavailable kind が `Capability` の場合に限定するか。
- (design で確定) `DecoderRegistry::native()` の pipeline 化方法
  (with 系 builder / IngestStage 側登録)。
- (impl で検証) fixture の最小 AAC-LC MP4 生成手順(ffmpeg 一括 or
  testkit writer + 埋め込み payload)。

## Promotion Candidates

- [x] spec-doc needed: AAC-LC in MP4 音声 decode の外部契約
  (provenance/partial ingest/成果物)を固定する
- [x] adr-doc needed: 複数 backend の fallback 合成と
  「container 正本は native、音声のみ symphonia」という責務分割

## Sources and Relations

- Issue #36 / `docs/discovery/remaining-issues-roadmap.md` の確定決定
- 確認済みコード: `crates/al-media/src/decoder.rs`、
  `crates/al-media/src/mp4.rs`、`crates/al-media/src/stage.rs`、
  `crates/al-media/src/metadata.rs`、`crates/al-testkit/src/mp4.rs`
