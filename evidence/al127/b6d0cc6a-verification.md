# ruviform pin `b6d0cc6a` 追従検証記録（AL#127 dataset-scenario slice）

- 実施日: 2026-11-18
- 対象: `cbcc83437616fdd7418da37183c6be66b6cfe225` →
  `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5`（upstream main、61 commits）
- 判定: **採用**（rollback 不要 — 全 lane exit 0）
- 環境: Windows（worktree `al127-dataset-scenario`）、rustc/cargo
  1.99.1（`mise run verify-toolchain` 実測）

## pin 一貫性

| lane | 結果 |
| --- | --- |
| `node tools/update-ruviform-rev.mjs b6d0cc6a…` | exit 0（26 package を単一 SHA に原子解決） |
| `mise run sync-ui-deps` | exit 0（vendor REV = b6d0cc6a、生成物は commit 対象外） |
| `mise exec -- cargo fetch --locked` | exit 0 |
| `mise run check-ruviform-lock` | exit 0 — `b6d0cc6a`、26 packages |
| `mise run check-ruviform-rev` | exit 0 — resolved `b6d0cc6a`、lockfile-primary、REV match |
| `mise run check-ui` | exit 0（sync check + `tsc --noEmit`） |
| `mise run test-ui` | exit 0（node --test 113 pass / 0 fail） |
| `mise run verify-toolchain` | exit 0 — rustc/cargo 1.99.1 |

## 差分影響（`cbcc8343..b6d0cc6a`、local clone read-only 実測）

AL 依存木内: `ruviform-ui-gpui`（real_window.rs +312 —
window-close/DPI 観測面 `5f0cd43f`/upstream #709、+764 test）、
`ruviform-exec`（executor.rs 新規 — bounded JobExecutor #744/#745）、
`ruviform-audio-core`（midi_input/mock.rs）、`ruviform-audio-test`
（EnvironmentKind serde #725、MIDI fault #707/#713、virtual-endpoint
#629/#680）、`ruviform-io-file`（README のみ）。
依存木外: `ruviform-script`/`ruviform-script-luau`/audio-device/
midi-device/plugin-* 系新規 crate（AL feature 木から未到達）。
`ruviform-ui`/`ruviform-ui-test`/`ruviform-ui-theme`/`ruviform-dsp`/
`ruviform-viz`/`ruviform-audio-analysis`/media/IO 系は差分なし。
破壊的 API 変更なし — 全 feature 組合せで check/lint/test が非回帰。

## slice 検証 lane（実測 exit code）

| lane | exit | 結果 |
| --- | --- | --- |
| `mise run fmt` | 0 | `cargo fmt --check` 合格 |
| `mise run check al-gui` | 0 | `--locked --all-targets` 合格 |
| `mise run lint al-gui` | 0 | clippy `-D warnings` 合格 |
| `mise run test al-gui` | 0 | 全 test target pass |
| `mise run check-ui-gpui` | 0 | `gpui-test` feature check 合格 |
| `mise run lint-ui-gpui` | 0 | `gpui-test` clippy 合格 |
| `mise run test-ui-gpui` | 0 | `gpui-test` 全 test pass（新規 `gpui_dataset_workspace…` 含む） |
| `mise run build-ui-gpui` | 0 | `al-desktop` build 合格 |
| `mise run check al-gui --features gpui-real` | 0 | 合格 |
| `mise run lint al-gui --features gpui-real` | 0 | 合格 |
| `mise run test al-gui --features gpui-real` | 0 | 全 test pass（`scenario_real` 5 件・catalog pin 一致） |
| `mise run test-ui-gpui-real-headless` | 0 | blocked-probe JSONL 9 case 全件 `not_run`+`environment_blocked`、`environment_kind != real-window` |
| `mise run check-cli-headless` | 0 | GUI/GPU 依存非含有（10 headless crates） |
| `mise run test al-analysis` | 0 | standing oracle 回帰 pass（rf628 oracle 含む） |

## 新規 scenario の実測

- `cargo test -p al-gui --test scenario
  fake_dataset_workspace_scenario_switches_tabs_with_al_oracle`:
  PASS — tab-select intent index 1,2,0・selected・6 component
  mounted・typed-cell oracle（alignments accepted・embed offset
  範囲・events typed cells）全て合格。
- `cargo test -p al-gui --features gpui-test --test scenario_gpui
  gpui_dataset_workspace_scenario_switches_tabs_with_al_oracle`:
  PASS — 同一契約を GpuiSimulated で確認。

## NotRun / Deferred（正直な記録）

- `mise run test-ui-gpui-real`（interactive Windows desktop session）:
  **NotRun** — headless host のため未実施。live lane の実機 Pass
  証拠は interactive session 前提の opt-in lane のみ。
- 実 pixel Golden・実 OS 入力の実測: **NotRun**（同上）。
- macOS/Linux OS-L1/GUI-L2: **Deferred** — merge 後対象 SHA の CI
  証拠が統合検証。
- cross-consumer 報告（`scarecrow173/ruviform#219`/`#294`）:
  **対象外** — Coordinator 責務。
- upstream main の covering CI run 個別確認: **NotRun** — merge 後
  対象 SHA の CI 結果で代替。
