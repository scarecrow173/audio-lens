---
id: "TSPEC-034G5V3XG7NaO7sBJ1UFew"
type: "test-spec"
status: "approved"
title: "decoded/normalized PCM read-only view の契約"
created: "2026-10-15"
updated: "2026-10-15"
owners: ["scarecrow173"]
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
    - "docs/designs/decoded-pcm-artifact-view.md"
  derived-by: []
  verifies:
    - "docs/specs/decoded-pcm-artifact-view.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# decoded/normalized PCM read-only view の契約

## Purpose

`PcmView`(mmap/Owned fallback)と融合 deinterleave、および
`decode_input`/`read_normalized_record` の view 移行が、現行
`fs::read` 経路と bit-identical で、既存の検証・エラー・Option 契約を
維持することを保証する。上流が変われば見直す。
2026-10-15 plan Judge ACCEPT の同義具体化。

## Feature

Artifact ID に紐づく read-only f64le PCM view による stage 読み出し。

## Rules

- Rule 1: view 経路の `channel_samples` は baseline(fs::read +
  collect + `dsp::deinterleave`)と bit-identical。
  Example: scale 1.0/非1.0、奇数サンプルの partial frame drop、
  channels==0、空ファイル、mono/stereo/多ch。
- Rule 2: 強制 Owned fallback と mmap で同一結果・同一エラー。
  Example: mmap 不能を強制して同じ出力を返す。
- Rule 3: `len % 8 != 0`・rate/channels 0・manifest/lineage/path
  検証は現行と同一エラー。`decode_input` の Option 契約を維持。
- Rule 4: 計測は fixture・環境・p50/p95・alloc・RSS を記録し、
  cache 導入可否の根拠を残す。未計測の性能 claim をしない。
- Rule 5: unsafe は `Mmap::map` のみ、artifact 不変性前提を code
  comment で明記する。

## Examples

plan の fixture/matrix どおり: scale 1.0/128.0/32768.0、3ch 10sample
(末尾1フレーム欠落)、empty f64le、len%8!=0、stereo/mono、
強制 Owned、36MB 計測 fixture。

## Guarantees

値・scale・deinterleave 順・完全フレームのみ処理・全エラー経路が
現行と同値。view は read-only で書き込み面を公開しない。resume は
manifest+artifact から独立復元可能(view/cache は永続正本でない)。

## Non-goals

deinterleave 済み cache、job cache 先行実装、部分読み取り API、
residual WAV、stage 間受け渡し、codec 再デコード、未計測性能主張。

## Risk

`dsp::deinterleave` との微細差(部分フレーム・channels==0)を
oracle 比較で防ぐ。mmap の利益が小さい場合は計測根拠を残し
per-call view で閉じることを失敗とみなさない。
