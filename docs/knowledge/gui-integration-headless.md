---
id: "KNOW-ypTKQfVSQnD9tcmquk1nXc"
type: "knowledge"
status: "confirmed"
lifecycle: "current"
knowledge_kind: "architecture"
title: "GUI 統合: ruviform controls/viz・SemanticGuiTest・実 OS E2E"
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

# GUI 統合: ruviform controls/viz・SemanticGuiTest・実 OS E2E

`al-gui`/`al-service`/`ui/` は Ruviform の UI 基盤(ui/core・ui/components・ui/binding・ui/viz・ruviform-ui-gpui・ruviform-ui-webview)を consumer として統合する層である。本書は headless 統合コア、driver-neutral scenario 検証、実 OS E2E lane、consumer oracle の設計をまとめる。

## al-service / al-gui の責務分割(AL#105)

- `al-service` は UI 非依存:`pipeline.rs` が al-cli の stage 構築/config 解決/辞書スナップショット/入力・stage 名解釈を移設し正本化、`jobs.rs` が `JobService`(run/resume/inspect/list/open_dataset)+`RunControl`+`ServiceError` 分類、`dataset.rs` の `DatasetSession` は `TableReader::read_range`/`TensorReader::read_range`/`read_audio_range` のみで**再計算しない**、`source.rs` が `DatasetWaveform`/`DatasetSpectrum`。
- `al-job` の `RunHooks`(`on_stage`/`is_cancelled`)+`run_with_hooks`/`resume_with_hooks` で、キャンセルは次 stage 起動前に判定し Pending のまま manifest 保存(stage を Failed にしない)。既存 `run`/`resume` は no-op hooks へ委譲し後方互換。
- `al-gui` の `service.rs` が `rvf.ipc.v1` op 群(`al.app.*`/`al.job.*`/`al.ds.*`/`al.view.*`)を提供し、長時間処理は `JobSpec` 非同期化+`JobContext::progress/is_cancelled`→`RunControl` 橋渡し。`al.view.*` は RVBW/RVST エンコード済み payload を `DataRegistry` へ登録し `{source, bytes, meta}` を返し、frontend は `rvf://data` から有界 pull(全量 JSON 埋め込みしない)。

## headless 統合コア(AL#131/#132)

`ui/` は framework-neutral TS パッケージ(`@audiolens/gui-headless`)。`ops.ts` が `service.rs` の TS ミラーで、`port.ts` が `AlService`/`AlJobHandle` を定義する。`view-state.ts` の `RevisionedStore<P>` は単調増加 revision で subscribe 時即時 push、rev 逆行の stale projection を破棄する。

`gateway.ts`(`AlGuiGateway`,BindingPort 実装)は `dispatchFor(intent,ctx,ack)` が per-binding ack closure へ receipt を配線する。missing source→error / stale revision→stale / unsupported→unsupported / 例外→error を区別し**確定値を偽装しない**。Query sourceId は `waveform:<session>:<role>:<name>` 等の grammar で、数値 segment は canonical 10 進 `^\d+$` のみ(`""`/`0x1`/`3.0`/`1e1` は拒否)。`AbortSignal` と `cancelQuery` の双方で cancel し、byte 超過は `over-budget`。

`decode.ts` の strict decoder: RVBW = 16B header + 48B/bin(upstream doc 表記「44B」は stale で実 wire は `count` を `+40..+48` に書く 48B、crate 側 layout test を正本に確認)、RVST = 24B header + row-major `rows×cols`(row 0=最低 Hz)。magic/version/dtype/長さ不整合は例外で、破損を空データに畳まない。`axis` 必須 —— 非周波数軸を Hz と偽らない。

## SemanticGuiTest と driver-neutral scenario

`SemanticGuiTest` scenario は `tests/common/scenario.rs` に 1 箇所に置き、`tests/scenario.rs`(Fake、headless)と `tests/scenario_gpui.rs`(GPUI、`gpui-test` feature)で**同一関数**を実行する構成。この「shared scenario を複数 driver で同一実行」パターンが driver-neutral 検証の基盤である(RF#464 Step C で採用)。

重要な配置判断: driver snapshot が観測できない内部状態(typed table cell)は「mount される spec 投影」を oracle にする —— spec 側不一致は mount 後表示にも現れるため、port 非依存かつ表示に直結する最も正直な oracle 位置となる。破壊的 case(window close)は catalog 末尾に配置し、観測不能は `Fail` でなく `NotRun`(前提不成立を区別)。

## 実 OS E2E lane(AL#219)

`gpui-real` feature + test-only bin `al-gui-real-scenario` で製品 `ViewSpec` を実 OS window へ in-process mount し、driver-neutral scenario・実 OS 入力・実 pixel capture を JSONL 証拠化する opt-in lane。

- `gpui-test`/`gpui-real` 同居は bin 内 `compile_error!` で排他。
- `RealOsGuiTest::run` で 1 session・2 window(desktop→source)、catalog 7→9 case(semantic 2・OS input 3・visual 2 + dataset_workspace・window_close)を `run_scenario`/`ScenarioSpec` で実行。
- verdict 写像: `OsInputOutcome`/`CaptureOutcome` の `EnvironmentBlocked`→`NotRun`+`environment_blocked`、`Unsupported`→`Unsupported`、`Injected`/`Captured` のみ oracle 判定へ進む gate。`environment_kind` を `real-window`(live)/`deterministic-backend`(blocked)で分離し、blocked 記録が実環境を主張しない。
- headless/非対話 host では `UiError::Host`→全件 `NotRun` が正しい結果(fail-closed だが環境依存を区別)。

## consumer oracle(AL#127/RF#628)

`ruviform-audio-test` の deterministic PCM fixture/oracle を file-based 解析の独立 consumer oracle として `crates/al-analysis` に実装した。`tests/rf628_oracle.rs`(~730 行、7 test)に AL-1(f64 fidelity)・AL-2(長尺非回帰)・AL-3(gap/Validity)を集約。

設計上の決まりごと: 期待値は 2 系統 —— (a) `oracle::pcm_reference_block` 出力の f64 widen 統計量(`level_expect` で dc/peak/rms/4x true-peak 再実装)、(b) 解析的参照(constant 厳密値・sine/square tonal 周波数・segment 幾何由来 validity map)。`pcm_assert_eq` は fixture 入力の妥当性確認限定で Pass 判定には使わない。AL-2 規模は実測で 30s(3×10s)に決定(支配コストは locate 相互相関でほぼ線形、30s=1.44M frames で trajectory 2999 windows)。AL-4(clock drift)Deferred、AL-5(device-pipeline legs)NotApplicable。

NaN セマンティクスの pin: `level.dc_offset` NaN→JSON `null`(record Valid のまま)、`rms_dbfs`/`integrated_dbfs` NaN→`db()` floor -240.0、`trajectory` は NaN 窓で NaN、`crest_factor` rms_all NaN で 0.0。

## 設計上の教訓

- upstream doc 表記と実 wire の齟齬(44B vs 48B)は crate 側 layout test を正本に確認する。
- 「写像不能は `undefined` で返し捏造しない」「`axis` 無し record は spectrum を供給しない」— 能力欠如を値の捏造で埋めない。
- vendored upstream ソースは「commit せず pin から再生成 + `--check` で REV 整合 + `.worktreeinclude` で引継ぎ」する vendoring パターン。
- delayed/reordered/duplicate receipt、ack timeout、detach/remount、2-window 転送(Job cancel しない・subscription 解放・attach 後 authoritative 再読)を契約どおり検証する観点表を作る。
- cancel 済み run が job_id を返さない場合 `al.job.list` で再発見する前提を port に持たせる。

関連 Issue/PR: AL#105、AL#127、AL#131、AL#132、AL#179、AL#180、AL#219、RF#314、RF#453、RF#458、RF#461、RF#463、RF#464、RF#628、RF#631、RF#709、ruviform PR #466、AL PR #168、#190、#193、#194、#213、#215、#216。
