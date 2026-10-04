---
id: "TASK-034AaFWPLKohMACaTcKkDv"
type: "task"
status: "todo"
title: "mp4: PCM エントリ取り出し + 圧縮コーデック明示失敗"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "docs/plans/mp4-ingest.md"
  implemented-by: []
  depends-on:
    - "docs/plans/mp4-ingest.md"
    - "docs/tasks/mp4-box-parser.md"
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
    - "docs/test-specs/mp4-ingest.md"
  references: []
  defers: []
  deferred-by: []
---

# mp4: PCM エントリ取り出し + 圧縮コーデック明示失敗

## Work

非圧縮 PCM サンプルエントリ(sowt=LE/twos=BE/lpcm=フラグ解釈)を mdat から取り出し、整数スケール f64 + `SampleFormat` で返す(`decode_input` が format_scale で割る前正規化契約。二重正規化禁止)。複数 soun では最初の PCM-decodable トラックのみ対象とする。圧縮コーデックは `decode/input unavailable` を記録する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/mp4-ingest.md`
- `docs/test-specs/mp4-ingest.md`

## Dependencies

- `docs/tasks/mp4-box-parser.md`

## Affected Files

- `crates/al-media/src/mp4.rs`
- `crates/al-media/src/metadata.rs`

## Done When

- [ ] sowt/twos/lpcm エントリが正しいエンディアン/スケールで decoded PCM として取り出せる
- [ ] 圧縮コーデックの decode 要求が明示失敗

## Verification

- [ ] `cargo test -p al-media`
