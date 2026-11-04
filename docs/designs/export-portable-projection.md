---
id: "DESIGN-034Rp43HK02Vpr5d3rbv2b"
type: "design"
status: "approved"
title: "Export 可搬投影と staging publish"
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
  implements: []
  implemented-by: []
  depends-on:
    - "docs/specs/export-artifact-portability.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "dataset-schema.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/export-artifact-portability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Export 可搬投影と staging publish

## Context

`package::build` は `analysis/` を `export/<name>/analysis/` へ
全コピーし、`manifest.json` の `inputs[].source_path` と `config`
のみを縮約していた。`declared.json` と `<id>.metadata.json` は
`source_path` を生のまま保持し、portability gate は絶対パスの
`/` 正規化 spelling のみを検査していた。結果として Ubuntu では
gate が検出して fail-closed 失敗、Windows では JSON エスケープ
された `\\` spelling が gate をすり抜けて非可搬パッケージが
公開されていた。

## Scope

### In Scope

- `dest/analysis/` 配下の全 `.json` 成果物への一括投影パス
- 絶対パス判定の 2 セパレータ対応(POSIX `/`・`\`・drive-letter
  `X:`・UNC `\\`)と basename 射影
- Gate の検査 spelling を `/`・`\`・`\\`(JSON escape)に拡張
- `export/.<name>.partial` staging → gate 通過後 rename publish
- 失敗時の staging 撤去と、成功済みパッケージの保存

### Out of Scope

- 元 `analysis/` 成果物の変更(projection はコピー側のみ)
- `.json` 以外のテキスト成果物の構造変換
- 元 manifest の schema・golden snapshot 形式の変更
- copy_media のバイト完全一致要件の緩和

## Components and Boundaries

- `shrink_abs_paths`: JSON 値木を再帰走査し、`looks_absolute`
  な文字列値を `base_name` へ置換。変更検出を返し、変更の無い
  ファイルは再シリアライズしない(byte 不変)。
- `sanitize_json_tree`: `dest/analysis/` 配下の全 `.json` を
  parse → shrink → 変更時のみ上書き。parse 不能な `.json` は
  `InvalidData` で fail-closed(ファイル相対パスを診断に含める)。
- `path_spellings`: 禁止パスの `/`・`\`・`\\`(JSON escape)の
  3 spelling を生成し、gate が全表現で一致検査する。
- `build` / `build_package` 分離: `build` は staging 名の確保・
  publish(rename)・失敗時 cleanup だけを担い、`build_package`
  はコピー・投影・media コピー・export_manifest・gate を担う。
  失敗時 `build` が staging を撤去するため、`export/<name>/` は
  「gate 通過済み完成品」だけを指す。

## Data and Control Flow

1. `validate_package_name` → `analysis/manifest.json` 存在確認
2. `export/.<name>.partial` を確保(残存物があれば撤去)
3. `copy_tree` で `analysis/` を staging へコピー
4. `manifest.json` を読み、input_records(input_id / role /
   file_name / media_path / sha256)を構築 — file_name は
   `base_name(source_path)`(ホスト規約非依存)
5. `sanitize_json_tree` で staging 内全 `.json` に投影を適用
   (`manifest.json` も含めて一括 — `source_path`・`config`・
   `declared.json`・`*.metadata.json`・将来成果物すべて)
6. `copy_media` 時は `media/<input_id>/<file_name>` へコピー
   (file_name も `base_name` で導出、sha256 はソースと一致)
7. `export_manifest.json` を staging 直下へ書き出し
8. Gate: `manifest.inputs[].path` と `job_dir` の絶対パスを
   `path_spellings` で展開し、パッケージ内全テキスト
   (`.json`/`.txt`/`.js`/`.html`)を走査。一致があれば
   `InvalidData` で失敗
9. 全過程が成功したら `export/<name>/` を撤去して staging を
   `rename` で publish。失敗時は staging を撤去して error 返却

## Non-Functional Constraints

- `fs::rename` は同一 FS 内の原子的差し替え(staging は
  `export/` 直下の sibling)
- Windows では dest 既存時に先に `remove_dir_all` が必要
  (rename の前に空にする)
- 投影は JSON 値のみを対象とし、キー・数値・bool・null は
  不変。変更の無い `.json` は byte 単位で保持
- Gate は private path の spot-check であり、投影は全絶対パス
  らしい文字列を対象とする(将来成果物・未宣言パスも縮約)

## Risks and Trade-offs

- 絶対パスらしい非パス文字列(例: 説明文 `/usr/lib` や URL 断片)
  も basename 化される — 既存 `config` 縮約と同じトレードオフ。
  実データセットの writer が生成する値に該当文字列はなく、
  漏洩公開より誤縮約の方が安全側。
- `.json` 成果物の数値は serde_json 標準の f64/i64 経由で
  再シリアライズされる — 値は保持されるが表記は正規化され得る
  (`8192.0`→`8192.0`、指数表記の整形)。byte 変化は変更のあった
  ファイルに限定される。
- staging sibling は `export/` 直下に置く — 同一 FS 内 rename
  を保証し、`export/<name>` 外部へは書き出さない。
- `manifest.json` の入力 source_path が空文字・非文字列の端
  ケースは `unwrap_or("")` で file_name `""` へ縮退 — 従来通り
  明示的失敗はしないが、input_id は保持される。

## Deferred Design Concerns

- `.json` 成果物に「絶対パスらしい非パス文字列」が混じる場合の
  フィールドスコープ化 | 現状は汎用規則で十分 | 誤縮約が実害と
  判明した場合 | 値が壊れる副作用を気付かず使い続けるリスク
- `copy_media` 対象が `.json` 拡張を持つ media の場合、media 側は
  byte 完全一致で保存し、投影は `analysis/` 側に限定 — gate が
  `media/**/*.json` の private path を検査して fail-closed に
  なるため、該当入力の export は明示失敗する(元データ破壊より
  優先)

## Verification Notes

- [x] `export_sanitizes_dataset_json_artifacts` で
      metadata/declared/manifest の投影と provenance/digest/
      stream 保持を確認
- [x] `export_fails_closed_when_a_text_artifact_cannot_be_sanitized`
      で `.txt` 漏洩時の fail-closed・staging 撤去・元データ
      保存を確認
- [x] `export_rerun_failure_keeps_the_last_verified_package` で
      再実行失敗時に公開済みパッケージが保存されることを確認
- [x] ユニットテストで POSIX/Windows/UNC/エスケープ/Unicode/
      相対パスの判定を網羅
