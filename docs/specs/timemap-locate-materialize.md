---
id: "SPEC-034VimECyj7GAvAfJLzSir"
type: "spec"
status: "approved"
title: "time_map.json の locate/ role materialize と SyncTimeline wire 経路接続(Issue #191)"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/timemap-locate-materialize.md"
    - "../test-specs/timemap-locate-materialize.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/timemap-locate-materialize.md"
  references:
    - "../impl/ir/gui-ruviform-viz-issue-132.md"
    - "../tasks/gui-ruviform-viz-issue-132.md"
  defers: []
  deferred-by: []
---

# time_map.json の locate/ role materialize と SyncTimeline wire 経路接続(Issue #191)

## What Should Be Built

Locate stage が job dir の `locate/<source>/<capture>/time_map.json`
へ persist する piecewise time map artifact(role `time_map`)を、
`DatasetStage::materialize_locate` が Analysis Dataset の `locate/`
role へ `locate/<source>/<capture>/time_map.json` として複写し、
既存の artifact serving 契約(`al.ds.artifacts` 一覧・
`al.ds.read` kind=`json`)から取得可能にする。

これにより #132 で実装済みの `SyncTimeline` の `timeMaps` 経路
(`timeMapViewFromWire` → `captureToSourceS` の segment/anchor
補間)が実データで到達可能になり、headless の ui test で
wire 応答 → view 変換 → timeline 消費までを検証する。

## Why Now

#132 の IR(`docs/impl/ir/gui-ruviform-viz-issue-132.md`)に
deferred note として記録されていたギャップ: 現行 dataset は
`time_map.json` を job dir にのみ persist し `locate/` role へ
materialize しないため、`SyncTimeline` の time_map 補間経路は
到達不能で、capture→source 写像は `locate_detail` の定数
オフセット写像のみに留まっていた。#132 の受入条件外だったため
独立 Issue(#191)として切り出された。

## Users and Value

- AudioLens の開発者・GUI 消費側: dataset だけを開けば
  piecewise な capture↔source 対応(drift/jitter/skip/gap/resume
  区間)を取得でき、同期 timeline の写像精度が上がる。
- `al.ds.read`/`al.ds.artifacts` の既存利用者: 新しい op を
  増やさず既存の汎用 JSON artifact 経路で到達できる。

## Scope

### In Scope

- `crates/al-dataset/src/stage.rs` の `materialize_locate` に
  `time_map` role artifact の `locate/` role への複写を追加
  (既存の locate_result 複写と同じ流儀・冪等)。
- `crates/al-dataset` の integration test で materialize と
  reader/service 層の read 経路(`DatasetReader::read_json` —
  `al.ds.read` kind=`json` が委譲する同一経路)を検証。
- `ui/` 側: `fakes.ts` の `al.ds.read` kind=`json` 応答モデル、
  `viz-timeline.test.ts` での実 wire 形を使った経路検証、
  `timeline.ts` の stale な deferred 注記の更新と detail_paths
  からの time_map 発見 helper。
- `docs/` の spec→design→plan→task→test-spec→impl record
  チェーン。

### Out of Scope

- ruviform 側の変更。
- `crates/al-gui/`・`crates/al-desktop/` の GUI 配線
  (headless のみで完結。実 GUI/実 OS は対象外)。
- `SyncTimeline` の再設計や `timeMapViewFromWire` の意味変更
  (envelope unwrap の追加のみ)。
- Non-Hz `SpectrumInfo.axis`(ruviform 側別 Issue)。
- `locate_evidence` 等他の job dir 残留 artifact の materialize。

## Requirements

- `materialize_locate` は job manifest の `time_map` role
  artifact を読み、dataset の `locate` role に
  `locate/<source>/<capture>/time_map.json` として登録する。
- time_map を持たない pair(accepted でない alignment)は
  artifact を生成せず、missing をエラーにしない。
- dataset 側の JSON 値は job dir 側と同一内容を保持する
  (f64 値の verbatim 性を保つ — #132 の oracle を回帰しない)。
- UI 側は `al.ds.locate` の `detail_paths` から time_map
  artifact を発見し、`al.ds.read` kind=`json` 相当の経路で
  取得した実 wire 形を `timeMapViewFromWire` で変換して
  `SyncTimeline` の `timeMaps` へ供給できる。

## Acceptance Criteria

- [ ] `materialize_locate` が `time_map.json` を `locate/` role に複写し artifact として供給される
- [ ] `SyncTimeline` の wire 経路が実データで動作し、ui test で検証される
- [ ] 既存の f64/NaN verbatim・bounded pull 契約を維持(#132 の oracle を回帰しない)

## Deferred Design Concerns

- `locate_evidence` artifact の dataset materialize | dataset が
  evidence を必要とする消費者が現れた時点 | evidence 閲覧の要件
  が出たとき | evidence が dataset 外に残り export package が
  不完全になる

## Sources and Relations

- Issue [#191](https://github.com/scarecrow173/audio-lens/issues/191)
- Issue [#132](https://github.com/scarecrow173/audio-lens/issues/132)
  の deferred note:`docs/impl/ir/gui-ruviform-viz-issue-132.md`
- 上流 spec:`docs/specs/analysis-dataset.md`、
  `docs/specs/source-localization.md`(locate-tracking 系)
