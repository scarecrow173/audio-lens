# ruviform pin `87a7ba8f` 追従検証記録（AL#127 pin 追従 slice）

- 実施日: 2026-11-26
- 対象: `e4c9f6f0715da3ac479f8c437d21a021327e51da` →
  `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`（dispatch 時 upstream
  main、281 commits・3,758 files）
- candidate tuple: ruviform
  `87a7ba8ff3fbe6d4502bea27d152d7f74c105899` / AL pin bump commit
  `67cc975ec651e502ad863c24db1db104cfe025b3` / resolved
  `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`
- 判定: **採用**（rollback 不要 — pin 一貫性 lane と canonical 回帰 lane が
  全て exit 0）
- 環境: Windows（worktree `al127-pin-current`）、rustc/cargo 1.99.1、
  Node.js v26.12.1、mise 2026.11.2 windows-x64

## pin 一貫性

| lane | 結果 |
| --- | --- |
| `git ls-remote https://github.com/scarecrow173/ruviform.git refs/heads/main`（dispatch 時） | `87a7ba8ff3fbe6d4502bea27d152d7f74c105899`（起稿時点から drift なし） |
| `node tools/update-ruviform-rev.mjs 87a7ba8f…` | exit 0（25 package を単一 SHA に原子解決） |
| `mise run setup-ui` | exit 0（fresh worktree の npm 依存復元、10 packages added） |
| `mise run sync-ui-deps` | exit 0（vendor REV = `87a7ba8f…`、生成物は commit 対象外） |
| `mise exec -- cargo fetch --locked` | exit 0 |
| `mise run check-ruviform-lock` | exit 0 — `87a7ba8f…`、25 packages |
| `mise run check-ruviform-rev` | exit 0 — resolved `87a7ba8f…`、lockfile-primary、REV match |
| `mise exec -- node tools/check-ruviform-rev.mjs --require-vendor` | exit 0 — 同上 |
| `Cargo.lock` 差分 | 25 個の ruviform source fragment + `gpui` source switch のみ。`[[package]]` 追加/削除 0、無関係 version bump なし、upstream dev-dep `proptest` は既存 1 entry のまま非増加 |
| manifest 非変更 | ルート/各 crate の `Cargo.toml` に `rev`/`branch`/`tag` を追加せず |

candidate freeze 後の再確認では upstream main が
`0fb97d1ba89e1fee154cdbbb72ceb2db15fdcd8c` へ進んでいた。これは本 slice の
採否対象確定後の drift であり、次回 rev-watch 対象とする。
`87a7ba8f..0fb97d1b` は `ruviform-audio-device` の 1 commit で、同 package は
AL lock に含まれない。

## `Cargo.lock` / dependency source impact

`gpui` 0.2.2 は crates.io registry から
`git+https://github.com/scarecrow173/gpui?tag=v0.2.2-rf2#b3144f286b942fbb5f478da73db13a826cef054a`
へ切り替わった。tag `v0.2.2-rf2` の peeled commit は匿名 `git ls-remote` で
`b3144f286b942fbb5f478da73db13a826cef054a` と再確認した。upstream
`ruviform-ui-gpui` の説明では、crates.io 0.2.2 を基礎に DirectComposition
target を non-topmost にする 1 行修正を含む fork である。

この source switch は pin bump に伴う upstream transitive dependency effect
であり、AL 側手編集ではない。`build-ui-gpui` と `test-ui-gpui-real-headless`
で同 source の compile/link/blocked-probe が通ることを確認したが、実 GUI
表示・pixel・IME の証拠ではない。

## 差分影響（`e4c9f6f0..87a7ba8f`、local clone read-only 実測）

- 全体: 281 commits・3,758 files（+212,585/−247,733）。rename detection
  は件数上限で省略。`crates/` は 157 files（+56,418/−5,897）。大半は
  docs/evidence/skill 記録と upstream 検証資産である。
- merge-base は旧 pin `e4c9f6f0715da3ac479f8c437d21a021327e51da` で、
  candidate は旧 pin の descendant。
- production `src/` 差分なし: `ruviform-audio-analysis`/
  `ruviform-audio-core`/`ruviform-dsp`/`ruviform-exec`/`ruviform-graph`/
  `ruviform-io-avi`/`ruviform-io-file`/`ruviform-media-core`/
  `ruviform-numeric`/`ruviform-rt`/`ruviform-ui-theme`/`ruviform-video-core`。
  `ruviform-io-symphonia`/`ruviform-media-io`/`ruviform-observability`/
  `ruviform-viz` も production `src/` 差分はない。`ruviform-audio-plugin-bridge`
  と `ruviform-audio-test` も `src/` 差分なし。
- `ruviform-audio/src/typed/probe.rs`: ProbeDeck capture chunk push が
  `debug_assert!` 内副作用から常時実行へ修正（upstream commit `7853ccd2`）。
  AL は `ProbeDeck` を直接参照しないため transitive compile のみ。
- `ruviform-math/src/dsp.rs`: 証拠文書参照コメントのみ。production 意味論
  差分なし。
- `ruviform-plugin-core`/`ruviform-plugin-host`: GUI floating/editor verbs、
  plugin→host GUI request channel、dirty/param notices、L1 diagnostics、
  wire v5→v9。`Plugin::buses` は `&[BusDescriptor]` から
  `Option<&[BusDescriptor]>` への破壊的契約変更（`None` = unavailable、
  `Some(&[])` = declared none）。AL の `crates/`/`tools/`/`ui` に plugin API
  実装・利用がなく、compile/test は非回帰。AL は plugin host を提供しない
  ため、機能そのものは NotApplicable。
- `ruviform-ui`: `InputSpec`/`InputValueSpec`、`ViewHost::submit_input`、
  `CommandParam`/`CommandParams`、`ComponentSpec::Input` を additive に追加。
  `CommandIntent::Action.params` は旧 pin にも optional/default の
  `serde_json::Value` として存在し、serde 形状は非回帰。
- `ruviform-ui-test`: `rvf_ctl` binary、transport/command-param tests、
  `ScenarioCapability::OutOfProcessPluginHost` を追加。AL は新 capability
  を主張しない。
- `ruviform-ui-gpui`: Windows IMM32 IME capability 向け `os_ime.rs`、
  native slot container/surface compositing、lane provenance capability
  claim の変更。gpui dependency は fork tag `v0.2.2-rf2` へ切替。
- 依存木外: `ruviform-audio-device`、plugin format adapter、virtual
  endpoint provisioning、driver install、実機検証資産。AL lock に現れず
  NotApplicable/Deferred。

## slice 検証 lane（実測 exit code）

全て Windows・mise 経由で実行した。

| # | lane | exit | 結果 |
| --- | --- | --- | --- |
| 01 | `mise run verify-toolchain` | 0 | rustc/cargo 1.99.1、rustfmt+clippy present |
| 02 | `mise run fmt` | 0 | `cargo fmt --check` 合格 |
| 03 | `mise run check al-analysis` | 0 | 合格 |
| 04 | `mise run test al-analysis` | 0 | 8 suites・151 passed / 0 failed。単体 96・avsync 7・expensive 8・golden 19・reference 12・rf628_oracle 7（長尺 scenario 含む）・ruviform_deps 2 |
| 05 | `mise run check al-service` | 0 | 合格 |
| 06 | `mise run test al-service` | 0 | 11 passed / 0 failed |
| 07 | `mise run check al-cli` | 0 | 合格 |
| 08 | `mise run test al-cli` | 0 | 48 passed / 0 failed |
| 09 | `mise run check-cli-headless` | 0 | 10 headless crates に GUI framework deps なし |
| 10 | `mise run check al-gui` | 0 | 合格 |
| 11 | `mise run lint al-gui` | 0 | clippy 合格 |
| 12 | `mise run test al-gui` | 0 | 16 targets・100 passed / 0 failed |
| 13 | `mise run check-ui` | 0 | vendor sync check + `tsc --noEmit` 合格 |
| 14 | `mise run test-ui` | 0 | node:test 36 suites・113 passed / 0 failed |
| 15 | `mise run check-ui-gpui` | 0 | `gpui-test` feature check 合格 |
| 16 | `mise run lint-ui-gpui` | 0 | `gpui-test` clippy 合格 |
| 17 | `mise run test-ui-gpui` | 0 | 17 targets・118 passed / 0 failed |
| 18 | `mise run build-ui-gpui` | 0 | gpui fork 経由の `al-desktop` Native Build/Link 合格 |
| 19 | `mise run test-ui-gpui-real-headless` | 0 | blocked-probe 9 case 全件 `not_run`+`environment_blocked`、`environment_kind` = `deterministic-backend`。`real-window` を主張しない fail-closed 形状を維持 |
| 20 | `mise exec -- prek run --all-files` | 0 | 全 hook Passed（biome・actionlint・check-ci-workflow・check-ci-tracker-workflows を含む） |

補助証拠ではなく canonical な mise lane として上記を記録する。`cargo fetch`
も `mise exec --` 経由で実行した。

### 実行時 warning（Fail ではないが隠さない）

- mbx が ruviform/gpui git checkout 配下で `absolute path has no stable
  cache mapping` warning を出した。cache 効率の警告で、compile/test 結果は
  exit 0。
- `proc-macro-error2 v2.0.1` に future-incompatibility warning が出た。
  既存 transitive dependency の警告で、本 slice で抑制しない。
- `test-ui-gpui-real-headless` では mbx が managed target への出力移動で
  `アクセスが拒否されました。 (os error 5)` を出したが、build は既存 target
  directory で継続し exit 0。検証成功条件は変更していない。
- `test-ui-gpui-real-headless` は tracked な
  `evidence/al219/gui-real-headless.jsonl` を更新する。本 slice の lease 外
  なので、出力内容で blocked-probe 形状を確認した後に HEAD 版へ復帰した。
  このファイルは PR に含めない。

## upstream covering CI

- `gh run list --repo scarecrow173/ruviform --commit
  87a7ba8ff3fbe6d4502bea27d152d7f74c105899`: **run なし — NotRun**。
- 直近 ancestor `ab6dc99fe8935ac09082d05e2176b10e729ba263`:
  - `ci` success — run 45749815004
    （https://github.com/scarecrow173/ruviform/actions/runs/45749815004）
  - `daily-ci` success — run 45749786810
    （https://github.com/scarecrow173/ruviform/actions/runs/45749786810）
- `ab6dc99f..87a7ba8f` は 3 commits・14 files で、docs/evidence/skill 記録
  のみ。production code 差分はない。

## rollback 条件

ruviform 起因の Fail が後続検証で確定した場合は、
`node tools/update-ruviform-rev.mjs
e4c9f6f0715da3ac479f8c437d21a021327e51da` で旧 pin へ復帰する。
lockfile-primary のため manifest rollback は不要。rollback 実施時は、
失敗 lane・原因分類・復帰後 lane を同じ形式で記録する。

## NotRun / Unsupported / Deferred（正直な記録）

- `mise run test-ui-gpui-real`（interactive Windows desktop session）:
  **NotRun** — headless worker のため未実施。実 window/pixel/OS input/IME
  の Pass 証拠ではない。
- 実 pixel Golden・実 OS 入力・IMM32 IME composition: **NotRun** — 同上。
  upstream `os_ime.rs` の存在を AL の capability claim へ転用しない。
- macOS/Linux OS-L1: **Deferred** — merge 後対象 SHA の CI 証拠が統合
  検証。本 slice は Windows local/headless lane。
- 3OS 統合 CI、candidate tuple の cross-consumer 判定、
  `scarecrow173/ruviform#294` 後続集約: **対象外/Deferred** — Coordinator
  または後続 slice。
- upstream `87a7ba8f` 自体の covering CI: **NotRun** — run なし。
  ancestor `ab6dc99f` の success は補助情報であり、candidate 自体の証拠へ
  昇格しない。
- candidate freeze 後の upstream main `0fb97d1b`: **未評価** — 次回
  rev-watch 対象。
- docs index（`docs/**/README.md`）更新: **未対応** — 本 slice の編集範囲外。
  Coordinator が統合時に整備する。
