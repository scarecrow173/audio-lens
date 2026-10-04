---
id: "ADR-034AapIjxqSGmopfqOgcJ5"
type: "adr"
status: "accepted"
title: "MP4 は demux とタイミング観測のみ対応しコーデック decode は行わない"
created: "2026-10-04"
updated: "2026-10-04"
owners: []
relations:
  source:
    - "../discovery/container-coverage-gaps.md"
    - "../adr/implementation-stack.md"

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
    - "../discovery/container-coverage-gaps.md"
    - "../adr/implementation-stack.md"

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

# MP4 は demux とタイミング観測のみ対応しコーデック decode は行わない

## Context and Problem Statement

issue #3 は「video capture files」の ingest を要求するが、実際の
ゲームキャプチャの主流は MP4/ISO-BMFF であり、現状は WAV と
非圧縮 AVI のみ対応。MP4 への対応範囲を決める必要がある。

## Decision Drivers

* 外部依存を増やさない(FFmpeg 等は大きな依存で承認が必要)
* 音声 PCM が取れなくてもタイミング/コンテナ観測は独立した価値を持つ
* 「対応不能は explicit failure」ポリシー

## Considered Options

* MP4 demux + タイミング観測のみ(コーデック decode は明示失敗)
* FFmpeg 依存でフルデコード
* MP4 非対応のまま

## Decision Outcome

Chosen option: "MP4 demux + タイミング観測のみ", because
純 Rust・決定的にコンテナ/トラック/タイミング観測を実装でき、
A/V タイミング観測(issue #11)を MP4 入力でも成立させられるため。
音声 decode は `decode/input unavailable` で明示失敗とし、
外部デコーダ導入は将来の別決定とする。

### Consequences

* Good, because 実用キャプチャのタイミング観測とメタデータ記録が
  可能になり、失敗が常に明示的。
* Bad, because MP4 の音声解析本体は引き続き不可。

### Confirmation

MP4 fixture でトラック/タイミング観測が記録され、コーデック
ストリームの decode 要求は明示失敗することをテストで確認する。

## Implementation Plan

* Affected paths: `crates/al-media`(mp4 demux 追加)
* Patterns to follow: `avi.rs` の demux/`StreamTiming`、
  `timing.rs` の観測モデル、`format.rs` の明示失敗
* Patterns to avoid: 公称 FPS・timescale 未考慮の時刻推定
* Dependencies or configuration: 新規依存なし

## Verification

* [ ] MP4 のトラック/タイミングが `TimingObservation` に整合して記録される
* [ ] 音声 decode 要求は `decode/input unavailable` で明示失敗

## Pros and Cons of the Options

### demux + タイミングのみ

* Good, because 依存なし・決定的・issue #3 の観測要求の大半を満たす。
* Bad, because 音声解析は依然不可。

### 外部デコーダでフルデコード

* Good, because 完全な実用性。
* Bad, because 大きな依存・再現性リスク。

### 非対応

* Good, because 変更不要。
* Bad, because 実用キャプチャを ingest できない。

## More Information

* GitHub issue #3
* `docs/discovery/container-coverage-gaps.md`
* `crates/al-media/src/format.rs`
