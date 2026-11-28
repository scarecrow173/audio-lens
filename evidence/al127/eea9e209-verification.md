# ruviform pin `eea9e209` 追従検証記録（AL#127 pin 追従 slice）

- 実施日: 2026-11-28
- 対象: `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` →
  `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`（dispatch 時 upstream
  main、69 commits・483 files）
- candidate tuple: ruviform
  `eea9e2090a23e24dd5cd7a0799b25175c91cad9c` / AL pin bump commit
  `4c8f2b7f9702ea4bc9a6bc01b4168e34719696a2` / resolved
  `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`
- 判定: **採用**（rollback 不要 — pin 一貫性 lane と canonical 回帰 lane が
  全て exit 0）
- 環境: Windows（MINGW64_NT-10.0-26300 x86_64）、rustc/cargo 1.99.1、
  Node.js v26.12.1、npm 11.20.0、mise 2026.11.2 windows-x64
- AL 基点: `origin/main` `0f1a1aac86336bff98df8bab1162e23bf5c36672`。
  最終確認時も同 SHA で、取り込み差分はなかった。

## pin 一貫性

| lane | 結果 |
| --- | --- |
| `git ls-remote https://github.com/scarecrow173/ruviform.git refs/heads/main` | dispatch 時・最終確認時とも `eea9e2090a23e24dd5cd7a0799b25175c91cad9c`（candidate drift なし） |
| `node tools/update-ruviform-rev.mjs eea9e2090a23e24dd5cd7a0799b25175c91cad9c` | exit 0（25 package を単一 SHA に原子解決） |
| `mise run setup-ui` | exit 0（fresh worktree の npm 依存復元） |
| `mise run sync-ui-deps` | exit 0（vendor REV = `eea9e209…`、生成物は commit 対象外） |
| `mise exec -- cargo fetch --locked` | exit 0 |
| `mise run check-ruviform-lock` | exit 0 — `eea9e209…`、25 packages |
| `mise run check-ruviform-rev` | exit 0 — resolved `eea9e209…`、lockfile-primary、REV match |
| `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor` | exit 0 — 同上 |
| `Cargo.lock` 差分 | 25 個の ruviform source fragment のみ。`[[package]]` 追加/削除 0、無関係 version bump なし、upstream crate manifest diff なし |
| manifest 非変更 | ルート/各 crate の `Cargo.toml` に `rev`/`branch`/`tag` を追加せず |

`gpui` 0.2.2 は現行 lock で既に `scarecrow173/gpui` tag `v0.2.2-rf2`
（peeled commit `b3144f286b942fbb5f478da73db13a826cef054a`）を使っており、
本 slice の新規 source switch ではない。

## 差分影響（`87a7ba8f..eea9e209`、local clone read-only 実測）

- merge-base は旧 pin `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` で、
  candidate は旧 pin の descendant（69 commits ahead）。
- 全体: 483 files（+11,813,666/−1,509）。大半は upstream evidence と
  検証資産。evidence/.agent-skills を除く非 evidence は 54 files
  （+16,133/−1,480）、`crates/` は 26 files（+12,683/−1,332）。
- production `src/` 差分なし: `ruviform-audio-analysis`/
  `ruviform-audio-core`/`ruviform-dsp`/`ruviform-exec`/`ruviform-graph`/
  `ruviform-io-avi`/`ruviform-io-file`/`ruviform-io-symphonia`/
  `ruviform-math`/`ruviform-media-io`/`ruviform-numeric`/
  `ruviform-observability`/`ruviform-plugin-host`/`ruviform-rt`/
  `ruviform-ui-gpui`/`ruviform-ui-theme`/`ruviform-video-core`/`ruviform-viz`。
- `ruviform-media-core`: `TransportPlan` に `musical_ppqn`、builder に
  `with_musical_ppqn`/`with_musical_transport`、`MusicalTimeSource` が追加。
  `SpanSink::span` は第 3 引数 `MusicalTimeSource<'_>` を取る破壊的 API
  変更。musical transport は tempo map と PPQN の両方がある場合だけ有効で、
  missing half を補わない。
- `ruviform-audio`/`ruviform-audio-plugin-bridge`/`ruviform-plugin-core`:
  offline render sink、production graph node、transport bridge が新 contract
  へ適合。`call_transport_info` は Playing かつ tempo map+PPQN が揃う場合
  だけ musical timing を載せ、Stopped/Paused/missing source では捏造・保持
  しない behavior fix。
- `ruviform-audio-test`: `src/midi.rs` の `TraceSink` が新 signature へ
  適合。`tests/os_endpoint.rs`/`tests/rt_contract.rs` は render timestamp
  observation、epoch classifier、OS target gate 等を含む upstream test
  runtime 強化。
- `ruviform-ui`: `ipc.rs` の op/envelope validation が byte-level pure
  decision core へ再構成され、Kani proof が追加。公開 `parse_intent` 契約は
  維持意図。
- `ruviform-ui-test`: control-channel frame length codec が decision core
  化され、partial frame prefix EOF が clean EOF ではなく `UnexpectedEof` /
  truncation として分類される test-only behavior fix。
- 依存木外: `ruviform-audio-device` の WASAPI input forward-gap xrun
  mapping / stereo channel preservation、`drivers/rvf-virtual-audio` の
  deterministic capture xrun injection、plugin format test adapter、
  `ruviform-io-archive` の ZIP/TAR budget decision core と TAR metadata
  block budget fix、`ruviform-io-structured` の CSV/input budget decision
  core、rf#1064 系 consumer-verify workspace manifest pin helper。これらは
  AL lock 25 package に入らない、または AL dependency tree の直接対象外と
  して NotApplicable/Deferred に分類する。

### AL 利用面の確認

- `crates/`/`ui`/`tools` で `SpanSink`、`TransportPlan`、
  `TransportPlanBuilder`、`musical_ppqn`、`run_midi_trace`、`MidiScenario`、
  `TraceSink` の direct use は 0 件。
- `ruviform_audio_test` の direct use は
  `crates/al-analysis/tests/rf628_oracle.rs` の
  `oracle::{PcmCompare, pcm_assert_eq, pcm_reference_block}` と
  `pcm::{PcmFixture, PcmPattern, PcmScenario, PcmSegment}` のみ。
- upstream `ruviform-ui-test` の `read_frame`/`write_frame` direct use は
  0 件。AL にある `read_frames` は GUI waveform viewport の local method で
  あり、control-channel frame codec ではない。
- したがって本 slice で AL `crates/` の API 適合変更は不要だった。

## canonical 回帰 lane（Windows local/headless、全て mise 経由）

| コマンド | exit | 結果 |
| --- | ---: | --- |
| `mise run verify-toolchain` | 0 | toolchain 要件一致 |
| `mise run fmt` | 0 | pass |
| `mise run check al-analysis` | 0 | pass |
| `mise run test al-analysis` | 0 | 151 passed / 0 failed（lib 96、avsync 7、expensive 8、golden 19、reference 12、rf628_oracle 7、ruviform_deps 2。doc-tests 0）。`rf628_long_scenario_non_regression` は 78.44s で pass |
| `mise run check al-service` | 0 | pass |
| `mise run test al-service` | 0 | 11 passed / 0 failed |
| `mise run check al-cli` | 0 | pass |
| `mise run test al-cli` | 0 | 48 passed / 0 failed（al_ctl 20、cli 15、logging 13） |
| `mise run check-cli-headless` | 0 | `no GUI framework deps in 10 headless crates` |
| `mise run check al-gui` | 0 | pass |
| `mise run lint al-gui` | 0 | pass（`-D warnings`） |
| `mise run test al-gui` | 0 | 100 passed / 0 failed |
| `mise run check-ui` | 0 | vendor sync check + TypeScript typecheck pass |
| `mise run test-ui` | 0 | 36 suites、113 pass / 0 fail |
| `mise run check-ui-gpui` | 0 | pass |
| `mise run lint-ui-gpui` | 0 | pass（`-D warnings`） |
| `mise run test-ui-gpui` | 0 | 118 passed / 0 failed |
| `mise run build-ui-gpui` | 0 | `al-desktop` Native Build/Link pass（gpui fork `v0.2.2-rf2` 経由） |
| `mise run test-ui-gpui-real-headless` | 0 | catalog 9 件すべて `not_run` + `environment_blocked`、`environment_kind=deterministic-backend`、`real_window_session=not_started` の fail-closed blocked-probe。実 window/pixel/OS input を主張しない |
| `mise exec -- prek run --all-files` | 0 | pass |

`test-ui-gpui-real-headless` は tracked `evidence/al219/gui-real-headless.jsonl`
を実行時 SHA で更新するが、当該ファイルは本 slice の lease 外であるため、
blocked-probe 形状を確認した後に HEAD へ復帰し、PR 差分には含めない。

非 fatal warning として、mbx の ruviform/gpui git checkout に対する
`absolute path has no stable cache mapping`、一部 build script の non-UTF-8
stdout による cache bypass、および `proc-macro-error2 v2.0.1` の
future-incompatibility warning を観測した。いずれも lane の exit code には
影響しなかった。

## upstream covering CI

- `ci` run 46134393707:
  <https://github.com/scarecrow173/ruviform/actions/runs/46134393707>
  — candidate `eea9e209`、workflow_dispatch、全体 success。`verify`/
  `verify-macos`/`msrv-verify`/`rt-verify`/`ui-verify`/GPUI/L1 real/IO/
  observability/bench を含む 23 jobs が全て success。
- `consumer-verify` run 46144010070:
  <https://github.com/scarecrow173/ruviform/actions/runs/46144010070>
  — candidate `eea9e209`、workflow_dispatch、全体 failure。job 別は
  preflight success / verify-audiolens success / verify-synvelis failure /
  summary failure。verify-audiolens success は upstream harness の補助証拠
  であり、AL の独立 Pass へ転用しない。verify-synvelis failure は
  cross-consumer gate の未解決事実として残す。
- `ci-issue-tracker` run 46145138917 も candidate で success だが、issue
  tracker 自動化であり consumer 互換性の covering evidence としては使わない。

## Requirement × Result

| 項目 | Result | 証拠/理由 |
| --- | --- | --- |
| pin bump 一連作業 | Pass | 25 package が単一 SHA、manifest 非変更、vendor REV 一致。上記 pin lane 全 exit 0 |
| 現行差分の実測評価 | Pass | `87a7ba8f..eea9e209` を read-only clone で分類。採用/見送り/NotApplicable を rev-watch 文書へ記録 |
| f64/長尺/PTS/gap/Validity oracle 非回帰 | Pass（Windows local/headless） | `test al-analysis` 151 pass。rf628 oracle 7 pass、長尺 non-regression pass |
| CLI headless 依存境界 | Pass | `check-cli-headless` exit 0、10 headless crates に GUI framework deps なし |
| GUI semantic/headless contract | Pass | `test al-gui` 100 pass、`test-ui` 113 pass、`test-ui-gpui` 118 pass |
| `al-desktop` Native Build/Link | Pass（Windows） | `build-ui-gpui` exit 0 |
| real-window fail-closed blocked probe | Pass | 9 件すべて not_run/environment_blocked。real window 未実測を正直に記録 |
| upstream candidate covering CI | Pass（補助証拠） | upstream `ci` run 46134393707 success。AL 統合受入へ転用しない |
| consumer-verify cross-consumer | Fail/未解決（slice 対象外） | verify-audiolens success、verify-synvelis failure、summary failure。cross-consumer tuple は後続 |
| interactive real GUI/pixel/OS input/IME | NotRun/Deferred | headless worker 環境では実 window/pixel/OS input を開かない。blocked-probe は Pass を主張しない |
| macOS/Linux OS-L1 | NotRun/Deferred（本 slice） | local lane は Windows。3OS 統合 CI 証拠は branch/Daily CI と Coordinator 統合受入 |
| 外部実機/physical device/virtual endpoint provisioning/EXT-L3 | Unsupported/Deferred | AL が提供しない機能または provisioning を持たない。本 slice の gate に繰り上げない |

## rollback 条件

ruviform 起因の Fail が後続検証で確定した場合は、

```sh
node tools/update-ruviform-rev.mjs 87a7ba8ff3fbe6d4502bea27d152d7f74c105899
```

で旧 pin へ復帰する。lockfile-primary のため manifest 変更は不要。rollback
した場合は、対象 lane・失敗理由・復帰後結果を evidence に記録する。本 slice
では rollback は実行しなかった。

## 残件

- cross-consumer tuple（ruviform/audio-lens/synvelis の同 candidate 横断
  判定）と ruviform #294 への最終報告 — Coordinator/後続 slice。
- interactive Windows desktop real GUI、実 pixel Golden、実 OS input、IME/DPI、
  GUI-L2 最終受入 — 後続 slice。
- 外部 Windows runner、実機/physical device、仮想 audio/MIDI endpoint
  provisioning、driver install、EXT-L3 実機 gate — 後続/別 slice。
- macOS/Linux を含む 3OS 統合 CI 証拠の最終判定 — branch/Daily CI と
  Coordinator 統合受入。
- 新 API（musical transport、plugin host/device boundary、virtual endpoint
  provisioning、xrun injection、archive/structured budget API）の消費 —
  利用要件ができた時点で別途評価。
- docs index（`docs/**/README.md`）更新 — 本 slice では未実施。
  Coordinator が統合時に整備する。
