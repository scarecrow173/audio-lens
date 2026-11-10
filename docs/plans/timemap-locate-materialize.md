---
id: "PLAN-034ViqBa1uvG8YFh7bNhqm"
type: "plan"
status: "completed"
title: "time_map materialize と SyncTimeline wire 経路の実装計画"
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
  implements:
    - "docs/specs/timemap-locate-materialize.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/timemap-locate-materialize.md"
    - "docs/designs/timemap-locate-materialize.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/timemap-locate-materialize.md"
  references:
    - "docs/impl/ir/gui-ruviform-viz-issue-132.md"
  defers: []
  deferred-by: []
---

# time_map materialize と SyncTimeline wire 経路の実装計画 Implementation Plan

## Goal

Locate stage が job dir に persist する `time_map.json`
(role `time_map`)を Analysis Dataset の `locate/` role へ複写し、
`SyncTimeline` の time_map 補間経路を実データで到達可能にする。

## Scope

単一の implementation stream。Rust 側(materialize + test)と
UI 側(fake + test + timeline.ts 更新)は同じ wire 契約
(`locate/<src>/<cap>/time_map.json` の JSON artifact)を共有し、
分割すると契約の齟齬を検証できないため 1 plan にまとめる。

## Upstream Documents

- `docs/specs/timemap-locate-materialize.md`
- `docs/designs/timemap-locate-materialize.md`
- `docs/impl/ir/gui-ruviform-viz-issue-132.md`(#132 の
  deferred note・既存 oracle)
- Issue [#191](https://github.com/scarecrow173/audio-lens/issues/191)

## Design Inputs

- `docs/designs/timemap-locate-materialize.md`

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/al-dataset/src/stage.rs` | `materialize_locate` に `time_map` role artifact の `locate/<src>/<cap>/time_map.json` 複写を追加 | 既存の locate_result 複写と同じ流儀。src/cap は `params.source_input_id`/`capture_input_id` から取得し欠落は明示 error |
| `crates/al-dataset/tests/integration.rs` | materialize → `DatasetReader::read_json` の read-back 検証を追加 | `al.ds.read` kind=`json` が委譲する同一経路 |
| `ui/test/fakes.ts` | `FakeDataset.jsonArtifacts` 追加 + `al.ds.read` kind=`json` の応答モデル | 実 service と同じ `.json` 補完・not_found 規約 |
| `ui/src/viz/timeline.ts` | `timeMapViewFromWire` の envelope unwrap、`timeMapPaths` helper 追加、stale 注記更新 | `SyncTimeline` 本体の意味は変更しない |
| `ui/test/viz-timeline.test.ts` | detail_paths → `al.ds.read` → `timeMapViewFromWire` → `SyncTimeline` の実 wire 経路 test | 定数オフセット写像と区別できる値で piecewise 補間を検証 |
| `docs/` 各種 | spec/design/plan/task/test-spec/impl record | 本チェーン |

## Implementation Sequence

- [x] T1: `crates/al-dataset/tests/integration.rs` に time_map
  materialize の失敗 test を追加する(job manifest に
  `locate_result` + `time_map` artifact を登録し、
  `analysis/locate/<src>/<cap>/time_map.json` の存在・manifest
  登録・`read_json` read-back・f64 verbatim を assert)
- [x] T2: `stage.rs` `materialize_locate` に `time_map` role の
  複写 loop を実装する
- [x] T3: `mise run test`(または対象 package の test)で T1 が
  green になることを確認する
- [x] T4: `ui/test/fakes.ts` に `jsonArtifacts` と
  `al.ds.read` kind=`json` を追加する
- [x] T5: `ui/src/viz/timeline.ts` に `timeMapPaths` helper と
  `timeMapViewFromWire` の envelope unwrap を実装し、stale な
  deferred 注記を更新する
- [x] T6: `ui/test/viz-timeline.test.ts` に wire 経路 test を
  追加する(detail_paths 発見 → ds.read → wire 変換 →
  SyncTimeline 消費)
- [x] T7: `mise run check-ui` / `mise run test-ui` で UI 側を
  検証する
- [x] T8: `docs/impl/ir/timemap-locate-materialize.md` に実装
  記録を残し、review-protocol レビューを実施する

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | `detail_paths` に `*/time_map.json` が混ざり、detail_paths を `<src>/<cap>.json` のみと仮定する消費者が誤読 | low | low | 現行消費者は exact-key lookup のみ。`timeMapPaths` は suffix で選別 |
| 2 | `time_map` artifact の `params` 欠落で dataset stage が失敗 | low | med | locate stage の契約上必ず存在。欠落は fail-fast で顕在化させる |
| 3 | `timeMapViewFromWire` の envelope 拡張が既存 wire 形の受理を壊す | low | med | bare `{segments}` を優先維持し、#132 の既存 test で回帰確認 |
| 4 | f64 verbatim 性の毀損(再 serialize で値が変わる) | low | med | serde_json の f64 往復は正確。test で代表値を assert |

依存: なし(先行 Issue は全て merge 済み)。

## Task Breakdown

- T1–T3: Rust materialize + test(同一ファイル群、逐次)
- T4–T7: UI fake/test/timeline(逐次、T3 と独立に着手可能だが
  wire 契約を共有するため同一 PR で検証)
- T8: IR 記録 + review

## Verification

| Step | Verification command or criteria | Pass condition |
|------|----------------------------------|----------------|
| T1–T3 | `mise run test` | al-dataset integration test を含め全 PASS |
| T4–T7 | `mise run check-ui` / `mise run test-ui` | typecheck + viz-timeline test PASS |
| 全体 | `mise run check` / `mise run lint` / `mise run fmt-check` | exit 0 |
| 契約維持 | 既存 `viz-timeline.test.ts` の #132 oracle test | 回帰なし |

## Open Questions / Gaps

- なし。`al.ds.read` kind=`json` の op 経路は `al-gui`
  `service.rs` に既存で、編集範囲外でも変更不要。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(単一 worker の bounded
  slice。独立 work stream なし)
- Independent work streams: none
- User approval required before dispatch: yes(本タスクでは
  review-protocol レビューを承認の代替とする)
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: `time_map` artifact の `params` に
  `source_input_id`/`capture_input_id` が必ずあるという
  locate stage の契約に依存する。契約が変わる場合は
  path-derived naming への切替を検討。
- reviewer 重点: `detail_paths` 混在の影響、envelope unwrap の
  優先順位、params 欠落時の fail-fast 妥当性。
