# ruviform pin `e4c9f6f0` 追従検証記録（AL#127 pin 追従 slice）

- 実施日: 2026-11-18
- 対象: `b6d0cc6a2e14f47e5324061d8dcac3d905e4abd5` →
  `e4c9f6f0715da3ac479f8c437d21a021327e51da`（upstream main、
  5 commits・78 files）
- candidate tuple: ruviform `e4c9f6f0715da3ac479f8c437d21a021327e51da`
  / AL pin bump commit `f08977ede7ca6affa0e74a046c38017819e5f40c`
  / resolved `e4c9f6f0715da3ac479f8c437d21a021327e51da`
- 判定: **採用**（rollback 不要 — 全 lane exit 0）
- 環境: Windows（worktree `al127-pin-e4c9f6f0`）、rustc/cargo
  1.99.1（`mise run verify-toolchain` 実測）

## pin 一貫性

| lane | 結果 |
| --- | --- |
| `git ls-remote .../ruviform.git refs/heads/main` | `e4c9f6f0715da3ac479f8c437d21a021327e51da`（drift なし） |
| `node tools/update-ruviform-rev.mjs e4c9f6f0…` | exit 0（26 package を単一 SHA に原子解決） |
| `mise run sync-ui-deps` | exit 0（vendor REV = e4c9f6f0、生成物は commit 対象外） |
| `mise exec -- cargo fetch --locked` | exit 0 |
| `mise run check-ruviform-lock` | exit 0 — `e4c9f6f0`、26 packages |
| `mise run check-ruviform-rev` | exit 0 — resolved `e4c9f6f0`、lockfile-primary、REV match |
| `mise run check-ui` | exit 0（sync check + `tsc --noEmit`） |
| `mise run test-ui` | exit 0（node --test 113 pass / 0 fail） |
| `mise run verify-toolchain` | exit 0 — rustc/cargo 1.99.1 |
| `Cargo.lock` 差分 | source fragment 26 行のみ・新規 package 混入なし（`audio-device`/`midi-device` 非含有を grep 実測） |

## 差分影響（`b6d0cc6a..e4c9f6f0`、local clone read-only 実測）

5 commits・78 files。AL 依存木内（`Cargo.lock` ruviform 26 package
と照合）:

- `ruviform-audio-test`（`27c53bd5` / `scarecrow173/ruviform#629` AC3）:
  `metadata.rs` +47（`RunMetadata::for_environment` additive）、
  `tests/os_endpoint.rs` +1,616（upstream test target — AL build
  graph 非含有）、`Cargo.toml` dev-dep audio-device/midi-device
  追加（consumer 非波及 — lockfile 差分で実測確認）。
- `ruviform-plugin-core`（`0450f92f` / `scarecrow173/ruviform#389`・
  `scarecrow173/ruviform#754`）: `GuiSize`、`Plugin` trait default
  実装付き `set_gui_size`/`set_gui_visibility`/`set_gui_parent`、
  `ErasedPlugin` forwarding。
- `ruviform-plugin-host`（同 commit）: `HostCommand::SetGuiSize`/
  `SetGuiVisibility`/`SetGuiParent`、`InProcessHost` dispatch、
  `EditorSessionOps::set_size`/`set_visibility`/`reattach` を
  refusal から実 verb 実行へ実装化（呼出し側不在のため影響なし）。

依存木外: `ruviform-plugin-{clap,dock-test,test-vst3,vst3}`（AL 非提供
の Plugin format adapter — NotApplicable）と docs index/evidence 系
commit。`ruviform-ui*`/dsp/viz/audio-analysis/rt/math/numeric/media/IO
系は差分なし。破壊的 API 変更・serde 形状変化なし。

新 API 採用/見送り: `RunMetadata::for_environment` 見送り（AL 未消費）、
L0 editor verbs 見送り（AL は plugin host 非提供）、os_endpoint/
endpoint probe NotApplicable（device boundary 非提供）。

## slice 検証 lane（実測 exit code）

| lane | exit | 結果 |
| --- | --- | --- |
| `mise run fmt` | 0 | `cargo fmt --check` 合格 |
| `mise run check al-gui` | 0 | `--locked --all-targets` 合格 |
| `mise run lint al-gui` | 0 | clippy `-D warnings` 合格 |
| `mise run test al-gui` | 0 | 全 test target pass（fail 0） |
| `mise run check-ui-gpui` | 0 | `gpui-test` feature check 合格 |
| `mise run lint-ui-gpui` | 0 | `gpui-test` clippy 合格 |
| `mise run test-ui-gpui` | 0 | `gpui-test` 全 test pass（gpui_parity 12・scenario_gpui 5 等 fail 0） |
| `mise run build-ui-gpui` | 0 | `al-desktop`（`gpui` feature）Native Build/Link 合格 |
| `mise run test-ui-gpui-real-headless` | 0 | blocked-probe JSONL 9 case 全件 `not_run`+`environment_blocked`、`environment_kind` = `deterministic-backend`（`real-window` を主張しない） |
| `mise run check-cli-headless` | 0 | GUI/GPU 依存非含有（10 headless crates）— audio-test dev-dep 波及なし |
| `mise run check-ui-webview` | 0 | `al-gui --features webview` 型チェック合格（`ruviform-ui-webview` は e4c9f6f0 で compile） |
| `mise run lint-ui-webview` | 0 | `al-gui --features webview` clippy 合格 |
| `mise run test-ui-webview` | 0 | `al-gui --features webview` 全 test target pass（fail 0） |
| `mise run test al-analysis` | 0 | standing oracle 回帰 pass — 単体 96 + avsync 7 + expensive 8 + golden 19 + reference 12 + rf628_oracle 7（長尺 scenario 116s 含む）+ ruviform_deps 2、fail 0 |
| `mise exec -- prek run --all-files` | 0 | 全 hook Passed（biome・check-ci-workflow・check-ci-tracker-workflows 含む） |

## upstream covering CI

- `gh run list --repo scarecrow173/ruviform --commit
  e4c9f6f0715da3ac479f8c437d21a021327e51da`: **run なし — NotRun**。
- 旧 pin `b6d0cc6a` も run なし。直近 covering run は祖先
  `7124f0b2`（ci success run 44013441029・daily-ci success
  run 44013404796、2026-11-17）。`e4c9f6f0` は同 SHA の
  ci/daily-ci/consumer-verify run が未取得。

## NotRun / Deferred（正直な記録）

- `mise run test-ui-gpui-real`（interactive Windows desktop
  session）: **NotRun** — headless host のため未実施。実機 Pass
  証拠は interactive session 前提の opt-in lane のみ（常設 gate）。
- 実 pixel Golden・実 OS 入力の実測: **NotRun**（同上）。
- macOS/Linux OS-L1: **Deferred** — merge 後対象 SHA の CI 証拠が
  統合検証。
- 3OS CI 実走・cross-consumer 報告（`scarecrow173/ruviform#281`
  集約先と推定）: **対象外** — 常設 gate・Coordinator 責務。
- upstream `e4c9f6f0` の covering CI run 個別確認: **NotRun** —
  merge 後対象 SHA の CI 結果で代替。
