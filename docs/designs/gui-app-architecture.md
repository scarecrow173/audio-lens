---
id: "DESIGN-035guiAppArch000000000000"
type: "design"
status: "approved"
title: "GUI アプリアーキテクチャ(al-service / al-gui)"
created: "2026-11-27"
updated: "2026-11-27"
owners: ["scarecrow173"]
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
  depends-on:
    - "ruviform-migration-responsibility-map.md"
    - "dataset-schema.md"
    - "job-orchestration.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/gui-app-mvp.md"
    - "../impl/ir/issue-103-job-dataset-provenance-regression.md"
  refines:
    - "ruviform-migration-responsibility-map.md"
  refined-by: []
  derives-from:
    - "../specs/gui-app-mvp.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# GUI アプリアーキテクチャ(al-service / al-gui)

Issue #105 の実装設計。責務マップ
(`ruviform-migration-responsibility-map.md` §GUI 配置)で仮称だった
`al-service` / `al-gui` の境界・op 契約・データ経路を確定する。

## Context

- #103 までに `al-job`(immutable JobDefinition・StageRunner・resume・
  manifest)と `al-dataset`(永続 Analysis Dataset・chunked
  table/tensor/audio・range reader・provenance)が確立済み。
- Ruviform 側は #156(`ruviform-ui` WebView 非依存 host + IPC +
  bounded data、`ruviform-ui-webview` wry/tao adapter)、
  #157(`ruviform-viz` 波形 LOD/スペクトログラム/時間軸
  primitive + `WaveformSource`/`SpectrumSource` port)が
  rev `f3f474b` で利用可能。
- 現行 `al-cli` が stage 構築と `StageRunner` 呼出を直接抱えており、
  「GUI・CLI 同一 Job 層」の要件に対して共有入口が存在しない。

## 方針

```
 al-cli ──┐                  (headless 保証: GUI 依存なし)
          ├─► al-service ──► al-job / al-dataset / ruviform-viz
 al-gui ──┘   (UI非依存)
   │             ▲
   └─ ruviform-ui(Service/UiRuntime/DataRegistry)
   └─ ruviform-ui-webview(webview feature のみ)
```

- **al-service**: AudioLens 所有の headless サービス層。Job 操作
  (create/run/resume/inspect/list/cancel)と Dataset query port
  (overview/timeline/events/locate/features/artifacts/view payload)
  を提供する。`ruviform-viz` の port を `DatasetReader` 上に実装し、
  表示ペイロード(RVBW/RVST)を組み立てる。GUI 依存なし。
- **al-gui**: thin presentation。`ruviform_ui::Service` 実装
  (`AlGuiService`)が op → `al-service` 呼出の変換だけを担い、
  結果 JSON / 登録した `DataSource` 名を返す。デフォルト build は
  headless-safe。`webview` feature で `ruviform-ui-webview` の
  wry/tao host binary を有効化する。
- **al-cli**: 引数解析・ログ初期化・出力整形だけ残し、stage 構築・
  config 解決・runner 呼出は `al-service` に集約。GUI との
  「同一 Job サービス」をコード構造で保証する。
- **al-job**: `StageRunner` に進捗通知・キャンセル hook を追加する
  (progress/cancel は job 実行の関心事であり、所有境界は runner)。
- **al-viz**: 本 Issue では触れない(#106 で撤去)。

## al-service API 概要

### Job 操作

```rust
pub struct JobService { base_dir: PathBuf }

run(spec: RunSpec, ctl: &RunControl) -> Result<Manifest, ServiceError>
resume(job_id: &JobId, ctl: &RunControl) -> Result<Manifest, ServiceError>
inspect(job_id: &JobId) -> Result<Manifest, ServiceError>
list() -> Result<Vec<JobSummary>, ServiceError>   // base_dir/*/manifest.json 走査
```

- `RunSpec` = inputs(role,path,input_id?)・stages・config path —
  `al-cli` の引数と同一概念。`JobDefinition::create` で絶対パス化・
  SHA-256 pin は従来どおり al-job が担う。
- `RunControl` = `{ progress: Option<&dyn Fn(StageEvent)>,
  cancelled: &AtomicBool }`。GUI は `JobContext::cancel_token` を
  `AtomicBool` にブリッジし、`progress` は `JobContext::progress`
  (stage 粒度 done/total)に流す。
- stage 構築(StageKind → `impl Stage`)と config 解決は al-service の
  `pipeline` モジュールに集約(al-cli から移動)。
- `inspect` は `Manifest::load` + dataset 有無・artifact 数の
  サマリを返す。`list` は `base_dir/JOB-*/manifest.json` を走査。

### StageRunner hook(al-job への追加)

```rust
pub struct RunHooks<'a> {
    pub on_stage: Option<&'a dyn Fn(StageEvent)>, // Started/Finished
    pub cancelled: Option<&'a AtomicBool>,         // stage 境界で観測
}
StageRunner::run_with_hooks / resume_with_hooks
```

- `execute` 内の各 stage 起動前に `cancelled` を確認。要求時は未実
  行 stage を Pending のまま manifest を保存し、
  `Err(StageError{cancelled})`相当を返す。キャンセルは失敗
  (`Failed`)として記録**しない** — resume で自然に続行できる
  ことが契約。
- `on_stage` は `Started(stage,index,total)` /
  `Finished(stage,status,duration_ms)` を発火。GUI の進捗粒度は
  stage 単位(サブ stage 内進捗は将来課題)。

### Dataset query port

```rust
pub struct DatasetSession { reader: DatasetReader, root: PathBuf }

open(root_or_job_dir) -> Result<DatasetSession>
overview()      // inputs・roles(produced/count)・manifest 概要
timeline()      // sample_rate・frames・入力別 audio info・event/locate 件数
waveform_view(input_id, channel, start_s, end_s, bins)
                // → ViewBin → RVBW encode (PagedBytes)
spectrogram_tile(pair_or_input, frame_range|time_range, rows, cols, aggregate)
                // → Tile → RVST encode (PagedBytes)
events(start_s, end_s)          // events/candidates table range 走査
locate() / locate_detail(src,cap) // locate/summary + locate/<src>/<cap>.json
features(role_filter, input_id, feature_id_prefix, time_range)
                // features/*.json の records を走査・素の record JSON を返す
artifact(path)  // manifest artifact 参照(schema/shape/provenance)
table_range / tensor_region / audio_info / json // 汎用 range 読み
```

- FeatureRecord は serde JSON を**そのまま透過**し、GUI 側で別表現
  に二度漬けしない(表現ドリフト防止)。
- `events` は table を行範囲で読み、start_s でフィルタ。件数は
  timeline が先に返すので巨大時もページング可能。
- `intermediate/<pair>/stft_re|stft_im` の複素 STFT を
  `SpectrumSource` として開き `Spectrogram::tile` で表示タイル化
  — STFT の再計算(`ruviform_viz::Stft::analyze`)は GUI 経路では
  **使わない**(再解析禁止の要件)。

### DatasetReader 上の port 実装

- `DatasetWaveform`: `inputs/<id>.wav` を `read_audio_range` する
  `WaveformSource`。interleaved `f64` を `PcmBlock` に詰める。
  `WaveformSeries::on_demand`(必要範囲のみ読む)を既定とし、
  `WaveformLod::build` は長尺時の選択肢として保持。
- `DatasetSpectrum`: `intermediate/<pair>` の `stft_re`/`stft_im`
  tensor を `read_tensor` 領域読みする `SpectrumSource`。
  `SpectrumData::Complex{re,im}` を返し magnitude 変換は viz に任
  せる。`FrameGrid` は job config の `analysis.cheap.stft_*` 由来
  (dataset manifest params に残る値)を参照し、欠落時は明示エラー。
- 上記で得た `PagedBytes`(LodBytes/Bytes)は `ruviform_ui::DataSource`
  への薄い adapter で `rvf://data` 登録できる(al-gui 側の接着)。

## al-gui

### 構成

- `crates/al-gui`: lib(`AlGuiService` + 埋め込み frontend assets +
  `PagedBytes→DataSource` adapter)。`webview` feature 時のみ
  `ruviform-ui-webview` に依存する `src/bin/al-gui.rs`。
- frontend: `assets/index.html` + `app.js` + `app.css` を
  `include_str!` で `MapAssetSource` に載せる(追加依存を増やさない)。
- 描画は Canvas2D(RVST tile → `DbRange`+`ColorMap` 合成は Rust 側で
  RGBA 化してもよいが、MVP はフロントで colormap 適用 — 後述)。

### Service op 一覧(rvfc.ipc.v1 上の AudioLens op)

| op | kind | 内容 |
| --- | --- | --- |
| `al.app.info` | call | バージョン・backend 情報 |
| `al.job.list` | call | `JobService::list` |
| `al.job.inspect` | call | `inspect(job_id)` + dataset 有無 |
| `al.job.run` | job | `run(spec)` — progress=stage 粒度、cancel 可 |
| `al.job.resume` | job | `resume(job_id)` 同上 |
| `al.ds.open` | call | dataset root or base_dir+job_id で session 開く |
| `al.ds.overview` | call | overview + timeline |
| `al.ds.events` | call | events range |
| `al.ds.locate` | call | locate summary + detail |
| `al.ds.features` | call | record 検索/一覧 |
| `al.ds.artifacts` | call | artifact ツリー + kind 別 meta |
| `al.ds.read` | call | table/tensor/audio/json の汎用 range 読み |
| `al.view.waveform` | call(job 不要: 有界) | RVBW を `ctx.data()` 登録、source 名+meta 返却 |
| `al.view.spectrogram` | call | RVST 同上 |

`job.cancel` は ruviform-ui 組込みの cancel 経路を使う
(op としては不要)。

### データ転送方針

- JSON 結果は小さく保つ: 波形/スペクトログラムの画素相当データは
  常に `rvf://data` pull(RVBW/RVST、`len` は `DataRegistry::max_chunk`
  で page)。
- セッション内で読んだ `PagedBytes` は `BoundedCache` で key
  (view パラメータ)管理し、同一 viewport の再要求を即応答。

## ライフサイクル・配布・セキュリティ

- **ライフサイクル**: 実行中 job は UiRuntime の worker に存続し、
  ウィンドウを閉じても job は継続(終了時に detach)。
  `has_running_jobs` で終了確認 UI を出せる。キャンセルは stage 境界。
- **オフライン**: 全処理ローカル完結。IPC は同一プロセス内
  (WebView bridge)、外部ネットワーク不要。
- **セキュリティ境界**: op 引数のパスは manifest/dataset の外を読ま
  せない(dataset artifact path は al-dataset が traversal 拒否済み)。
  `ds.open` も指定 root 配下の `manifest.json` 必須で、任意ファイル
  読み出し op は提供しない。フロントは `rvf://app` assets と
  `rvf://data` のみ到達可能。
- **OS 依存 / packaging**: `webview` feature は OS の WebView ランタ
  イムに依存 — Windows=WebView2 (同梱 Runtime 必須)、
  macOS=WKWebView(標準)、Linux=WebKitGTK(dev パッケージ)。
  デフォルト feature は headless のまま `cargo build` が通り、
  `--features webview` で実窓 binary `al-gui` を build。
  パッケージング(インストーラ・署名)は本 Issue のスコープ外で
  ドキュメントのみ(#105 配布節参照)。

## 検証

- headless: `al-service` 単体テスト(合成 dataset → query port)、
  `al-gui` は `UiRuntime::dispatch_op` 駆動の op レベルテスト
  (ruviform の demo_cli と同型)。
- `tools/check-headless-deps.mjs` の `HEADLESS_CRATES` に
  `al-service` を追加。`al-gui` は GUI crate として除外
  (al-viz と同じ扱い。ただし default feature は headless-safe)。
- webview lane: mise に `check-ui-webview` / `lint-ui-webview` /
  `test-ui-webview` を追加(`cargo * -p al-gui --features webview`)。
- 実窓動作は手動検証手順を IR に記録。

## 残課題・Ruviform Issue 起票候補

- stage 内の細粒度 progress(Ingest の decode 進捗等)は hook 未対
  応 — `StageContext` への progress 伝播は別 Issue 検討。
- `ruviform-viz` の colormap→RGBA 化を Rust 側で済ませる op 補助
  (RVST→RGBA 変換)はフロント JS 負荷を減らす改善候補。
- `DataRegistry` に TTL/eviction がなく、session 増で payload が蓄積
  — Ruviform 側の改善提案候補(現状は session 単位で名前要約し
  手動管理)。
