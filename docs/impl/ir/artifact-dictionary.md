---
id: "IMPL-034z0XEXdIWZM92ZZF2emx"
type: "impl"
status: "completed"
title: "artifact dictionary"
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
    - "docs/tasks/artifact-dictionary.md"
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

# artifact dictionary

## Summary

バージョン付きキャプチャアーティファクト辞書(JSON)と注釈クエリを
実装し、初期版 `dictionaries/capture-artifacts.v1.json` を追加した。

## Context

task `docs/tasks/artifact-dictionary.md`、test-spec
`av-timing-observation` に従う。

## Implementation

### Changes

- `dictionary.rs`: `Dictionary` / `DictEntry` / `EntryMatch` /
  `Annotation`。`annotate(codec, container)` はセレクタの AND マッチ
  (未指定フィールドはワイルドカード)。返り値は全て辞書バージョン付き
- `Dictionary` は読み込んだ JSON テキストの SHA-256 を保持し、
  `Dictionary::embedded()` でバイナリ埋め込みの辞書を提供
  (既定が CWD に依存しないよう、かつ注釈の provenance を
  内容ハッシュで追跡できるようにするため)
- `dictionaries/capture-artifacts.v1.json`: 初期版。
  codec_hf_cutoff(aac/mp3)、container_block_size、src_route、
  lossy_codec_present の最小構成

### Decisions During Implementation

- 注釈は「コピーを返す」だけの API にし、生計測を変更する経路を
  型上持たない
- 初期収録は広く知られた事実(フレーム長・HF カットオフ)のみに限定

## Validation

- `cargo test -p al-media`: 20 tests pass(辞書ロード・マッチ・
  ワイルドカード・注釈のみ契約)
- `clippy -D warnings` / `cargo fmt --check`: clean

## Follow-ups

- 実測に基づく辞書項目の拡充は運用フェーズで追加

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Risks

- なし
