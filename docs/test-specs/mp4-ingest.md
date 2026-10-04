---
id: "TSPEC-034AaE5e9xkwiiwSpmK0fa"
type: "test-spec"
status: "proposed"
title: "MP4 demux とタイミング観測の検証"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/mp4-ingest.md"
  derived-by: []
  verifies:
    - "docs/specs/media-ingest.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---


# MP4 demux とタイミング観測の検証

## Purpose

issue #3 の MP4 コンテナ観測が、コーデック decode なしで
トラック/タイミングを正しく記録することを保証する。

## Feature

MP4(ISO-BMFF)の demux とサンプルタイミング観測

## Rules

- Rule: トラック種別・timescale・codec fourcc が記録される
- Rule: stts/ctts 由来のサンプル時刻列が
  `TimingObservation` に整合して記録される
- Rule: VFR 的な時刻列が公称 FPS 演算に依らず観測される
- Rule: 圧縮コーデックの decode 要求は
  `decode/input unavailable` で明示失敗する
- Rule: 非圧縮 PCM サンプルエントリは PCM として取り出せる

## Examples

- Example: 既知 stts を持つ合成 MP4 → サンプル時刻列が回復される
- Example: ctts オフセットを持つ合成 MP4 → PTS が DTS+ctts に一致
- Example: `mp4a`(AAC)トラック → メタデータ記録 + decode は明示失敗

## Guarantees

- コンテナ観測が decode 失敗と分離して記録される
- 時刻計算に nominal FPS が使われない

## Non-goals

- 圧縮コーデックの decode 正しさ
- fMP4 の完全対応

## Risk

- タイミング解釈違いが A/V ズレとして誤観測される
