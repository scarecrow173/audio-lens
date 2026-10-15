---
id: "TASK-034G5V4SprTlEV2DE7NzIS"
type: "task"
status: "todo"
title: "decode_input/read_normalized_record の view 移行"
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
  implements:
    - "docs/plans/implement-decoded-pcm-artifact-view.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-decoded-pcm-artifact-view.md"
    - "docs/tasks/pcm-view-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/decoded-pcm-artifact-view.md"
  references: []
  defers: []
  deferred-by: []
---

# decode_input/read_normalized_record の view 移行

## Work

1. `decode_input` を `PcmView::open` + `view.deinterleave(channels, scale)`
   に置き換え、中間 `Vec<f64>` を除去。`len % 8` は
   `view.byte_len()` で現行と同一エラーメッセージを維持。
2. `read_normalized_record` を同様に移行(scale=1.0)。expected
   length 検証は `view.byte_len()`、有限性検証は materialize 後
   channel で同一エラーを返す。
3. manifest/lineage/path/metadata/scale/rate/channels の全検証は
   変更しない。`dsp::deinterleave` は residual/他 caller のため残す。
4. `cargo test -p al-locate -p al-analysis` 全 green
   (golden/locate/prepared/既存検証テスト)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-decoded-pcm-artifact-view.md`(approved)

## Dependencies

- `docs/tasks/pcm-view-foundation.md`

## Affected Files

- `crates/al-locate/src/decode.rs`

## Done When

- [ ] 2 reader が view 経由で中間 `Vec<f64>` を持たない。
- [ ] `len % 8`・expected length・有限性の全エラー経路が同一。
- [ ] `cargo test -p al-locate -p al-analysis` 変更なし green。

## Verification

- [ ] `mise exec -- cargo test -p al-locate -p al-analysis`
- [ ] test-spec Rule 1/3 の該当範囲を満たす証拠を IR に記録。
