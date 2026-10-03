---
id: "IMPL-034zkPfGy6qmBc2hwZj6Nz"
type: "impl"
status: "completed"
title: "export-package"
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
    - "docs/tasks/export-package.md"
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

# export-package

## Summary

`export/<name>/` 配下に再配置可能な解析パッケージを構築する
`package.rs` と、決定的ゴールデンスナップショットの
`golden.rs` を実装した。

## Context

task `docs/tasks/export-package.md`、plan
`docs/plans/visualization-export.md`、spec
`docs/specs/visualization.md` に従う。

## Implementation

### Changes

- `crates/al-viz/src/package.rs`(新規):
  `ExportConfig`(`config.export.*`: `name` /
  `copy_media` / `golden_paths`)と `build(job_dir,
  manifest, cfg)`。analysis/manifest.json を必須とし、
  出力先を毎回全削除→再構築、dataset をコピー、コピー後の
  manifest から絶対パス依存を除去、任意で media コピー、
  `export_manifest.json` を出力、テキストファイルに絶対
  パスが残っていれば明示エラー
- `crates/al-viz/src/golden.rs`(新規): `snapshot(root,
  only)` でパッケージ内全ファイルの SHA-256 を
  パスソート済み一覧として出力
- `crates/al-viz/src/stage.rs`: `ExportStage` が
  `package::build` + golden 書き出しを実行

### Decisions During Implementation

- ゴールデンは「パッケージ全体」(export_manifest 含む、
  golden 自身を除く)を対象にし、`golden_paths` 設定時は
  そのパスに限定する
- パッケージ manifest の schema_version=1・generator=`al-viz`
  を固定値で持つ(pre-release のため互換処理なし)

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-viz --test viz`:
  `export_package_is_portable_deterministic_and_manifested` で
  パッケージ再構築・manifest・絶対パス残存なしを確認

## Risks

- `copy_media` で巨大メディアを複製するため出力サイズは
  入力次第(仕様どおり)

## Follow-ups

- なし
