---
id: "IMPL-034G5Xo5i1U2eFPNXIb2la"
type: "impl"
status: "completed"
title: "PcmView 基盤(mmap/fallback + 融合 deinterleave)"
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
    - "docs/tasks/pcm-view-foundation.md"
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

# PcmView 基盤(mmap/fallback + 融合 deinterleave)

## Summary

`decoded_pcm` / `normalized_pcm`(f64le interleaved)artifact を
read-only の `PcmView` で読む基盤を `crates/al-locate/src/decode.rs`
に実装した。mmap を第一候補とし、空ファイル・非 regular・
mmap 失敗時は `fs::read` ベースの owned buffer に fallback する。
caller 側の移行は
`docs/impl/ir/pcmview-caller-migration.md` を参照。

## Context

`docs/tasks/pcm-view-foundation.md`(Slice 1)を実施。
`docs/designs/decoded-pcm-artifact-view.md` と
`docs/adr/artifact-id-read-only-pcm-view-cache.md` の決定 —
永続正本は manifest の artifact、共有表現は Artifact ID に紐づく
read-only view、deinterleave 済み `Vec<Vec<f64>>` はキャッシュ
しない、Stage 契約(artifact ID のみの通信)を維持 — に従う。

## Implementation

### Changes

- `crates/al-locate/Cargo.toml`: `memmap2 = "0.9.11"` を追加
  (2026-06-22 公開、承認済み依存)。
- `decode.rs`: `PcmBacking`(`Mapped`/`Owned`)と
  `pub(crate) PcmView` を追加。`open` は非空 regular file の mmap
  を試み、失敗時は `open_owned`(`fs::read`)に fallback。
  唯一の unsafe は `Mmap::map` で、artifact は `write_durable` の
  rename 完了後に不変である前提を SAFETY コメントに記録した。
- `PcmView::deinterleave(channels, scale)` は per-sample の
  `f64::from_le_bytes / scale` と deinterleave を融合し、
  中間 interleaved `Vec<f64>` を生成しない。`chunks_exact` 相当の
  部分フレーム drop・`channels == 0` で空・チャンネル順序保持の
  逐語契約は `dsp::deinterleave` に揃えた。
- `cfg(test)` に oracle 等価・境界・fallback テストを追加
  (`PcmView` は `pub(crate)` のため integration test から不可視)。

### Decisions During Implementation

- 基盤だけでは non-test build が dead_code になるため、caller 移行
  (`decode-callers-view-migration`)と同一コミットでリリースした。
  IR は task 単位で分割し、caller 側の変更・判断は
  `pcmview-caller-migration.md` に記録する。

## Related Experiments

### Adopted

(なし — 計測は Slice 3 `pcm-view-measurement-cache-decision` で実施)

### Rejected

(なし)

## Validation

- `cargo test -p al-locate decode::tests`: 6 件パス
  (oracle 等価・scale 1.0/128.0/32768.0・mono/stereo/3ch/6ch・
  部分フレーム drop・`channels == 0`・空ファイル Owned fallback・
  mmap/Owned 等価・missing file エラー伝播)
- `cargo test -p al-locate -p al-analysis`: 全件パス
  (al-locate 53 + golden 16 + locate 12 + prepared 5 +
  argmax_tie 1、al-analysis 99 + 各 integration)
- `cargo clippy -p al-locate --all-targets -- -D warnings`: クリーン
- `cargo fmt --all -- --check`: パス(フォーマット適用済み)

## Risks

- mmap は「artifact が rename 公開後に不変」という前提に依存。
  手動で job ディレクトリの artifact を書き換えた場合の動作は
  保証対象外(design に記録済み)。
- 計測によるキャッシュ要否の判定は未実施(Slice 3)。

## Follow-ups

- caller 移行: `docs/impl/ir/pcmview-caller-migration.md`
- 計測・cache 判定: `docs/impl/ir/pcmview-measurement-cache-decision.md`
