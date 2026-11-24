---
id: "EVID-jP7nWvXsQbZkMfRhYdGt"
type: "evidence"
status: "approved"
title: "検証記録: GUI — headless・parity・実 OS lane・consumer oracle"
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

# 検証記録: GUI

`al-gui`/`al-service`/`ui/` の GUI 統合・parity 検証・実 OS lane・consumer oracle に関する事実記録。
機能 × lane の個別判定の対応表は `docs/specs/issue-189-native-gui-feature-matrix.md`（Issue #189 AC2）を参照。

## headless 統合と service_ops

- `tests/service_ops.rs` 5 test: `sys.capabilities`→`al.app.configure`→`al.job.run`(JobSpec→worker→progress→Finished with job_id)→`al.job.inspect`(dataset_ready=true)→`al.ds.open`/`overview`/`features`(validity 保持)/`al.view.waveform`→`rvf://data` 有界 read が RVBW 先頭。エラー経路 INVALID_PARAMS・missing dataset・UNKNOWN_JOB。
- `al-service/tests/service.rs`: run→inspect→list、cancel Pending 保持→resume 完走、corrupt manifest は inspect 失敗だが list に要約残存、corrupt dataset manifest→io。
- `al-job/tests/hooks.rs`: stage 境界イベント・cancel で Pending 保持・resume で Reused。
- headless 依存検査 `tools/check-headless-deps.mjs`: al-service を headless 対象追加、al-gui は GUI crate で対象外。denylist 照合で headless 10 crate に GUI 依存なしを確認。

## driver parity・receipt

- `tests/desktop.rs` + `tests/receipt.rs`: resume の ack(`accepted`、status なし)と job 終端の 2 回目 `result`(完了は `applied` かつ job event の後、失敗は `ok:false`)。Run tab mount で Resume Button の phase が `Pending`→`Accepted`→`Applied`、同一 spec 再射影で Run tab と phase 保持、`active` が Jobs に戻る既知挙動。
- parity は mount key・component projection(mounted/phase/display/committed、f64 は bit 比較)・Seek gesture の command intent JSON を比較。
- Tabs pointer 切替の receipt revert 検証は `receipt_statuses` で `rejected` の到着を先に確認する手法。

## scenario 検証

- `tests/scenario.rs`(Fake、headless)+`tests/scenario_gpui.rs`(GPUI、`gpui-test`)で同一 scenario 関数を実行。GPUI adapter 非同期 lifecycle は `settle()` を mount/scenario 後に挿入。
- `fake_dataset_workspace`/`gpui_dataset_workspace`/`scenario_real` 5 PASS(blocked-probe fail-closed・catalog pin 一致)。

## 実 OS E2E lane(AL#219)

- `test-ui-gpui-real-headless` exit 0: 全 7 case `not_run`+`environment_blocked`、`environment_kind=deterministic-backend`。
- `test-ui-gpui-real` exit 0: interactive Windows session で全 7 case `pass`、`environment_kind=real-window`(証拠 `evidence/al219/gui-real-scenario.jsonl`/`gui-real-headless.jsonl`)。pointer は win32-SendInput、capture は win32-printwindow backend。
- `issue-219-gui-real-e2e-exp.jsonl`(7 行): in-process mount 採用・`#[path]` 取込み・verdict 写像 gate の decision と headless/live 両 lane の validation を時系列記録。

## consumer oracle(AL#127/RF#628)

- `cargo test --locked -p al-analysis --test rf628_oracle`: 7/7 Pass(Windows debug)。`rf628_long_scenario_non_regression` ~85s、他 ~2s。
- `issue-127-rf628-long-scenario-scale.jsonl`: 60s(3×20s)=総150s、45s=総130s、30s(3×10s)=総~85s。支配コストは locate 相互相関でほぼ線形、30s 採用。
- NaN pin 実測: `level.dc_offset` NaN→JSON `null`、`rms_dbfs`/`integrated_dbfs` NaN→floor -240.0、`trajectory` NaN 窓で NaN。

## 実窓検証(AL#179/#180/RF#463)

- canvas 0 サイズ修正後の実窓 pixel 解析(pin `863a7a9d`): Waveform envelope・Meter bars・TimeAxis tick 9 本・Spectrogram 65,536 cell の非空 paint を確認。before(旧 pin)は blue-family 17 色・飽和ペア 100.0%、after は blue-family 179 色・旧飽和ペア 0.2%。
- spectrogram tab hang(pin `863a7a9d` で解消): 実機で tab 切替が即時完了、`Responding=True`・CPU almost idle。`spectrogram_tab_paint` 0.13s pass(本番解像度 65,536 cell)。
- `spectro-intensity.py` pixel 解析: blue luminance 56–164、飽和ペア 0.2%。

## 未検証・制約

- 実 OS window の目視確認は大部分の slice で NotRun(spectrogram-tab-hang と canvas-paint-empty は実機検証済み)。
- `check-ui-driver-boundary`・`test-ui-parity` は mise.toml に存在しない task で常に NotRun。
- `Cancelled` の resume job 終端 receipt は ruviform の分類依存で AL 側直接検証未実施。
- workspace Tabs 非採用のため Dataset 内 Source・Spectrogram・Analysis tab は pointer で到達不能(P2 既知残課題、受入欠落ではなく非スコープ)。
- drawn plan に Spectrogram・Label・Table の variant が無く drawn parity の対象外(RF#442)。
