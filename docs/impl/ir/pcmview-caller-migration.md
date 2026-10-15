---
id: "IMPL-034G6svuSB1SI6xwuxaUNX"
type: "impl"
status: "completed"
title: "decode_input/read_normalized_record の PcmView 移行"
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
  implements:
    - "docs/tasks/decode-callers-view-migration.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# decode_input/read_normalized_record の PcmView 移行

## Summary

`decode_input` と `read_normalized_record` の
`fs::read` → interleaved `Vec<f64>` 構築 → `dsp::deinterleave` 経路を
`PcmView::open` + `view.deinterleave(channels, scale)` に移行し、
中間 interleaved `Vec<f64>` の確保を除去した。基盤は
`docs/impl/ir/pcmview-mmap-fallback-deinterleave.md` を参照。

## Context

`docs/tasks/decode-callers-view-migration.md`(Slice 2)を実施。
`docs/designs/decoded-pcm-artifact-view.md` の要求 —
`DecodedInput` / `PreparedInput` の出力を既存経路と bit-identical に
保ち、manifest・lineage・path・検証・`Option` 挙動を維持する — に従う。

## Implementation

### Changes

- `decode_input`: `fs::read` + `as_chunks::<8>` collect を
  `PcmView::open` に置き換え。`len % 8 != 0` のエラー文言は
  `view.byte_len()` で完全に維持し、`view.deinterleave(channels,
  scale)` で融合 materialize。
- `read_normalized_record`: 同様に `view.deinterleave(
  params.channels, 1.0)` に置き換え。byte length 検証
  (`frames * channels * 8` と厳密一致)と有限性検証を維持。
- `dsp::deinterleave` 自体は変更せず(residual 等の他 caller が
  依存する oracle として残置)。

### Decisions During Implementation

- 有限性検証は従来 interleaved `Vec<f64>` 上で行っていたが、
  deinterleave 後の `channel_samples.iter().flatten()` に移した。
  byte length 検証を先に通るため部分フレームは存在せず、検査対象の
  値集合は同値であり、検出する非有限値は同一。
- `PcmView` が caller 無しでは dead_code となるため、基盤と本移行を
  同一コミットでリリースし、IR のみ task 単位で分割した。

## Related Experiments

### Adopted

(なし)

### Rejected

(なし)

## Validation

- `cargo test -p al-locate decode::tests`: 6 件パス
  (oracle 等価・scale 1.0/128.0/32768.0・mono/stereo/3ch/6ch・
  部分フレーム drop・`channels == 0`・空ファイル Owned fallback・
  mmap/Owned 等価・missing file エラー伝播)
- `cargo test -p al-locate -p al-analysis`: 全件パス
  (al-locate 53 + golden 16 + locate 12 + prepared 5 +
  argmax_tie 1、al-analysis 99 + 各 integration。
  `normalized_pcm_f64le_bytes_are_exact`・
  `normalized_pcm_reuse_layout_coordinates_and_fail_closed_loading`・
  `residual_child_reopens_the_exact_subtracted_signal_and_lineage`
  を含む)
- `cargo clippy -p al-locate --all-targets -- -D warnings`: クリーン
- `cargo fmt --all -- --check`: パス

## Risks

- `decode_input` の mmap 失敗時は Owned fallback で同一結果・同一
  エラーを返すため、環境差による出力差は生じない。エラー時の
  文言は `byte_len()` 経由で維持済み。

## Follow-ups

- `docs/impl/ir/pcmview-measurement-cache-decision.md`:
  3経路の計測と job cache 導入可否の判定。
