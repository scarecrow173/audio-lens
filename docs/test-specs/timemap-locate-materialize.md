---
id: "TSPEC-034VislcmPREwzmnWFNruv"
type: "test-spec"
status: "approved"
title: "time_map materialize と SyncTimeline wire 経路の保証"
created: "2026-11-10"
updated: "2026-11-10"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/timemap-locate-materialize.md"
    - "../plans/timemap-locate-materialize.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/timemap-locate-materialize.md"
  derived-by: []
  verifies:
    - "docs/specs/timemap-locate-materialize.md"
    - "docs/designs/timemap-locate-materialize.md"
  verified-by: []
  references:
    - "../impl/ir/gui-ruviform-viz-issue-132.md"
  defers: []
  deferred-by: []
---

# time_map materialize と SyncTimeline wire 経路の保証

## Purpose

`time_map.json` の dataset materialize と `SyncTimeline` の
wire 経路について、将来の変更で静かに失われてはいけない
保証を固定する。dataset layout・serving 経路・UI 変換の
いずれかが変わったとき、この test spec が「何を守るべきか」
の判断材料になる。materialize 経路または SyncTimeline の
wire 消費が撤去・再設計されるまで有効。

## Feature

Feature: time_map artifact の dataset materialize と
SyncTimeline wire 経路

## Rules

- Rule: materialize の網羅性 — job manifest に `time_map`
  role artifact が存在すれば、dataset の `locate` role に
  `locate/<source>/<capture>/time_map.json` として登録され、
  `locate/<source>/<capture>.json`(locate_result)と共存する。
- Rule: optional 性 — accepted でない pair 等 time_map を
  持たない alignment は artifact を生成せず、dataset stage
  はエラーにしない。
- Rule: verbatim 性 — dataset 側の JSON 値は job dir 側と
  同一内容を保持する(f64 値を含む。#132 の f64/NaN oracle
  を回帰させない)。
- Rule: serving 経路 — materialize された artifact は
  `DatasetReader::read_json("locate", "<src>/<cap>/time_map")`
  (= `al.ds.read` kind=`json` の委譲先)で取得できる。
- Rule: wire 変換 — `timeMapViewFromWire` は artifact 本体
  envelope(`{time_map: <TimeMap>, ...}`)と bare TimeMap
  wire 形(`{segments}`)の両方を受理し、壊れた入力は
  `undefined` を返す(変換を捏造しない)。
- Rule: timeline 消費 — `SyncTimeline` は `timeMaps` に
  path key(`locate/<src>/<cap>/time_map.json`)で供給された
  view を使い、`captureToSourceS` が time_map segment の
  piecewise 補間を優先し、写像不能は `undefined` を返す。

## Examples

- Example: accepted pair の materialize — job dir に
  `locate/src-1/cap-1/{result.json,time_map.json}` と
  対応する manifest artifact があるとき、dataset stage 実行後
  `analysis/locate/src-1/cap-1/time_map.json` が存在し、
  dataset manifest の `locate` role(StoreKind::Json)に登録
  され、`read_json` が job dir 側と同じ JSON 値を返す。
- Example: time_map を持たない pair — `locate_result` のみの
  pair では time_map artifact が dataset に現れず、stage は
  成功する。
- Example: wire 経路 — `al.ds.locate` の `detail_paths` に
  `locate/src-1/cap-1/time_map.json` が現れ、`al.ds.read`
  kind=`json`(role=`locate`, name=`src-1/cap-1/time_map`)が
  artifact 本体を返し、`timeMapViewFromWire` が `TimeMapView`
  を返し、`SyncTimeline.captureToSourceS` が detail の
  定数オフセット写像とは異なる piecewise 補間値を返す。
- Example: 回帰 — `timeMapViewFromWire` の既存 test
  (#132:bare wire 形・不正形の `undefined`)が変更なく PASS
  する。

## Guarantees

- dataset だけを開いた消費者が time_map を発見・取得できる
  (`detail_paths` 列挙 + `al.ds.read` kind=`json`)。
- `SyncTimeline` の capture→source 写像が time_map の
  piecewise 意味(drift/jitter/skip/gap/resume)を反映する。
- f64 値・bounded pull 契約など #132 の既存 oracle が
  回帰しない。

## Non-goals

- `locate_evidence` 等他の job dir artifact の materialize。
- 実 GUI/実 OS での描画検証(headless のみ)。
- `al.ds.locate_detail` の応答形式変更。
- time_map の内容生成ロジック(build_time_map)自体の検証
  (al-locate 側の責務・既存 test が担保)。

## Risk

- materialize が欠けると wire 経路が再び到達不能になり、
  piecewise 写像の検証が静かに骨抜きになる。
- envelope unwrap の優先順位を誤ると bare wire 形の受理が
  壊れ、#132 の oracle が回帰する。
- `detail_paths` への time_map 混在を誤って detail 文書と
  して扱う消費者が現れると誤読する(現行消費者は影響なし)。
