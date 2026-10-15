---
id: "TSPEC-034Gs7WHAxB3WpFCH8fHIQ"
type: "test-spec"
status: "draft"
title: "symphonia AAC-LC audio decode fallback"
created: "2026-10-15"
updated: "2026-10-15"
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
  verifies:
    - "docs/specs/symphonia-aac-lc-audio-decode-in-mp4.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# symphonia AAC-LC audio decode fallback

## Purpose

MP4/AAC-LC 入力が ingest で `decoded.f64le` を生成し、native の
container/timing 観測と provenance 契約を損なわないことを固定する。
AAC 対応が別 backend で完結する(映像が変わる/他 codec が増える)まで有効。

## Feature

MP4 入力の音声 decode — native が音声を生成しない場合に限り
symphonia fallback で `audio` を生成する。

## Rules

- Rule 1: fallback は `request.audio && result.audio.is_none()` の
  ときだけ走る。native が音声を返した経路では symphonia は
  呼ばれない(attempts は1件)。
- Rule 2: fallback 成功時は `audio` のみ採用され、`streams`
  (probe facts)・`mp4`・`timing`・`video*` は native の値を保持する。
- Rule 3: fallback 成功時 `provenance.fallback == true`、attempts
  に symphonia の `executed: true, outcome: "decoded"` が記録され、
  merge 対象の audio `StreamResult` が `decoded: true,
  unavailable: None` になる。
- Rule 4: fallback が音声を返さない/失敗した場合は attempts に
  失敗が記録され `fallback` は false のまま、native の
  Unavailable/integrity 観測が維持され、stage は失敗しない。
- Rule 5: `DecodedAudio` は `Float32`・`codec == "aac"`・
  `channels > 0 && sample_rate > 0`・`samples.len()` が
  `channels` の倍数で、AAC priming/delay は trim 済み。
- Rule 6: 複数 `soun` track の選択は最初の decode 可能な AAC
  track のみで、他の audio track は `unavailable` で記録される。

## Examples

- AAC-LC stereo 48kHz MP4 fixture を `IngestStage` に投入 →
  `decoded.f64le` artifact が存在し `metadata.json.audio.codec ==
  "aac"`・`sample_rate == 48000`・`channels == 2`。
- PCM (`sowt`) soun track の MP4 → attempts は `native` のみ、
  `fallback == false`。
- 破損した MP4(ftyp のみで moov なし等)→ `audio` は None、
  `fallback == false`、attempts に symphonia の失敗が記録される。

## Guarantees

- 同一入力 → 同一 `decoded.f64le`(deterministic)。
- 非 AAC-LC(`mp4a` だが codec が別物 / SBR 等 decode 不能)は
  audio なし metadata + Unavailable で正常終了する。
- track_id→track_index 逆引きに失敗した fallback 結果は棄却され、
  誤った stream が decoded と記録されない。

## Non-goals

- AAC デコード値の bit-exact 検証(デコーダ出力は symphonia 依存;
  本 spec は「音声が生成され metadata/provenance が正しい」ことのみ)。
- 映像(H.264)、他コーデック、他コンテナ、任意 stream 選択。
- symphonia 内部の ADTS/他フォーマット経路。

## Risk

- これらの保証が落ちると、AAC 入力が静かに audio なしになるか、
  誤った provenance(decoded=true だが別 track)が記録されて
  下流解析の信頼を壊す。
