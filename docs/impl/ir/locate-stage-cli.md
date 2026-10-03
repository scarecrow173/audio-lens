---
id: "IMPL-034z8Dg25tmGhA8pXvBQhj"
type: "impl"
status: "completed"
title: "locate-stage-cli"
created: "2026-10-02"
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
    - "docs/tasks/locate-stage-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# locate-stage-cli

## Summary

`LocateStage` を実装し `al-cli` に接続。`manifest.inputs[].role` で
`source_asset` × `capture_bug`/`capture_reference` をペアリングし、
ingest の `decoded.f64le` + `metadata.json` から ch 数・fs・
sample_format を復元して ch 単位で定位する。fs 不一致ペアは
リサンプルせず `DecodeInputUnavailable` で明示マーク。

## Implementation

### Changes

- `crates/al-locate/src/stage.rs`: `LocateStage` / `LocateConfig`
- `crates/al-cli/src/main.rs`: `StageKind::Locate` 配線 + USAGE 更新
- `crates/al-cli/Cargo.toml`: `al-locate` 依存追加

### Decisions During Implementation

- 定位証拠は ch ペアごとに収集し、手法ごとに最良ペアを投票代表にする。
  全 ch ペアの証拠は `locate_evidence` artifact(evidence.json)として
  別途永続化し、ch 詳細を残す
- decoded PCM はソースネイティブスケールなので `sample_format` で
  float 域に正規化してからメソッドへ渡す
- 受理時は一致窓で mixer 推定を result.json の `derived` に同梱し、
  ループ出現・anchor・`build_time_map` による区分的 time map は
  `time_map` artifact(time_map.json)として分離登録する
  (レビュー修正: 以前は anchor が常に source=0 の擬似区間で、
  drift/jitter/skip/gap/resume を表現できなかった)
- anchor はソースを `anchor_window`/`anchor_hop` の窓に分割して
  窓ごとに NCC 定位して生成する(`best_window_match` /
  `window_anchors_in`)。合議スコア・ピーク比の閾値を窓単位で
  適用し、長尺ソース内の drift/jitter/skip/gap/resume を
  観測可能にする(レビュー修正: 以前は発生ごとに 1 anchor のみで
  内部のずれを表現できなかった)。窓 anchor が得られない場合は
  従来の出現 anchor にフォールバックする
- 合議が検出した出現位置の Voronoi セルを取り、主出現以外の
  出現はソース先頭窓がそのセル内で実際に一致するかで検証する。
  検証済み出現が 2 件以上なら出現ごとのセル内で窓定位し、
  ループグリッド上に展開した source 座標で anchor を構成する。
  1 件なら capture 全域を走査してプレーンなソース座標で構成する
  (レビュー修正: 検証なしの出現クラスタをそのまま使うと
  偽ピークが擬似パス化し anchor 列が破壊されていた)
- `LocateConfig` に `anchor_window`/`anchor_hop`/`anchor_gap`
  (既定 512/512/256)を追加。`ConsensusConfig` に container 級の
  `#[serde(default)]` を付け、部分オーバーレイ設定を可能にした
- 受理ペアごとに残差探索を実行: 受理アライメント窓の混合を減算し、
  残差キャプチャに全ソースを再定位。減算来歴(親ソース・
  alignment・チャンネル変換・エネルギー・子 locate)は
  `residual` artifact(residual_<parent>.json)として登録する
  (レビュー修正: `residual_search` がステージから未接続だった)
- 複数の正当な出現は手法候補の残クラスタとして `occurrences` に記録
  (クラスタ幅は `position_tolerance` で上限)

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-stage-cli.md`、plan
`docs/plans/locate-and-tracking.md`。commit `1a742a6`。

## Validation

- `al run --stages ingest,locate` で locate_result artifact が
  manifest に登録される(CLI 結合テストで検証)

## Risks

- なし

## Follow-ups

- なし
