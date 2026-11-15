# AL#127 rev-watch 検証台帳: ruviform pin `abb9f372` → `eafe0a08`

実行環境: Windows / Orca worktree
`G:\orca\workspaces\audio-lens\al127-ruviform-eafe0a08` /
rustc 1.99.1（mise 解決 toolchain、`verify-toolchain` で一致確認）/
Node 26.12.1 / mise 2026.11.2 系。

- 対象 revision（ruviform resolved SHA）:
  `eafe0a08b71bd9a74566ae6678c3e0683fce1421`
  （`git ls-remote https://github.com/scarecrow173/ruviform.git
  refs/heads/main` と local clone `G:/repositry/audio-products/
  ruviform` の `origin/main` HEAD で一致確認。Spec 起稿時点と
  同一 SHA で upstream の進行なし）
- 旧 pin: `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`（PR #215）
- 差分: `abb9f372..eafe0a08` = 6 commit・93 file・
  +13,204/−317（crates/ 38 file・+7,442/−259、docs/ 45 file・
  +5,302/−43、evidence/ 8 file・+454/−14、root `Cargo.lock`/
  `mise.toml`。`.github/`・ruviform 側 `ui/` 変更なし）
- `Cargo.lock` 差分: ruviform 26 package の `source` fragment
  （SHA）26 行 + `ruviform-audio-test` の `serde`/`serde_json`
  依存 edge 2 行のみ。version 変動・新規 package 追加・他 crate
  への推移的依存変動なし（`serde`/`serde_json` は既に lockfile
  内に存在）。
- 依存木内の変更 crate: dev-dependency の `ruviform-audio-test`
  のみ（RF#628 close-out `#669` + rustfmt `#672`、全て additive）。
  `crates/al-analysis` が消費する `pcm`/`oracle` module は無変更。
- 依存木外の変更: `ruviform-plugin-vst3`/`ruviform-plugin-clap`
  （RF#630 adapter state bridge `#667`）、`ruviform-plugin-test-
  vst3`/`test-clap`（RF#630 test_editor `#668`・test_stateful
  `#671`）、`ruviform-plugin-dock-test`（RF#631 real-editor-attach
  `#670`）。AL lockfile 26 package に非含有。

## 実行した検証（全て正規 mise task、exit code 実測）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `node tools/update-ruviform-rev.mjs eafe0a08b71bd9a74566ae6678c3e0683fce1421` | 0 | 26 package が `eafe0a08` へ原子更新 |
| `mise run setup-ui`（`npm ci --prefix ui`） | 0 | fresh worktree の ui 依存を復元 |
| `mise run sync-ui-deps` | 0 | `ui/vendor/ruviform` 再生成、`REV` = `eafe0a08b7...` |
| `mise run check-ruviform-lock` | 0 | 26 package 単一 SHA（fail-closed） |
| `mise run check-ruviform-rev` | 0 | REV = lockfile SHA、lockfile-primary |
| `mise run check-ui` / `mise run test-ui` | 0 | typecheck + node:test 113 件 fail 0 |
| `mise run fmt` | 0 | fmt 差分なし |
| `mise run check al-analysis` | 0 | `cargo check --locked --all-targets -p al-analysis` Pass |
| `mise run lint al-analysis` | 0 | clippy `-D warnings` Pass |
| `mise run test al-analysis` | 0 | 全 suite fail 0（`tests/rf628_oracle.rs` 7 tests・`rf628_long_scenario_non_regression` 約 84 秒実走を含む） |
| `mise run check-cli-headless` | 0 | headless 10 crate に GUI 依存なし（`serde`/`serde_json` dev-dep 波及なし） |
| `mise run verify-toolchain` | 0 | rustc/cargo 1.99.1 一致 |
| `mise exec -- cargo fetch --locked` | 0 | locked fetch 成功（補助証拠） |
| `mise run check-full`（補助証拠） | 0 | workspace check Pass |
| `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply` | 0 | `written` 5 dir（specs/designs/tasks/test-specs/impl/ir の README）、`blocked: []` |

## NotRun / Unsupported（POLICIES.md §3 語彙）

- 実 OS GUI・実 Pixel・IME・DPI: **NotRun**（実ウィンドウ操作・
  実機 pixel 証拠を要する lane は本実行では未実施）
- macOS / Linux lane: **Unsupported**（実行環境は Windows のみ。
  統合検証は merge 後対象 SHA の CI 証拠が正本）
- cross-consumer（Synvelis 側）非回帰: **NotRun**（Coordinator
  側責務）
- `gui-real` CI job: `not_run` placeholder（既存仕様どおり）
- upstream covering CI（`scarecrow173/ruviform` の `ci`/
  `daily-ci`/`consumer-verify` を candidate `eafe0a08` に対して
  実行した記録）: **NotRun** — `gh run list --commit
  eafe0a08b71bd9a74566ae6678c3e0683fce1421` では schedule の
  `ci-issue-tracker-reconcile`（success、run 43749542109）のみ。
  最新 covering run は `ac10a2f7` 対象（`ci` run 43661273201・
  `daily-ci` 43661251041・`consumer-verify` 43661286172、
  2026-11-15）。ruviform#625 tracker は Open のまま。
- ruviform 側 manual/opt-in lane（`test-l1-real-plugin`・
  `test-ui-plugin-dock`・`test-ui-real-scenario`・windowed
  `uitest` 等）: **NotRun**（実 desktop session / 実 Plugin
  前提の ruviform 側 lane であり AL の検証 lane ではない）
- `lint-full`/`test-full`/`build-full`・`check-ui-gpui`/
  `lint-ui-gpui`/`test-ui-gpui`/`build-ui-gpui`・`check-ui-webview`/
  `lint-ui-webview`/`test-ui-webview`・`check-ci-workflow`/
  `check-ci-tracker-workflows`: **NotRun**（本 slice の正規検証
  コマンド対象外。変更は al-analysis dev-dep 経路に限定され、
  補助証拠として `check-full` のみ実施）

## 記録のみの警告（main 既存・抑制なし）

- mbx cache の absolute-path mapping / build-script UTF-8 warning
  （Git checkout 由来）

## 採用/保留判断

- pin 追従として **採用**。全正規検証 lane が対象 revision で
  Pass、破壊的 API/Feature/OS 変更・AL 側恒久 workaround 要求は
  検出されなかった。
- upstream 新 API/feature（`RunMetadata` JSON 永続化・
  `StreamFault::Cancellation`・plugin state bridge・
  test_editor/test_stateful fixture・plugin-dock real-editor
  attach）は opt-in/test-only/依存木外であり AL 側の採用
  gate・需要が未定義のため本 slice では **採用しない**。
  `RunMetadata` JSON は `rf628_oracle` の証拠強化に使える可能性が
  あるが、現行 oracle は run metadata を永続化していないため
  見送り（採用判断は別 slice）。
- rollback: `Cargo.lock` を `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`
  へ戻す revert + `mise run sync-ui-deps` 再実行で復帰可能
  （製品コード差分なし）。
