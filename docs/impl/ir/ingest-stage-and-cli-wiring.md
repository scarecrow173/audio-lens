---
id: "IMPL-034z11SIMsstkfX6w5uKok"
type: "impl"
status: "completed"
title: "ingest stage and cli wiring"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/ingest-stage.md"
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

# ingest stage and cli wiring

## Summary

`IngestStage`(Stage trait 実装)を `al-media` に追加し、全入力の
検出・WAV デコード・SHA-256・辞書注釈を成果物として manifest に
登録する。`al-cli` で Ingest のスタブを実装に差し替えた。

## Context

task `docs/tasks/ingest-stage.md`、design `ingest-av` に従う。

## Implementation

### Changes

- `stage.rs`: `IngestStage`。入力ファイル読込 → フォーマット検出 →
  WAV デコード → `inputs/<input-id>/decoded.f64le`(生 f64 PCM)と
  `metadata.json`(AudioMetadata + annotations)を書き出し、
  `decoded_pcm` / `ingest_metadata` の ArtifactRecord を返す。
  辞書は `with_dictionary_file` で注入(不在なら注釈なし)
- `al-cli`: `make_stage()` を追加。Ingest は実装、他はスタブ維持。
  辞書パスは config `dictionary` キー、既定は
  `dictionaries/capture-artifacts.v1.json`

### Decisions During Implementation

- 未対応形式・読み込み失敗はステージ全体を `decode/input unavailable`
  で明示的失敗させる(部分的成功を作らない)
- PCM は `decoded.f64le`(raw little-endian)として分離保存し、
  メタデータは JSON に集約

## Validation

- 実 WAV(48kHz/16bit/2ch 正弦波)で `al run --stages ingest` →
  PCM + metadata.json が生成され hash/codec/annotations が記録される
- `al resume` が no-op で Completed を維持
- 存在しない入力で `decode/input unavailable` の明示的失敗(exit 2)
- `cargo test --workspace` / `clippy -D warnings` / `fmt --check`: clean

## Risks

- PCM を f64 raw で保存するため容量は大きい。圧縮・型選択は dataset
  プランで見直す

## Follow-ups

- 実コンテナパーサ追加時に `detect_format` / `ingest_input` の分岐を拡張
