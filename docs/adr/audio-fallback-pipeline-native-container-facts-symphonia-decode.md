---
id: "ADR-034Gs0lgFjV7eNdvRdduFg"
type: "adr"
status: "accepted"
title: "audio fallback pipeline: native container facts + symphonia decode"
created: "2026-10-15"
updated: "2026-10-15"
owners: []
relations:
  source:
    - "docs/discovery/symphonia-aac-lc-decoder-backend.md"
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
  refined-by:
    - "ruviform-foundation-migration.md"
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# audio fallback pipeline: native container facts + symphonia decode

## Context and Problem Statement

`DecoderRegistry` は現在 `resolve` で detect した最初の backend を
1つだけ選び、その probe/decode だけを使う単一解決。native は
WAV/AVI/MP4 全てを detect するため、symphonia を `register` しても
MP4 入力には到達しない。

しかし MP4 の `soun` track で PCM ではない codec(AAC 等)は native が
`Unavailable::capability` を返し、`IngestResult.audio` が `None` に
なる。Issue #36 はこの穴を symphonia で埋めることを確定済みで、
「native の container/timing 観測はそのまま、音声だけを symphonia
で補完する」という部分統合が求められている。

## Decision Drivers

* `IngestResult` は1入力あたり1つしかないため、container facts と
  audio の供給元を分離する責務境界が要る。
* `DecoderProvenance.attempts`/`fallback`/`fallback_codec` は
  既存フィールドであり、複数 backend 試行の記録形式は決まっている。
* native の `mp4` 抽出(track/timing/PCM bytes)は既に正確であり、
  symphonia 側で重複実装すべきではない。
* Stage 契約と manifest schema は変更しない(artifact 契約の既存性)。

## Considered Options

* A. registry に audio fallback 合成を追加: resolve した backend が
  音声を返さない場合に限り、残りの登録 backend を detect 順に
  試行し、`audio` のみを採用して merge する。
* B. symphonia を MP4 全体の backend にする: probe/timing/mp4 facts
  を symphonia 側で再実装する。
* C. 外部プロセス(ffmpeg)で音声を decode する subprocess backend。

## Decision Outcome

Chosen option: "A — registry に audio fallback 合成を追加", because
native の container 観測(track/timing/mp4 facts/PCM track)を正本として
保持しつつ、symphonia は音声 decode にだけ責務を限定でき、既存の
attempts/fallback/fallback_codec フィールドで複数 backend 試行を
記録できるため。

### Consequences

* Good, because native の partial-ingest 契約(integrity observation、
  `metadata.json` スキーマ)を変更せず、AAC のみ追加される。
* Good, because `IngestResult` の供給元が「container 観測=native、
  音声=native → symphonia fallback」という明確な境界になる。
* Bad, because `DecoderRegistry::decode` に条件分岐と merge ロジック
  (streams entry の decoded 更新)が増える。
* Bad, because symphonia 側も `MediaDecoder` trait に適合させるため
  `probe`/`decode`/`detect` を実装する必要がある(detect は MP4 のみ)。

### Confirmation

* AAC-LC in MP4 fixture で `decoded.f64le` が生成され、
  `metadata.json.audio.codec == "aac"` となること。
* PCM track だけの MP4 では attempts が1件(native のみ)のままであること。
* AAC 不能な MP4 では attempts が2件(native + symphonia 失敗)で
  `fallback == false` のままであること。

## Implementation Plan

* Affected paths: `crates/al-media/src/decoder.rs`(registry の
  fallback 合成)、`crates/al-media/src/symphonia.rs`(新規 backend)、
  `crates/al-media/src/stage.rs`(registry 構成)、
  `crates/al-media/Cargo.toml`(symphonia 依存)、
  `crates/al-testkit/`(AAC fixture)。
* Patterns to follow: `MediaDecoder` trait(detect/probe/decode)、
  `BackendAttempt`/`Unavailable`/`IntegrityObservation` 記録、
  `StreamResult` の stream 単位の decoded/unavailable。
* Patterns to avoid: symphonia による container facts の再実装、
  `DecodedAudio` の stage 間直接受け渡し、metadata.json スキーマの
  非互換変更。
* Dependencies or configuration: `symphonia`(承認済み、pin =
  `0.5.5` 系、features は AAC/MP4 のみに絞る)。

## Verification

* [ ] `cargo test -p al-media` の fixture ベーステスト(AAC-LC MP4
      正常 decode・PCM track は native のみ・破損/非 AAC-LC は
      失敗しない・複数 track 選択)がパスする。
* [ ] `cargo test --workspace` がパスする。
* [ ] `cargo clippy --workspace --all-targets -- -D warnings` が
      clean。

## Pros and Cons of the Options

### Option A(registry audio fallback)

* Good, because 責務分割が明確(container facts=native、音声=
  native→symphonia)で、既存の provenance 記録に自然に乗る。
* Good, because IngestStage の変更は registry 構成のみで済む。
* Bad, because fallback merge(streams entry の更新、attempts 記録)
  の実装が少し複雑になる。

### Option B(symphonia が MP4 全体を handle)

* Good, because MP4 の demux/codec を symphonia 一箇所に集約できる。
* Bad, because native の mp4.rs の PCM 抽出・timing・integrity
  observation を symphonia 側で再実装する必要があり、観測値が
  二重化して drift する。

### Option C(ffmpeg subprocess backend)

* Good, because 将来の H.264 など映像 codec も1つの backend で
  カバーできる。
* Bad, because 外部バイナリ依存・配布・セキュリティ境界・
  プロセス管理の複雑さがあり、確定方針では次フェーズ扱い。

## More Information

- `docs/discovery/symphonia-aac-lc-decoder-backend.md`
- `docs/specs/symphonia-aac-lc-audio-decode-in-mp4.md`
- Issue #36

Review notes (non-blocking): `metadata.json.audio` の
sample_rate/channels は `DecodedAudio` 自身の値を正本とする
(native probe の観測値と乖離する場合、例えば将来の SBR は、
DecodedAudio 側が真値)。`provenance.backend` は container 正本の
"native" を維持し、音声の供給元は `attempts`/`fallback` で表現する。
container が `Malformed` の場合は fallback を試行しない
(corruption は capability gap ではない)。
