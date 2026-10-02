---
id: "TASK-034z0J7BD4f6wbRbKvnycr"
type: "task"
status: "done"
title: "キャプチャアーティファクト辞書"
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
    - "docs/test-specs/av-timing-observation.md"
  references: []
  defers: []
  deferred-by: []
---

# キャプチャアーティファクト辞書

## Work

バージョン付きキャプチャアーティファクト辞書(JSON)の読み込みと、
コーデック / コンテナに対する注釈クエリ API を実装する。
辞書は注釈のみを返し、生計測を変更しない契約を型で表す。
初期版 `dictionaries/capture-artifacts.v1.json` を作成する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-ingest-av.md`
- `docs/specs/media-ingest.md`
- `docs/test-specs/av-timing-observation.md`

## Dependencies

- `docs/tasks/media-skeleton.md`

## Affected Files

- `crates/al-media/src/dictionary.rs`
- `dictionaries/capture-artifacts.v1.json`

## Done When

- [ ] 辞書がバージョン付きで読み込める
- [ ] クエリが注釈(参照情報)のみを返し生計測に触れない
- [ ] 注釈結果に辞書バージョンが含まれる

## Verification

- [ ] `cargo test -p al-media` が通る
