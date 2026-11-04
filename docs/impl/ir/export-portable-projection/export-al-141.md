---
id: "IMPL-034Rpl0BVSth9xoYVXtfGd"
type: "impl"
status: "completed"
title: "Export 可搬投影 (AL#141)"
created: "2026-11-03"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/141"
  changes:
    added: []
    modified:
      - type: "code"
        path: "crates/al-dataset/src/export/package.rs"
      - type: "code"
        path: "crates/al-dataset/tests/export.rs"
      - type: "doc"
        path: "docs/specs/export-artifact-portability.md"
      - type: "doc"
        path: "docs/designs/export-portable-projection.md"
      - type: "doc"
        path: "docs/plans/export-portable-projection.md"
      - type: "doc"
        path: "docs/tasks/export-portable-projection-issue-141.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/export-portable-projection-issue-141.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/export-artifact-portability.md"
    - "docs/designs/export-portable-projection.md"
    - "docs/plans/export-portable-projection.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Export 可搬投影 (AL#141)

## Summary

`build_package` は `analysis/` を丸ごとコピーしつつ `manifest.json`
の inputs/config にしか縮約を適用していなかったため、
`inputs/declared.json` と `inputs/*.metadata.json` が絶対パス入りの
まま漏洩していた。コピー済み analysis 配下の全 `.json` 成果物に
再帰的な portable projection を適用し、portability gate を全
separator/escape spelling に拡張、staging publish で部分成果物を
分離した。

## Context

- task: `docs/tasks/export-portable-projection-issue-141.md`
- plan: `docs/plans/export-portable-projection.md`
- design: `docs/designs/export-portable-projection.md`
- spec: `docs/specs/export-artifact-portability.md`

Ruviform consumer-verify run #41604498852 の Ubuntu lane で
`crates/al-cli/tests/cli.rs` の 3 テストが
`export file analysis/inputs/<input_id>.metadata.json still embeds
absolute path` で失敗していた(Ruviform #219 / AudioLens #127 の
継続回帰)。

## Implementation

### Changes

- `sanitize_json_tree`: コピー済み `analysis/` 配下の全 `.json` を
  parse し、絶対パスらしい文字列値(POSIX root / drive-letter /
  UNC / rooted backslash)を `base_name` で最終 segment に縮約。
  変更のあった文書のみ `to_string_pretty` で再シリアライズし、
  無変更文書は bytes を保持。parse 不能な `.json` は `InvalidData`
  で fail-closed。
- `shrink_abs_paths` は `bool` を返し全分岐を巡回(短絡せず全
  sibling を投影)。
- `path_spellings`: gate が `/` 正規化・`\` 生・JSON escaped `\\`
  の全 spelling を検査。対象は declared input paths と job dir の
  記録 spelling + 絶対化 spelling。
- `base_name`: host 非依存 basename(`/`・`\` 両方で最後の非空
  segment)。manifest 入力と copy_media の file name 導出を
  統一し、`file_name` が空または絶対形式の入力を拒否。
- staging: `export/.<name>.partial` で全構築・gate を完了し、
  成功時のみ rename で `export/<name>/` に publish。失敗時は
  staging を除去し既存 publish 済みパッケージを保持。
- 元 `analysis/` / Job manifest / Source Provenance は不変。

### Decisions During Implementation

- manifest の `input_records`(export_manifest.json 用)は投影
  **前**のコピーから読み、digest/role/input_id を保存した上で
  `media_path` には縮約後 basename を記録。
- Clippy `unnecessary_fold` 回避のため `any` ではなく
  `changed |= shrink_abs_paths(item)` の明示ループ。`any` は
  最初の変更で短絡し残 sibling を未投影にする。
- 検証は `looks_absolute`/`base_name`/`shrink_abs_paths`/
  `path_spellings` の unit test(24 件)と、実 E2E の
  `export.rs` integration test(8 件、declared/metadata 投影・
  fail-closed・rerun 保存を含む新規 3 件)に配置。

## Related Experiments

### Adopted

- なし。

### Rejected

- per-field スキーマ駆動(既知フィールドだけの縮約): 将来の
  テキスト成果物を契約でカバーする必要があるため棄却。
- 任意文字列置換の gate 緩和: 無関係テキストを破壊し得るため
  spec の通り棄却。

## Validation

- `mise run lint` — clippy 全 workspace 0 warning(exit 0)。
- `mise run test` — `cargo test --workspace` 全 pass。
  `al-dataset` unit 24 + export integration 8 + dataset 13、
  `al-cli` 15 件(issue の 3 件含む)。
- `mise run check` / `mise run fmt` — exit 0。
- 実 job に対して rebuilt `al` を実行し、
  `export/analysis/analysis/inputs/*.json` へのホストパス grep が
  CLEAN。元 dataset/job metadata は絶対パスを保持。
- OS: Windows 11、toolchain: Rust 1.99.0。
- **Ubuntu / Linux E2E と Ruviform 横断 consumer-verify は本機では
  未検証。** coordinator 側での再実行に委譲。

## Risks

- 非 `.json` テキスト成果物に絶対パスが残る場合は fail-closed
  (現行 analysis 配下に該当なし、gate が検出して build 失敗)。
- JSON parse 不能な将来 artifact は fail-closed で報告される。

## Follow-ups

- AudioLens #139(product-specific metadata projection の分離)は
  将来検討。Ruviform #282 は明示的にスコープ外。
- Ruviform 横断 CI(run #41604498852 と同等 lane)での再確認は
  coordinator が PR/merge 後に観測する。
