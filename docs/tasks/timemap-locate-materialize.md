---
id: "TASK-034VirKgERFCR5QUVzfBTN"
type: "task"
status: "done"
title: "time_map.json の locate/ role materialize と SyncTimeline wire 経路接続(AL#191)"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/191"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/timemap-locate-materialize.md"
  implemented-by: []
  depends-on:
    - "docs/plans/timemap-locate-materialize.md"
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
    - "docs/test-specs/timemap-locate-materialize.md"
  references: []
  defers: []
  deferred-by: []
---

# time_map.json の locate/ role materialize と SyncTimeline wire 経路接続(AL#191)

## Work

Issue [#191](https://github.com/scarecrow173/audio-lens/issues/191)。
`materialize_locate` が job manifest の `time_map` role
artifact を dataset の `locate` role へ
`locate/<src>/<cap>/time_map.json` として複写し、
`al.ds.read` kind=`json` の既存 serving 経路から到達可能に
する。UI 側は `fakes.ts` の ds.read kind=`json` モデルと
`viz-timeline.test.ts` の実 wire 経路 test、
`timeline.ts` の `timeMapPaths` helper・envelope unwrap・
stale 注記更新を行う。headless のみ。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/timemap-locate-materialize.md`
- `docs/specs/timemap-locate-materialize.md`
- `docs/designs/timemap-locate-materialize.md`
- `docs/test-specs/timemap-locate-materialize.md`
- `docs/impl/ir/gui-ruviform-viz-issue-132.md`

## Dependencies

- なし(全て merge 済み)

## Affected Files

- `crates/al-dataset/src/stage.rs`
- `crates/al-dataset/tests/integration.rs`
- `ui/src/viz/timeline.ts`
- `ui/test/fakes.ts`
- `ui/test/viz-timeline.test.ts`
- `docs/`(本チェーン)

## Done When

- [x] `materialize_locate` が `time_map.json` を `locate/` role に複写し artifact として供給される
- [x] `SyncTimeline` の wire 経路が実データで動作し、ui test で検証される
- [x] 既存の f64/NaN verbatim・bounded pull 契約を維持(#132 の oracle を回帰しない)

## Verification

- [x] `mise run test` — al-dataset integration test PASS
- [x] `mise run check-ui` / `mise run test-ui` — viz-timeline PASS
- [x] `mise run check` / `mise run lint` / `mise run fmt`(fmt --check) — exit 0
