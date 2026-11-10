---
id: "IMPL-034ViLXXlAu1Pm47cIRYLN"
type: "impl"
status: "completed"
title: "time_map.json の locate/ role materialize と SyncTimeline wire 経路接続(AL#191)"
created: "2026-11-10"
updated: "2026-11-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/191"
  changes:
    added:
      - type: "code"
        path: "docs/specs/timemap-locate-materialize.md"
      - type: "code"
        path: "docs/designs/timemap-locate-materialize.md"
      - type: "code"
        path: "docs/plans/timemap-locate-materialize.md"
      - type: "code"
        path: "docs/tasks/timemap-locate-materialize.md"
      - type: "code"
        path: "docs/test-specs/timemap-locate-materialize.md"
      - type: "code"
        path: "docs/impl/ir/timemap-locate-materialize.md"
    modified:
      - type: "code"
        path: "crates/al-dataset/src/stage.rs"
      - type: "code"
        path: "crates/al-dataset/tests/integration.rs"
      - type: "code"
        path: "ui/src/viz/timeline.ts"
      - type: "code"
        path: "ui/test/fakes.ts"
      - type: "code"
        path: "ui/test/viz-timeline.test.ts"
      - type: "doc"
        path: "docs/specs/README.md"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/plans/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/test-specs/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/timemap-locate-materialize.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/impl/ir/gui-ruviform-viz-issue-132.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/test-specs/timemap-locate-materialize.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# time_map.json の locate/ role materialize と SyncTimeline wire 経路接続(AL#191)

## Summary

Issue #191。`al-locate` が accepted pair ごとに job dir へ persist
する `time_map.json` を `DatasetStage::materialize_locate` が
dataset の `locate` role へ `locate/<src>/<cap>/time_map.json`
として複写し、`al.ds.read` kind=`json`(`DatasetReader::read_json`)
の既存 serving 経路から到達可能にした。UI 側は `fakes.ts` に
json artifact モデルを追加し、`timeline.ts` に `timeMapPaths`
helper と artifact envelope の unwrap を実装、`viz-timeline.test.ts`
に `al.ds.locate` → `al.ds.read` → `SyncTimeline` の実 wire 経路
test を追加した。#132 で deferred だった「dataset が time_map を
materialize しないため wire 経路が到達不能」の gap を解消する。

## Context

- task: `docs/tasks/timemap-locate-materialize.md`
- plan: `docs/plans/timemap-locate-materialize.md`
- spec: `docs/specs/timemap-locate-materialize.md`
- design: `docs/designs/timemap-locate-materialize.md`
- test-spec: `docs/test-specs/timemap-locate-materialize.md`
- 前提 IR: `docs/impl/ir/gui-ruviform-viz-issue-132.md`
  (SyncTimeline・`timeMapViewFromWire` 実装、time_map materialize
  は別 issue として deferred)

## Implementation

### Changes

- `crates/al-dataset/src/stage.rs` — `materialize_locate` の末尾に
  `time_map` role artifact の走査を追加。job manifest の
  `params.source_input_id`/`params.capture_input_id` から配置を
  決め、job dir の JSON を `serde_json::Value` として読み、
  `DatasetWriter::write_json("locate", "<src>/<cap>/time_map", ...)`
  で dataset へ書き込む。artifact_id + inputs を parents、
  `{"source": "time_map"}` を params とする Provenance を付与し、
  lineage に job artifact_id を追加する。params 欠落は
  `StageError` で fail-fast。
- `crates/al-dataset/tests/integration.rs` —
  `locate_time_map_materializes_into_dataset` を追加。accepted
  pair(time_map あり)と rejected pair(time_map なし)を持つ job
  manifest で dataset stage を実行し、ファイル存在・manifest 登録
  (role `locate` / StoreKind::Json)・`read_json` による verbatim
  read-back(f64 score 値を含む)・rejected pair の非存在を検証。
- `ui/test/fakes.ts` — `FakeDataset.jsonArtifacts`(artifact path →
  JSON 値)を追加し、`al.ds.read` kind=`json` を実装。name の
  `.json` 補完は `DatasetReader::read_json` と同規約。未登録は
  実 service と同じ `io` コード(`no Json artifact at <path>` —
  InvalidData io error → ServiceError Io)で失敗させる。
- `ui/src/viz/timeline.ts` — `timeMapPaths(locate)` helper を追加
  (`detail_paths` から `/time_map.json` suffix の path を選別)。
  `timeMapViewFromWire` は `{time_map: <TimeMap wire>, ...}` の
  artifact envelope を unwrap してから変換し、bare `{segments}`
  wire 形も従来どおり受理する。materialize 未対応を示す stale な
  コメント 2 箇所を現行契約へ更新。
- `ui/test/viz-timeline.test.ts` — `al.ds.locate` の
  `detail_paths` → `timeMapPaths` → `al.ds.read` kind=`json` →
  `timeMapViewFromWire` → `SyncTimeline` の一連の wire 経路 test
  を追加。piecewise 補間値(0.25s)が detail の定数オフセット写像
  (0.5s)と異なることを assert し、segment 範囲外は `undefined`
  を返すことも確認。

### Decisions During Implementation

- serving 層(al-service/al-gui)は変更しない — `al.ds.read`
  kind=`json` → `json_artifact` → `read_json` の既存経路がそのまま
  要件を満たすため、新 op を足すと policy が重複する。
- `time_map` artifact の params 欠落は skip ではなく `StageError`
  — manifest が artifact を宣言している以上、配置情報の欠落は
  データ不整合であり silent drop を避ける。
- fake の未登録 json artifact エラーは `not_found` ではなく `io`
  — 実 `read_json` は `InvalidData` io error を返し、
  `ServiceError::from` が `io` コードへ写像するのが実契約。
- envelope unwrap は `raw.time_map` が record のときのみ内側を
  使う — bare wire 形との後方互換(#132 oracle)を維持するため、
  `time_map` キーが無い入力は従来どおり `raw` 自体を segment 源と
  する。

## Related Experiments

### Adopted

なし(探索的実験は行っていない)。

### Rejected

なし。

## Validation

補助証拠(直接実行。canonical は `mise run *`):

- `cargo test -p al-dataset` — PASS(14 integration tests、
  `locate_time_map_materializes_into_dataset` 含む)
- `cargo clippy -p al-dataset --all-targets -- -D warnings` — PASS
- `cargo fmt -p al-dataset` 適用済み
- `node --test --import=./test/env.ts test/viz-timeline.test.ts`
  — PASS(9 tests、新規 wire 経路 test 含む)
- `npm --prefix ui test` — PASS(113 tests)
- `npm --prefix ui run typecheck`(tsc --noEmit)— PASS

canonical 検証(`git merge origin/main` は Already up to date の
うえで実施、Windows / mise 管理 toolchain):

- `mise run fmt`(`cargo fmt --all -- --check`)— PASS
  (`fmt-check` という task 名は存在せず `fmt` が check 相当)
- `mise run check`(`cargo check --workspace --all-targets`)— PASS
- `mise run lint`(biome check + `cargo clippy --workspace
  --all-targets -- -D warnings`)— PASS(biome の warning は
  既存の tools/cinematic-sword-cutscene 由来で本変更と無関係)
- `mise run test`(`cargo test --workspace`)— PASS
- `mise run check-ui`(sync 照合 + `tsc --noEmit`)— PASS
- `mise run test-ui`(`node --test`、113 tests)— PASS

review-protocol: Reviewer A(cooperative)/Reviewer B(adversarial)
の 2 pass を実施し、Validator が候補を実コードと照合、Judge が
severity 判定。CONFIRMED P0/P1 なし → ACCEPT。指摘への対応:
biome `useOptionalChain`/format/organizeImports を修正済み。

## Risks

- `detail_paths` に `time_map.json` が混在するため、detail 文書
  として全 path を読む既存消費者が誤読する可能性 — 現行の
  消費者は個別 path を `locate_detail` で引くため影響なし。
  `timeMapPaths` helper が suffix で選別する契約を test-spec で
  固定した。
- envelope の `time_map` キーと bare `segments` が同居する入力は
  envelope 側を優先する — artifact 本体の wire 形が変わった場合は
  `timeMapViewFromWire` の unwrap 規約を見直す必要がある。

## Follow-ups

- 実 GUI(al-gui/al-desktop)での SyncTimeline 描画配線は本 Issue
  の対象外(headless のみ)。
