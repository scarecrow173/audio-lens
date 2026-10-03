---
id: "IMPL-034z8DgpY2YxCiEdMepbuV"
type: "impl"
status: "completed"
title: "locate-integration-tests"
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
    - "docs/tasks/locate-integration-tests.md"
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

# locate-integration-tests

## Summary

合成 fixture の CLI 結合テストを追加。LCG ホワイトノイズのソース
(512 frame)をゲイン 0.5 で frame 2000 に埋め込んだキャプチャに対し
`al run --stages ingest,locate` を実行し、locate_result artifact が
受理済みで真の位置 ±256 frame 内に合議されることを検証する。

## Implementation

### Changes

- `crates/al-cli/tests/cli.rs`:
  `locate_finds_source_embedded_in_capture` /
  `locate_emits_time_map_and_residual_artifacts` /
  `locate_tracks_long_source_with_windowed_anchors` /
  `locate_tracks_linear_drift_as_drift_segment` /
  `locate_tracks_skip_and_resume_piecewise`

### Decisions During Implementation

- `--input` は `role:path,role:path` のカンマ区切り単一フラグ
  (繰り返し指定ではない) — テストで実 CLI 契約をそのまま使う
- 追加分は 4096 サンプルの長尺ソースを対象に、窓 anchor が複数
  生成されること、線形 1.1x ドリフトが `drift` 区間になること、
  スキップ+巻き戻しが `gap`/`resume` 区間になること、
  全 segment の capture/source 範囲が単調・非重複であることを
  検証する
- ドリフト fixture は全ソース相関で手法間の位置が約 270 サンプル
  ずれるため、`--config` で `locate.consensus.position_tolerance`
  を 512 に広げた実設定オーバーレイを使用する
  (`ConsensusConfig` の serde default で部分指定を許容)

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Context

task `docs/tasks/locate-integration-tests.md`、plan の検証要件
(`al run --input capture_bug:…,source_asset:… --stages ingest,locate`)
に従う。commit `1a742a6`。

## Validation

- `cargo test -p al-cli` 全 pass
- `cargo test --workspace` / clippy / fmt clean

## Risks

- 長尺・リニアドリフト・スキップ+リジュームは E2E 追加済み。
  EQ フィルタ済み挿入・極性反転 mixture 等の fixture は
  ユニットテストで個別カバー済み。さらなる E2E fixture は
  今後の拡張余地

## Follow-ups

- なし
