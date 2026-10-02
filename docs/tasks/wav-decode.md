---
id: "TASK-034z0J62VyM1KI8DpxaE3D"
type: "task"
status: "todo"
title: "WAV デコード・メタデータ・SHA-256 ハッシュ"
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
    - "docs/plans/implement-ingest-av.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-ingest-av.md"
    - "docs/tasks/media-skeleton.md"
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
    - "docs/test-specs/ingest-wav-decode.md"
  references: []
  defers: []
  deferred-by: []
---

# WAV デコード・メタデータ・SHA-256 ハッシュ

## Work

RIFF/WAV デマックスと PCM デコード(正規化前)、音声メタデータ
(コーデック・レート・チャンネル・ビット深度・長さ)記録、
自前 SHA-256 による元ファイルハッシュを実装する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ingest-av.md`
- `docs/specs/media-ingest.md`
- `docs/test-specs/ingest-wav-decode.md`

## Dependencies

- `docs/tasks/media-skeleton.md`

## Affected Files

- `crates/al-media/src/wav.rs`
- `crates/al-media/src/metadata.rs`
- `crates/al-media/src/hash.rs`

## Done When

- [ ] 合成 16bit ステレオ WAV がデコードされ PCM・メタデータ・
  ハッシュが得られる
- [ ] SHA-256 が既知テストベクトルと一致する
- [ ] 破損ヘッダが明示的失敗になる

## Verification

- [ ] `cargo test -p al-media` が通る
