---
id: "IMPL-034zkPg5p5rQVa7CP3hiKj"
type: "impl"
status: "completed"
title: "viz-export-integration-tests"
created: "2026-10-03"
updated: "2026-10-03"
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
    - "docs/tasks/viz-export-integration-tests.md"
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

# viz-export-integration-tests

## Summary

viz/export の結合テスト `crates/al-viz/tests/viz.rs` と、
フルパイプライン E2E を `crates/al-cli/tests/cli.rs` に
追加した。

## Context

task `docs/tasks/viz-export-integration-tests.md`、test-spec
`docs/test-specs/visualization-export.md` に従う。

## Implementation

### Changes

- `crates/al-viz/tests/viz.rs`(新規、4 件):
  - `visualization_produces_timeline_lod_tiles_views_and_summary`:
    シードした dataset から全 product 群が生成されること
  - `visualization_rerun_replaces_products_deterministically`:
    2 回実行で全出力の SHA-256 が一致すること
  - `export_package_is_portable_deterministic_and_manifested`:
    パッケージ構築・manifest・決定性を確認
  - `produced_role_without_expected_store_fails_explicitly`:
    produced ロールの store 契約違反が明示エラーになること
- `crates/al-cli/tests/cli.rs`:
  `full_pipeline_reaches_visualization_and_export` で
  `ingest → … → export` 完走とパッケージ出力を確認
- `crates/al-viz/Cargo.toml`: dev-dependency に
  `al-analysis`(fixture で features JSON を生成するため)

### Decisions During Implementation

- ゴールデンスナップショットは「ファイルをリポジトリに
  固定」せず、実行時に 2 回実行して全ファイル hash を比較する
  方式にした。fixture の絶対パスが環境依存で変わるため
  固定スナップショットでは再現できない

## Related Experiments

### Adopted

- なし

### Rejected

- `tests/golden/` への固定スナップショット: fixture 生成の
  絶対パス差分で環境間一致が崩れるため却下し、同一実行内での
  再実行一致を検証する方式に置き換えた

## Validation

- `cargo test -p al-viz --test viz`: 4 件全 pass
- `cargo test -p al-cli --test cli`: 7 件全 pass
- `cargo test --workspace` 全 pass、clippy clean

## Risks

- なし

## Follow-ups

- なし
