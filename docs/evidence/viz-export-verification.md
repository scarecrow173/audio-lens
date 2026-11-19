---
id: "EVID-rW2vKxTjYzNbLpQcHgSm"
type: "evidence"
status: "approved"
title: "検証記録: 可視化・export・golden"
created: "2026-11-19"
updated: "2026-11-19"
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
  implements: []
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
---

# 検証記録: 可視化・export・golden

`al-viz`/`al-dataset::export` の可視化製品・export パッケージ・golden 検証に関する事実記録。

## 可視化製品の検証

- `cargo test -p al-viz`/`cargo test --workspace`: timeline/LOD/tiles/views/summary/report の生成と `analysis/visualization/` 配下の role 置換を検証。`skipped_products` の「必須だが未生成」記録と「生成されたが期待 artifact 無し」の明示エラーを分離確認。
- `timeline.json` が最初に書かれ units/window/hop 規約を確定することを検証。
- 決定性契約: ファイル内容は dataset・schema version・config から決定的で、生成 artifact ID は対象外。

## export パッケージ検証

- `cargo test -p al-dataset`(export 系 5 件): `depends_on`、パッケージ内容、同一 basename 退避、`export.name` の traversal/絶対/区切り/空名拒否、絶対パス痕跡走査を検証。
- `full_pipeline_reaches_export` で `ingest,cheap_analysis,dataset,export` 完走と visualization/report 非生成を確認。
- スタンドアロン `al export`: `al run --stages ingest,cheap_analysis,dataset` → `al export` が `Export: Completed` 完走、`export/analysis/`+`export_manifest.json`(`generator.id = "al-dataset"`)+`golden/snapshot.json` 生成、manifest `stages` に `export: completed` 追記。

## golden 検証

- `golden/snapshot.json`(相対パス→SHA-256)方式で export artifact の内容照合を固定。
- `cargo test -p al-dataset` export 系全 pass、`cargo test -p al-viz`(撤去前)・`cargo test -p al-cli`・workspace 全 pass。

## Ruviform 移行後の検証(AL#106)

- `cargo test -p al-dataset`(export 系含む全 pass)、`-p al-service`、`-p al-cli`、workspace 全 pass。
- `check-cli-headless`: 10 headless crate の cargo tree denylist 照合 pass(al-viz は一覧から除去)。
- baseline 比較(9 run): 全差分が既知 3 クラス(#101/#100/#106 期待)に帰属・unexplained 0。`analysis/visualization/` 配下・`report.html`/`report.data.js` の path-set 消滅、manifest `stages` の visualization 脱落、`generator.id` の `al-viz`→`al-dataset`、roles の visualization 脱落、`summary.json` 除去による `ART-#N` 再採番(lineage 参照 -1 シフト)。

## 未検証・制約

- GUI 長尺サンプルの CPU/メモリ/応答性は未計測(定量化ゲートを設けるなら別 Issue)。
- GUI 手動検証は未実施(当時 webview GUI の対話起動が不能、手順のみ記録)。
- `--stages visualization` 拒否は破壊的仕様変更(pre-release 規約で互換層なし)。
