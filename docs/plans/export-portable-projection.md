---
id: "PLAN-034Rp5hEUjbyXqjbKwlxDA"
type: "plan"
status: "completed"
title: "Export 可搬投影の実装"
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
    - "docs/specs/export-artifact-portability.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/export-artifact-portability.md"
    - "docs/designs/export-portable-projection.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Export 可搬投影の実装 Implementation Plan

## Goal

`analysis/` 配下の全 `.json` 成果物にスキーマ耐性のある可搬投影を
適用し、gate を全 spelling で検査、staging publish で部分成果物を
分離して、Export パッケージのプライベートパス流出を遮断する。

## Scope

`crates/al-dataset/src/export/package.rs` とそのテストのみ。
元データセット・Job manifest・CLI・golden・解析数値は触れない。

## Upstream Documents

- `docs/specs/export-artifact-portability.md`(approved)
- `docs/designs/export-portable-projection.md`(approved)
- Issue #141

## Design Inputs

- `docs/designs/export-portable-projection.md`

## File Map

- `crates/al-dataset/src/export/package.rs` —
  `shrink_abs_paths`(変更検出)、`sanitize_json_tree`、
  `path_spellings`、`build`/`build_package` 分離と staging publish、
  gate の spelling 拡張、`base_name` による host 非依存 basename
- `crates/al-dataset/tests/export.rs` —
  `seed_input_records` fixture、3 つの新規 integration test

## Implementation Sequence

- [x] `package.rs` の投影パス・staging publish・gate spelling
      拡張を実装
- [x] `export.rs` に投影・fail-closed・rerun の 3 テストを追加
- [x] `mise run fmt` / `cargo clippy -p al-dataset` /
      `cargo test -p al-dataset` / `-p al-cli` を緑にする
- [x] 実 Export 成果物の漏洩スキャンで E2E 確認

## Dependencies and Risks

- `.json` 値の serde_json 再シリアライズで数値表記が正規化
  される可能性 — 値は保持、変更ファイルのみ byte 変化
- 絶対パスらしい非パス文字列の誤縮約 — 既存 `config` 縮約と同じ
  安全側のトレードオフとして受容
- Windows の `fs::rename` は dest 既存時に先の撤去が必要 —
  publish 直前に `remove_dir_all` で対応

## Task Breakdown

- `export-portable-projection-issue-141` — 本 plan の全実装・
  検証を 1 タスクとして実施済み

## Verification

- [x] `cargo test -p al-dataset`(45 件全緑)
- [x] `cargo test -p al-cli`(15 件全緑、issue の 3 件含む)
- [x] `mise run fmt` / `cargo clippy -p al-dataset --all-targets
      -- -D warnings`
- [x] 実 job 実行後の `export/analysis/analysis/inputs/*.json` へ
      `akiyoshi`/ホストパス grep が CLEAN

## Open Questions / Gaps

- Ubuntu CI lane の実測 pass は本 worker の環境外 — 同一修正で
  Windows の全テスト緑と成果物スキャン CLEAN を確認済み。CI 実
  走結果は coordinator 側で報告する

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none(単一 crate の局所変更)
- User approval required before dispatch: no(実装済み)

## Review Handoff

- `shrink_abs_paths` が全ブランチを走査する非短絡評価である点と、
  `path_spellings` が JSON エスケープ `\\` を正しくカバーする点を
  レビュー対象とする
