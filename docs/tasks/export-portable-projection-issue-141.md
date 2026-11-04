---
id: "TASK-034Rp5Mtq7VNi310OoekT0"
type: "task"
status: "done"
title: "Export 可搬投影と staging publish (AL#141)"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/141"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/export-portable-projection.md"
  implemented-by: []
  depends-on:
    - "docs/plans/export-portable-projection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "export-package.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Export 可搬投影と staging publish (AL#141)

## Work

Issue [#141](https://github.com/scarecrow173/audio-lens/issues/141)
を正本とし、コピー済み `analysis/` 配下の全 `.json` 成果物に
可搬投影を適用し、portability gate を全 spelling で検査して
staging publish で部分成果物を分離する。

- `package.rs`: `sanitize_json_tree` で `.json` 一括投影、
  `path_spellings` で `/`・`\`・`\\` 全 spelling の gate 検査、
  `export/.<name>.partial` staging → rename publish
- `base_name` を host 非依存 basename に統一(manifest 入力と
  copy_media ファイル名の導出)
- `export.rs` tests: declared/metadata 投影・fail-closed・
  rerun 保存を確認する 3 テスト追加
- 元 Dataset / Job manifest / Source Provenance は変更しない

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/export-portable-projection.md`
- `docs/designs/export-portable-projection.md`
- `docs/specs/export-artifact-portability.md`

## Dependencies

- `docs/tasks/export-package.md`(既存「絶対パス除去」作業の
  対象外だった成果物をカバー)

## Affected Files

- `crates/al-dataset/src/export/package.rs`
- `crates/al-dataset/tests/export.rs`

## Done When

- [x] `analysis/inputs/declared.json`・`*.metadata.json`・
      `manifest.json` の source_path / config がファイル名に
      縮約され、provenance・digest・stream metadata が保持される
- [x] Gate が `/`・`\`・`\\` 全 spelling で漏洩を検出する
- [x] 失敗した Export が `export/<name>/` に部分成果物を残さず、
      成功済みパッケージを破壊しない

## Verification

- [x] `cargo test -p al-dataset`(45 件全緑)
- [x] `cargo test -p al-cli`(15 件全緑、issue の 3 件含む)
- [x] `mise run fmt` / `cargo clippy -p al-dataset --all-targets
      -- -D warnings`
- [x] 実 job 実行後の `export/analysis/analysis/inputs/*.json` へ
      ホストパス grep が CLEAN
