---
id: "DESIGN-034Vioow1Mm5KhHailH7rG"
type: "design"
status: "approved"
title: "time_map materialize と SyncTimeline wire 経路の設計"
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
    - "../impl/ir/gui-ruviform-viz-issue-132.md"
  refines:
    - "../specs/timemap-locate-materialize.md"
  refined-by: []
  derives-from:
    - "docs/specs/timemap-locate-materialize.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../impl/ir/gui-ruviform-viz-issue-132.md"
  defers: []
  deferred-by: []
---

# time_map materialize と SyncTimeline wire 経路の設計

## Context

`al-locate` の `LocateStage` は accepted な alignment 毎に
`locate/<source>/<capture>/time_map.json`(role `time_map`)を
job dir へ persist する(`persist_json` — 本体は
`{time_map: TimeMap, anchors, loop_occurrences, anchor_reference,
source_coordinate_space, source_pcm, capture_pcm}` の envelope)。
一方 `al-dataset` の `materialize_locate` は `locate_result`
artifact のみを `locate/<src>/<cap>.json` として複写し、
`time_map` role を dataset へ持ち込まない。このため #132 で
実装済みの `SyncTimeline`(`ui/src/viz/timeline.ts`)の
`timeMaps` 経路は供給元を持たず到達不能だった。

serving 側は既に汎用: `al.ds.locate` の `detail_paths` は
`locate` role の全 JSON artifact を列挙し、`al.ds.read`
kind=`json`(`DatasetSession::json_artifact` →
`DatasetReader::read_json`)は `<role>/<name>` の JSON artifact
をそのまま返す。変更が必要なのは materialize と UI 側の
消費経路の検証のみで、service op 契約は変更しない。

## Scope

### In Scope

- `materialize_locate` への `time_map` role artifact の複写
- 複写の Rust 側検証(integration test)
- UI 側の wire 経路検証(fake の `al.ds.read` kind=`json`、
  `viz-timeline.test.ts`、`timeline.ts` の注記更新と
  detail_paths からの発見 helper)

### Out of Scope

- spec の Out of Scope に同じ(ruviform 変更、GUI 配線、
  `SyncTimeline` 再設計、`locate_evidence` 等他 artifact)

## Components and Boundaries

- `crates/al-dataset/src/stage.rs` `materialize_locate`:
  job manifest の `role == "time_map"` artifact を走査し、
  `locate/<src>/<cap>/time_map.json` として `locate` role に
  複写する。src/cap は artifact の `params`
  (`source_input_id`/`capture_input_id` — locate stage の
  `persist_json` が必ず記録する)から取り、欠落時は明示
  error とする(不正 artifact を黙って落とさない既存流儀に
  合わせる)。
- `crates/al-dataset/tests/integration.rs`: materialize と
  `DatasetReader::read_json("locate", "<src>/<cap>/time_map")`
  による read-back を検証する test を追加。
- `ui/test/fakes.ts`: `FakeDataset.jsonArtifacts`
  (`"<role>/<name>.json"` → 応答本体)を追加し、
  `al.ds.read` kind=`json` を実 service と同じ name 正規化
  (`.json` 補完)・not_found 規約でモデル化する。
- `ui/src/viz/timeline.ts`:
  - `timeMapViewFromWire` が artifact 本体の envelope
    (`{time_map: <TimeMap wire>, ...}`)を unwrap して受理する
    よう拡張する。bare の `{segments}` wire 形も従来どおり受理。
  - `timeMapPaths(locate)` helper を追加し、`al.ds.locate`
    の `detail_paths` から `*/time_map.json` を発見する。
  - stale になった deferred 注記を実態に合わせて更新する。
- `ui/test/viz-timeline.test.ts`: detail_paths 発見 →
  `al.ds.read` kind=`json` → `timeMapViewFromWire` →
  `SyncTimeline` の `timeMaps` → `captureToSourceS` が
  piecewise 補間を使うことを、定数オフセット写像と区別
  できる値で検証する。

## Data and Control Flow

1. Locate stage: accepted pair 毎に `time_map.json` を
   `locate/<src>/<cap>/` 配下へ persist(role `time_map`)。
2. Dataset stage: `materialize_locate` が `locate_result`
   を `locate/<src>/<cap>.json` へ、`time_map` を
   `locate/<src>/<cap>/time_map.json` へ複写し、dataset
   manifest の `locate` role に StoreKind::Json で登録。
3. Serving: `al.ds.locate` の `detail_paths` に
   `locate/<src>/<cap>/time_map.json` が現れる。
   `al.ds.read {kind:"json", role:"locate",
   name:"<src>/<cap>/time_map"}` が artifact 本体を返す。
4. UI: `timeMapPaths` で発見 → `al.ds.read` で取得 →
   `timeMapViewFromWire` で `TimeMapView` 化 →
   `SyncTimeline({timeMaps})` へ path key で供給 →
   `captureToSourceS` が `TimeMap::capture_to_source` と
   同規約の piecewise 補間を使う。

## Non-Functional Constraints

- headless のみ: 実 GUI/実 OS を必要としない。
- verbatim 性: JSON 値は parse→再 serialize の既存流儀を
  踏襲し、f64 値・NaN 透過の #132 oracle を回帰させない
  (JSON に NaN は存在せず、f64 は serde_json の往復で
  bit 正確)。
- bounded pull 契約(#132)は変更しない — time_map は小さな
  JSON artifact で pull 対象外。
- 冪等: `DatasetWriter::write_json` の reset_store 規約により
  resume/再実行で重複・混在しない。

## Risks and Trade-offs

- `detail_paths` に `*/time_map.json` が混ざることで、
  detail_paths を「`<src>/<cap>.json` のみ」と仮定する
  消費者が誤読するリスク → 現行消費者(SyncTimeline の
  exact-key lookup、al-gui app.js の alignments 利用)は
  影響なし。`timeMapPaths` は `/time_map.json` suffix で
  選別する。
- `timeMapViewFromWire` の envelope 受理拡張は受理形を
  広げるが、bare wire 形の判定を優先順位付けで維持する
  (`time_map` record がある場合のみ unwrap)。
- `time_map` artifact の `params` 欠落 → 明示 error。
  locate stage の契約上必ず存在するため、欠落は不正
  manifest として fail-fast が正しい。

## Deferred Design Concerns

- `locate_evidence` の materialize(spec の Deferred と同じ)

## Verification Notes

- [ ] `crates/al-dataset` integration test: time_map artifact が
  `analysis/locate/<src>/<cap>/time_map.json` に存在し、
  dataset manifest の `locate` role に登録され、
  `DatasetReader::read_json` で job dir 側と同一値が読める
- [ ] `mise run test`(al-dataset)・`mise run test-ui`
  (viz-timeline)が green
- [ ] `timeMapViewFromWire` の既存 test(#132 oracle)が
  回帰しない
