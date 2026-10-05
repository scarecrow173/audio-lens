---
id: "TASK-034AaFAJYK7iYyxVUmUpoL"
type: "task"
status: "todo"
title: "avsync: 音声イベント↔映像イベント照合"
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
    - "docs/plans/video-av-features.md"
  implemented-by: []
  depends-on:
    - "docs/plans/video-av-features.md"
    - "docs/tasks/video-features-ingest.md"
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
    - "docs/test-specs/video-av-features.md"
  references: []
  defers: []
  deferred-by: []
---

# avsync: 音声イベント↔映像イベント照合

## Work

`al-analysis` に avsync 照合を実装する。ingest の映像イベント時刻系列と cheap の音声オンセットを、設定可能な探索窓の最近傍で対応付け、ペアごとの差分・確度と、系列の median/MAD・線形フィットによる drift 傾きを `avsync.*` レコードとして記録する。照合前に両系列を各ストリームクロック由来の秒時刻へリベースし、`dwStart≠0` のコンテナレベルオフセットが delta に混入しないようにする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/video-av-features.md`
- `docs/test-specs/video-av-features.md`

## Dependencies

- `docs/tasks/video-features-ingest.md`

## Affected Files

- `crates/al-analysis/src/avsync.rs`
- `crates/al-analysis/src/stage.rs`
- `crates/al-analysis/src/lib.rs`

## Done When

- [ ] 音声/映像イベントのペアに delta と confidence が記録される
- [ ] auds/vids の dwStart≠0 fixture で delta が両クロックの差だけずれない
- [ ] 映像系列がない場合は validity で明示される
- [ ] 分布 median/MAD と drift 傾きが記録される

## Verification

- [ ] `cargo test -p al-analysis`
