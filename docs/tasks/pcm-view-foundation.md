---
id: "TASK-034G5V4oS9KJDKepLo5oND"
type: "task"
status: "todo"
title: "PcmView 基盤(mmap/fallback + 融合 deinterleave)"
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

# PcmView 基盤(mmap/fallback + 融合 deinterleave)

## Work

1. `crates/al-locate/Cargo.toml` に memmap2 を追加(crates.io 公開7日超
   の安定版を pin、承認済み)。
2. `crates/al-locate/src/decode.rs` に `PcmBacking`(Mapped/Owned)と
   `pub(crate) PcmView`(open/byte_len/sample/deinterleave)を実装。
   empty・非通常ファイル・mmap 失敗で Owned fallback、I/O エラーは
   伝播。unsafe は `Mmap::map` のみ、artifact 不変性前提を comment。
3. `cfg(test)` モジュールに `dsp::deinterleave` oracle 等価テスト:
   scale=1.0/128.0/32768.0、partial frame drop(3ch 10sample)、
   channels==0、空ファイル、mono/stereo/多ch、強制 Owned
   (`PcmView::open_owned` 等の pub(crate) 構築)で mmap/Owned 同値。
4. fixture は temp dir に生成、新依存なし。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-decoded-pcm-artifact-view.md`(approved)
- `docs/designs/decoded-pcm-artifact-view.md`(approved、API 唯一の正)

## Dependencies

- なし(plan 直下の第1 slice)

## Affected Files

- `crates/al-locate/Cargo.toml`・`Cargo.lock`
- `crates/al-locate/src/decode.rs`

## Done When

- [ ] memmap2 が pin され workspace が build する。
- [ ] PcmView/融合 deinterleave が oracle 等価テスト全件 green。
- [ ] 強制 Owned と mmap で同一結果。境界(空・partial frame・ch0)。
- [ ] `cargo fmt --check`・`cargo clippy -p al-locate -- -D warnings` green。

## Verification

- [ ] `mise exec -- cargo test -p al-locate decode::tests`
- [ ] test-spec Rule 1/2/5 の該当範囲を満たす証拠を IR に記録。
