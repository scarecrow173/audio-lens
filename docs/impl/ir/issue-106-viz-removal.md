---
id: "IMPL-OLRGo454j948be9Zg2yojU"
type: "impl"
status: "completed"
title: "Issue #106 旧 al-viz / Visualization Dataset / 静的 HTML の撤去と export 分離"
created: "2026-10-31"
updated: "2026-10-31"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/106"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/ruviform-viz-retirement.md"
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

# Issue #106 旧 al-viz / Visualization Dataset / 静的 HTML の撤去と export 分離

- 対象 Issue: audio-lens #106
- 前提: #105(Ruviform webview GUI + `al-service`/`DatasetSession`)、
  #103(Viz 非依存 export 契約 `export_stage_does_not_require_visualization`)
- 関連: #98(golden baseline)、責務マップ
  `docs/designs/ruviform-migration-responsibility-map.md`
- baseline 比較出力: `docs/impl/ir/issue-106-baseline/`
  (`tools/baseline/compare.mjs` + `classify-diffs-106.mjs` の結果)

## 目的

GUI が永続化 Analysis Dataset を直接読む構成(#105)が確立したため、
表示専用の派生 dataset・静的レポート・Visualization Stage を担った
`al-viz` を撤去し、可搬な解析データ export だけを表示責務から分離して
`al-dataset` へ再配置する。

## 棚卸し:`crates/al-viz/` 全ファイル分類

| ファイル | 分類 | 判断 |
| --- | --- | --- |
| `src/stage.rs`(`VisualizationStage`) | 可視化専用 | 削除。`StageKind::Visualization` ごと撤去 |
| `src/stage.rs`(`ExportStage`) | 可搬 export | `al-dataset::export` へ移設(実体は `package.rs` 呼出の薄い stage) |
| `src/package.rs` | 可搬 export | `al-dataset::export::package` へ移設。`analysis/` 複写・入力参照の相対化・`export_manifest.json`・media コピー option・portability 検証を維持。`report.html`/`report.data.js` 生成を除去 |
| `src/golden.rs` | 可搬 export(検証) | `al-dataset::export::golden` へ移設。export パッケージの deterministic snapshot(相対パス→SHA-256 map) |
| `src/report.rs` + `src/report.html` | 可視化専用(静的レポート) | 削除。`include_str!` 埋め込みごと撤去 |
| `src/lod.rs` | 可視化専用 | 削除。LOD 計算は `ruviform-viz`(#157)が担う |
| `src/tiles.rs` | 可視化専用 | 削除。spectrogram tile は `ruviform-viz` が担う |
| `src/timeline.rs` | 可視化専用 | 削除。FrameGrid は `ruviform-viz::timeline` |
| `src/views.rs` | 可視化専用 | 削除。view 合成は `al-service::dataset` + `ruviform-viz` |
| `src/summary.rs` | 可視化専用 | 削除。GUI が dataset から直接表示 |
| `src/lib.rs` | 配線 | crate ごと削除 |
| `tests/viz.rs` | 検証 fixture | export 系を `crates/al-dataset/tests/export.rs` へ移植。viz 生成系は撤去対象のため不移植 |
| `examples/regen_report.rs` | 可視化専用(旧レポート再生成) | 削除 |
| `Cargo.toml` | 配線 | workspace members / deps から除去 |

「解析保存」に分類されるファイルは無かった(dataset 書込は元々
`al-dataset` 所有で、`al-viz` は dataset を読む側のみ)。

## 削除 / 移設の構成

### `StageKind`・パイプライン

- `al-core::types::StageKind::Visualization` と `PIPELINE_ORDER` 内の
  同エントリを削除。`PIPELINE_ORDER` は
  `Ingest → Locate → CheapAnalysis → ReferenceAnalysis →
  ExpensiveAnalysis → Dataset → Export`
- `al-service::pipeline::default_stage_kinds()` は
  `StageKind::PIPELINE_ORDER.to_vec()` に簡素化
- `make_stage` は `StageKind::Export` → `al_dataset::ExportStage`
- `ExportStage::depends_on` は `vec![StageKind::Dataset]` を維持
  (#103 契約テスト `export_stage_does_not_require_visualization` を
  `al-dataset` 側へ移植)
- `--stages` 表記から `visualization` を除去。`parse_stage` は
  `visualization` を拒否(`al-service` テストで担保)

### スタンドアロン export

- `al-job::StageRunner::run_stage` / `run_stage_with_hooks` を追加し、
  plan 済みでない単一 stage を既存ジョブへ実行できるようにした
  (依存 stage の完了記録のみ要求。dataset 完了済みジョブに対し
  export を後から実行できる)
- `al-service::JobService::export(job_id, ctl)` を追加。job 定義を
  読み、`ExportStage` を runner 経由で実行する run/resume と同じ
  headless 経路
- `al export --base-dir <dir> --job-id <JOB-...>` を CLI に追加
  (薄い入口。実装の二重化なし)

### 削除したもの

- `analysis/visualization/` ロール(書込・manifest 役割一覧・spec)
- `report.html`/`report.data.js` 生成と `include_str!` 埋め込み
- `visualization` stage 名の CLI/help/テスト/fixture 参照
- `al-viz` crate 全体と `al-cli` → `al-viz` 依存、workspace 定義

## GUI サービス比較(#105 確立済み経路の確認)

GUI は `al-service::DatasetSession` 経由で永続化 dataset を直接読み、
解析を再実行しない。旧 `al-viz` 生成物との対応:

| 旧 viz 生成物 | 代替経路 | 再計算 |
| --- | --- | --- |
| `visualization/lod`(waveform pyramid) | `al-service::dataset` が `inputs/<id>.wav` を `WaveformSource` 経由で範囲読みし `ruviform_viz::encode_buckets` で実行時 LOD 化 | なし(PCM 範囲読み + min/max 畳込みのみ) |
| `visualization/tiles`(spectrogram) | `intermediate/stft_re|im` の永続化複素 STFT を `SpectrumSource` 経由で tile 読みし `ruviform_viz::Spectrogram::tile` | なし(STFT 再計算なし・量子化のみ) |
| `visualization/timeline` | `ruviform_viz::timeline::FrameGrid` が dataset の timemap/feature 系列から frame 合成 | なし |
| `visualization/views` | `al-service::dataset` の view op が table/tensor/audio range を組立て | なし |
| `visualization/summary`・`report.html` | GUI 画面が manifest・events・features を直接表示 | なし |

`al-service` は `ruviform-viz` の汎用 primitive のみを使い、AudioLens
固有の Visualization Dataset スキーマは Ruviform 側へ漏らしていない
(ruviform リポジトリは無変更)。

## 検証結果

### 単体・統合テスト

- `cargo test -p al-dataset` — export 系 5 件を含む全件 pass。
  `crates/al-dataset/tests/export.rs` で以下を担保:
  - `export_stage_does_not_require_visualization`
    (depends_on == [Dataset] のみ)
  - パッケージ内容:`analysis/` 複写・`export_manifest.json`・
    `golden/snapshot.json`・入力参照の相対化・portability 検証
  - 同一 basename 入力の `media/<input_id>/<file_name>` 退避
  - `export.name` の traversal/絶対/区切り/空名の拒否
  - 絶対パス痕跡の走査(静的 report・`analysis/visualization/` が
    生成されないことの否定確認含む)
- `cargo test -p al-service` / `-p al-cli` / workspace 全体 — 全件 pass。
  CLI テスト `full_pipeline_reaches_export` が
  `ingest,cheap_analysis,dataset,export` の完走と
  `analysis/visualization`・`report.html`・`report.data.js` 非生成を確認

### headless 分離

- `mise run check-cli-headless`:`tools/check-headless-deps.mjs` が
  al-cli/al-core/al-job/al-dataset/al-media/al-locate/al-analysis/
  al-service/al-testkit/al-observability の `cargo tree`(normal+build)
  を denylist 照合 — **pass**(`al-viz` は一覧から除去済み)

### スタンドアロン export 実検証

`target/release/al` で実施:

- `al run --stages ingest,cheap_analysis,dataset` で dataset までの
  ジョブを作成 → `al export --base-dir <dir> --job-id <JOB-...>` が
  `Export: Completed` で完走
- `export/analysis/` に `analysis/` 複写・`export_manifest.json`
  (`generator.id = "al-dataset"`)・`golden/snapshot.json` を確認。
  manifest の `stages` に `export: completed` が追記される
- `al --help` の STAGES 一覧は
  `ingest, locate, cheap_analysis, reference_analysis,
  expensive_analysis, dataset, export`(visualization なし)

### baseline 比較(#98 golden)

- `baseline/runs/<name>`(#98 凍結)を `<name>-golden98` へ退避し、
  本 Issue 変更後コード(`target/release/al`)で `capture.mjs` が
  9 run を再収集、`compare.mjs`(rel 1e-9 / abs 1e-12)で比較後に
  凍結 golden を復元。差分実体は
  `issue-106-baseline/*-vs-golden98.json`、機械分類は
  `tools/baseline/lib/classify-diffs-106.mjs` + `classify.log`
- **全 9 ラン・全差分が既知 3 クラスに完全帰属し、説明不能な
  新規差分は 0 件**:

| run | total | #101 既知 | #100 decoder | #106 期待 | unexplained |
| --- | --- | --- | --- | --- | --- |
| canonical | 1447 | 45 | 30 | 1372 | 0 |
| aa | 1447 | 45 | 30 | 1372 | 0 |
| resume | 1447 | 45 | 30 | 1372 | 0 |
| small | 1833 | 1149 | 40 | 644 | 0 |
| variant-desync | 150 | 20 | 130 | 0 | 0 |
| variant-clipping | 172 | 42 | 130 | 0 | 0 |
| variant-reverb | 161 | 31 | 130 | 0 | 0 |
| variant-filter | 167 | 37 | 130 | 0 | 0 |
| variant-clicks | 181 | 51 | 130 | 0 | 0 |

- `#106 期待` の内訳: `analysis/visualization/` 配下・
  `export/analysis/report.html`/`report.data.js` の path-set 消滅
  (追加 0 件)、manifest `stages` の `visualization` 脱落、
  `export_manifest.json` の `generator.id` 変更(`al-viz` →
  `al-dataset`)と `roles` の `visualization` 脱落、登録 artifact
  `analysis/visualization/summary.json` 除去による正規化
  `ART-#N` 再採番(lineage 参照 -1 シフト、除去件数と一致)
- dataset までで止まる 5 variant run は visualization 成果物を
  元々生成しないため #106 差分 0 件(合計値も #103 時と一致)
- plan 集合検査: stage 集合の差分は `visualization` 除去のみ・
  artifact 追加 0 件・除去は visualization/report のみ(全 run ok)

### GUI 手動検証(長尺サンプル)

手順(`docs/impl/ir/issue-105-gui-app.md` の手順に準拠):

1. 実サンプルで `al run --stages ingest,...,dataset` を実行
2. `al-gui` でジョブを開き、waveform 全体表示 → ズーム →
   spectrogram 表示 → events 一覧を辿る
3. 読込・描画の CPU/メモリ/応答性を計測

計測結果:

- (実測値をここに記入 — 本環境では webview GUI の対話起動が
  不能のため、計測は手順として記録し実測は未実施。
  `DatasetSession` の範囲/tile 読みは `mise run test` の
  service テストで機能的に担保済み)

## 残課題・起票候補

- 長尺実サンプルの GUI CPU/メモリ/応答性の実測:webview が開ける
  環境での手動計測が必要(手順は上記)。定量化ゲートを設けるなら
  別 Issue
- `docs/discovery/remaining-issues-roadmap.md` 等の過去の課題列挙に
  `al-viz` 参照が残るが、発見時点の記録として保持(更新対象外)
- 旧 export パッケージ(`report.html` 同梱)の reader/shim は
  pre-release 規約により不導入
