# Issue #105 Ruviform GUI host 上の AudioLens GUI 実装記録

- 対象 Issue: audio-lens #105
- 画面・操作仕様: `docs/specs/gui-app-mvp.md`
- アーキテクチャ設計: `docs/designs/gui-app-architecture.md`
- 関連: #97(責務仕様)/ #103(UI 非依存 reader 契約)/ Ruviform #156(`ruviform-ui`/`ruviform-ui-webview`)/ #157(`ruviform-viz`)
- Ruviform 依存 rev: `f3f474b`

## 構築した構成

```
al-cli (bin) ─┐
              ├─→ al-service ─→ al-job / al-dataset / al-media / al-locate / al-analysis / al-viz
al-gui ───────┘                    │
  ├─ service.rs (rvf.ipc.v1 op)    └─ ruviform-viz: WaveformSource / SpectrumSource / WaveformSeries / Spectrogram
  ├─ assets/ (rvf://app)           ruviform-ui: Service / OpInfo / CallOutcome / JobSpec / UiRuntime
  └─ bin/al-gui (webview feature)  ruviform-ui-webview: HostConfig / run / detect_backend (feature 分離)
```

- `al-job`: `RunHooks`(`on_stage`/`is_cancelled`)と `run_with_hooks`/`resume_with_hooks` を追加。
  `StageEvent`{stage,index,total,kind=Started|Completed|Reused|Failed} を stage 境界で発火。
  キャンセルは次 stage 起動前に判定し、Pending のまま manifest 保存して中断する
  (stage を Failed にしない)。既存 `run`/`resume` は no-op hooks へ委譲し後方互換。
- `al-service` (新規): UI 非依存サービス層。
  - `pipeline.rs`: `al-cli` が持っていた stage 構築/config 解決/dictionary
    スナップショット/input・stage 名解釈を移設し正本化。
  - `jobs.rs`: `JobService`(run/resume/inspect/list/open_dataset)と
    `RunControl`(進捗・キャンセル橋渡し)、`ServiceError` の分類
    (invalid/not_found/io/stage/cancelled)。
  - `dataset.rs`: `DatasetSession` — manifest-driven な overview/timeline/
    events/locate/features/artifacts/table_rows/tensor_region/json_artifact/
    waveform_view/spectrogram_tile。`TableReader::read_range`・
    `TensorReader::read_range`・`read_audio_range` のみ使用し再計算しない。
  - `source.rs`: `DatasetWaveform`(`inputs/<id>.wav` → `WaveformSource`)、
    `DatasetSpectrum`(`intermediate/<input>/ch<N>/stft_re|stft_im` 永続複素
    STFT → `SpectrumSource`)。STFT の再計算は行わない。
- `al-cli`: binary 本体を `JobService` 経由に再配線(`resolve_config`/
  `make_stage`/`parse_stage`/`parse_input` を al-service へ移設して削除)。
  CLI が生成する job・manifest・dataset は GUI と完全に同一。
- `al-gui` (新規): thin presentation 層。
  - `service.rs`: `AlGuiService` — `rvf.ipc.v1` op 群
    (`al.app.info|configure`, `al.job.list|inspect|run|resume`,
    `al.ds.open|close|overview|events|locate|locate_detail|features|
    artifacts|artifact|read`, `al.view.waveform|spectrogram`)。
    長時間処理は `JobSpec` で非同期化し `JobContext::progress/is_cancelled`
    を `RunControl` へ橋渡し。`al.view.*` は RVBW/RVST エンコード済み
    payload を `DataRegistry` へ登録し `{source, bytes, meta}` を返す —
    frontend は `rvf://data` から有界 pull する(全量 JSON 埋め込みをしない)。
  - `assets/`: AudioLens 所有の frontend(index.html/app.js/app.css)。
    `MapAssetSource` + `include_bytes!` で `rvf://app` へ供給。
  - `src/bin/al-gui.rs`: `webview` feature 時のみビルドされる
    `ruviform_ui_webview` host。`detect_backend` で runtime 診断し、
    `HostConfig` + `run` で実窓を起動。
- `tools/check-headless-deps.mjs`: `al-service` を headless 対象に追加
  (al-gui は GUI crate として対象外 — al-viz と同じ扱い)。
- `mise.toml`: `check-ui-webview` / `lint-ui-webview` / `test-ui-webview`
  lane を追加(ruviform 側同名 task と同形。既定 check/lint/test は
  headless のまま)。

## MVP で検証した操作・表示(headless)

`crates/al-gui/tests/service_ops.rs` が `UiRuntime::dispatch_op` で
フロントエンドと同じ intent 経路を検証:

- `sys.capabilities` → `al.*` op 一覧露出
- `al.app.configure` → `al.job.run`(JobSpec → worker thread →
  progress events → Finished with job_id)→ `al.job.inspect`
  (dataset_ready=true)
- `al.ds.open` → `al.ds.overview`(timeline: input 数・sample_rate)→
  `al.ds.features`(全 record が `validity` を保持)→
  `al.view.waveform` → `rvf://data` 有界 read が `RVBW` 先頭を返す
- エラー経路: INVALID_PARAMS 写像、missing dataset → error result、
  `job.cancel` 未実行 job → UNKNOWN_JOB

`crates/al-service/tests/service.rs` がサービス層契約を検証:

- run→inspect→list、cancellation(Pending 保持)→resume 完走、
  dataset open 後の features(validity 保持)・artifacts・
  waveform_view(RVBW・有界)・timeline
- 失敗分類: missing job → not_found、corrupt manifest → inspect は
  失敗するが list には要約として残る、corrupt dataset manifest → io
- `parse_stage`/`parse_input_spec`/`assign_input_ids` 規則

`crates/al-job/tests/hooks.rs` が `RunHooks` を検証: stage 境界
イベント(Started/Completed、index/total)、キャンセルで次 stage 未起動
+ Pending 保持、resume で完了済み stage が Reused。

## 検証結果(実行済み)

| task | 結果 |
| --- | --- |
| `mise run fmt` (`cargo fmt --check`) | pass |
| `mise run check` | pass |
| `mise run lint` (biome + clippy -D warnings) | pass |
| `mise run test` (`cargo test --workspace`) | 全 crate pass(新規 +18 test 含む) |
| `mise run check-cli-headless` | pass — 10 headless crate に GUI framework 依存なし |
| `mise run check-ui-webview` | pass(webview feature + bin target) |
| `mise run lint-ui-webview` | pass |
| `mise run test-ui-webview` | pass(service_ops 5 test) |

## 実装中に見つけて直した問題

- `DatasetSession::artifact_detail` が `path.split_once('/')` で
  role/name を分割していたが、dataset role は `features/cheap` の
  ように `/` を含むため tensor/table artifact で name 解決が壊れた。
  manifest の `artifact.role` を正本に name を剥がすよう修正
  (service.rs の `al.ds.read` は role/name 分離パラメータなので影響なし)。
- ingest `metadata.json` の audio 情報は `audio.{sample_rate,channels}`
  にネストされている(最初トップレベルで読んでいた)。
- `source_reference` は `InputRole` に存在しない(正:
  `source_asset`。frontend の role 選択肢も修正)。

## 手動検証手順(実窓)

```sh
# Windows: WebView2 Runtime、macOS: WKWebView、Linux: WebKitGTK が必要
cargo run -p al-gui --features webview -- <jobs_base_dir>
```

1. `al.app.info` がタイトルバー下の status に表示されること
2. base_dir 設定 → 入力行追加(role + path 実ファイル)→ Run で
   progress bar が進み `job finished` が log に出ること
3. job 一覧クリック → inspect(stage 状態一覧)→ dataset 自動 open →
   波形・スペクトログラム(永続 STFT ある場合)・timeline が描画されること
4. timeline クリックで viewport センタリング、ズーム in/out、
   canvas 下の artifact ツリー → 詳細パネルに validity/confidence/
   provenance を含む JSON が出ること
5. 実行中 job の Cancel で中断 → job が Pending 残しで一覧に残り、
   再 Run(resume 相当 flow)で完走できること

## 残課題・制限

- **video 情報の表示**: `inputs` 系の video frame/timestamp artifact は
  dataset 側で `al.ds.read`/`al.ds.artifact` 経由で取得可能だが、
  frontend の描画パネル未接続(#105 の MVP 範囲では time map は
  locate alignment 区間として timeline に描画済み)。
- **spectrogram 前提**: `DatasetSpectrum` は persist 済み複素 STFT
  (`intermediate/<input>/ch<N>/stft_re|stft_im`)を読む。STFT を
  persist しない入力/stage 構成ではタイル生成は `not_found` となり
  frontend では非表示(再計算しない規約どおり)。
- **residual audio / locate detail の UI 未接続**: `al.ds.read`
  (kind=audio)・`al.ds.locate_detail` op は提供済みだが frontend の
  専用ビューは未実装(artifact ツリー経由で JSON 参照は可能)。
- **event 大量時**: `al.ds.events` は行範囲 read 対応だが frontend は
  全件 marker 描画。長尺では行範囲絞込が必要。
- **payload lifecycle**: `al.view.*` が登録する `DataRegistry` 名は
  連番で増殖し、session 中の古い payload を unregister する経路が
  ruviform-ui に無い(下記 Issue 案 1)。
- **マルチチャネル**: `al.view.*` は channel 指定 1ch 表示のみ。

## Ruviform Issue 起票案

1. **DataRegistry の eviction/unregister 契約**: view op が返す
   `{source}` 名を session 内で増殖させないための `unregister` または
   generation/LRU 上限の提供。AudioLens 側では現状 `al/<tag>/<n>` の
   連番名で回避しており、長時間 session でメモリが単調増加しうる。
2. **Job progress の detail チャネル**: `JobContext::progress(done,total)`
   は件数のみで「どの stage/処理中か」を運べない。label 相当の
   detail 文字列を進捗に添える API があると、stage 進捗の表示名を
   追加 op なしで運べる(現状は total=stage 数の done のみ)。
3. **`rvf://data` の paging 契約の文書化**: per-request cap と
   `X-Rvf-Total` による全長取得は実装依存仕様で、frontend は
   paging loop を書く必要がある。契約として crate 文書/README に
   明文化されると利用側の実装ミスを防げる。
