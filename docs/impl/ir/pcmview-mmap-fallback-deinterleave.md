---
id: "IMPL-034G5Xo5i1U2eFPNXIb2la"
type: "impl"
status: "in-progress"
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
に実装し、`decode_input` と `read_normalized_record` の2経路を
移行した。mmap を第一候補とし、空ファイル・非 regular・mmap 失敗時は
`fs::read` ベースの owned buffer に fallback する。

## Context

`docs/tasks/pcm-view-foundation.md`(Slice 1)および
`docs/tasks/decode-callers-view-migration.md`(Slice 2)を実施。
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
- `decode_input`: `fs::read` + interleaved `Vec<f64>` 構築を
  `PcmView::open` + `view.deinterleave(channels, scale)` に置き換え。
  `len % 8 != 0` のエラー文言は `view.byte_len()` で完全に維持。
- `read_normalized_record`: 同様に `view.deinterleave(channels, 1.0)`
  に置き換え。byte length・有限性の検証を materialize 後の
  `channel_samples` 上で行い、同値の値集合で結果を保持。

### Decisions During Implementation

- caller 移行(Slice 2)は基盤が未使用だと non-test build で
  dead_code になるため、同一ファイル内で一連の変更として実装し、
  コミットは依存追加と機能実装の2つに分割した。
- 有限性検証は従来 interleaved `Vec<f64>` 上で行っていたが、
  deinterleave 後の `channel_samples.iter().flatten()` に移した。
  byte length 検証(`frames*ch*8` と厳密一致)を先に通るため
  部分フレームは存在せず、検査対象の値集合は同値。

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

- Slice 3: allocation/wall-time/RSS 計測と job-scoped cache 要否の
  判定(`docs/tasks/pcm-view-measurement-cache-decision.md`)。
- 判定結果に応じて roadmap・closeout を更新。
