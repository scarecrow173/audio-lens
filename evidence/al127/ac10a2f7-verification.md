# AL#127 rev-watch 検証台帳: ruviform pin `c6e4f334` → `ac10a2f7`

実行環境: Windows / Orca worktree
`G:\orca\workspaces\audio-lens\al127-rev-watch` /
rustc 1.99.1（mise 解決 toolchain、`verify-toolchain` で一致確認）/
Node 26.12.1 / mise 2026.11.2 系。

- 対象 revision（ruviform resolved SHA）:
  `ac10a2f7761c08db73aebd861c0a5c1f53f269b7`
  （`git ls-remote https://github.com/scarecrow173/ruviform.git
  refs/heads/main` と local clone `G:/repositry/audio-products/
  ruviform` の `origin/main` HEAD で一致確認。Spec 起稿時点と
  同一 SHA で upstream の進行なし）
- 旧 pin: `c6e4f334242567dbd0e0d8b7713d9d480a6aab66`（PR #213）
- 差分: `c6e4f334..ac10a2f7` = 14 commit・148 file・
  +22,339/−223（docs/ 87 file・+11,733/−37、evidence/ 18 file・
  +1,510、crates/ 40 file・+9,049/−184、root Cargo.toml/
  Cargo.lock/mise.toml +47/−2。`.github/`・`ui/` 変更なし）
- `Cargo.lock` 差分: ruviform 20 package の `source` fragment
  （SHA）20 行のみ。推移的依存の変動・新規 package 追加なし。
- 依存木内の変更 crate: `ruviform-numeric`（test-only、
  RF#646）・`ruviform-ui-test`（additive、RF#631）のみ。

## 実行した検証（全て正規 mise task、exit code 実測）

| コマンド | exit | 結果 |
| --- | --- | --- |
| `node tools/update-ruviform-rev.mjs ac10a2f7761c08db73aebd861c0a5c1f53f269b7` | 0 | 20 package が `ac10a2f7` へ原子更新 |
| `mise run setup-ui`（`npm ci --prefix ui`） | 0 | fresh worktree の ui 依存を復元 |
| `mise run sync-ui-deps` | 0 | `ui/vendor/ruviform` 再生成、`REV` = `ac10a2f776...` |
| `mise run check-ruviform-lock` | 0 | 20 package 単一 SHA（fail-closed） |
| `mise run check-ruviform-rev` | 0 | REV = lockfile SHA、lockfile-primary |
| `mise run check-ui` / `mise run test-ui` | 0 | typecheck + node:test 113 件 fail 0 |
| `mise run fmt` | 0 | fmt 差分なし |
| `mise run check-full` | 0 | workspace check Pass |
| `mise run lint-full` | 0 | biome + clippy `-D warnings` Pass |
| `mise run test-full` | 0 | 58 suite fail 0 |
| `mise run build-full` | 0 | workspace build Pass |
| `mise run check-cli-headless` | 0 | headless 10 crate に GUI 依存なし |
| `mise run check-ui-gpui` / `lint-ui-gpui` / `test-ui-gpui` / `build-ui-gpui` | 0 | `gpui`/`gpui-test` feature lane Pass |
| `mise run check-ui-webview` / `lint-ui-webview` / `test-ui-webview` | 0 | Legacy WebView lane Pass |
| `mise run check-ci-workflow` / `check-ci-tracker-workflows` / `verify-toolchain` | 0 | workflow 構造契約・toolchain 一致 |
| `mise exec -- cargo fetch --locked` | 0 | locked fetch 成功（補助証拠） |
| `node .agents/skills/doc-maintenance/scripts/doc_maintenance.js index apply` | 0 | `written` 5 dir、`blocked: []`、再 plan で全 dir `in-sync` |

## NotRun / Unsupported（POLICIES.md §3 語彙）

- 実 OS GUI・実 Pixel・IME・DPI: **NotRun**（実ウィンドウ操作・
  実機 pixel 証拠を要する lane は本実行では未実施。
  `test-ui-gpui` は `GpuiSimulated` driver の semantic/headless
  検証であり実機 acceptance の代替ではない）
- macOS / Linux lane: **Unsupported**（実行環境は Windows のみ。
  統合検証は merge 後対象 SHA の CI 証拠が正本）
- cross-consumer（Synvelis 側）非回帰: **NotRun**（Coordinator
  側責務）
- `gui-real` CI job: `not_run` placeholder（既存仕様どおり）
- ruviform 側 `test-ui-plugin-dock`（manual/opt-in）・
  `test-ui-real-scenario`・windowed `uitest` lane・Linux X11
  offscreen golden lane: **NotRun**（実 desktop session 前提の
  ruviform 側 manual lane であり AL の検証 lane ではない。
  AL は当該機能を有効化していない）

## 記録のみの警告（main 既存・抑制なし）

- `proc-macro-error2 v2.0.1` の future-incompatibility warning
  （依存由来）
- mbx cache の absolute-path mapping / build-script UTF-8 warning
  （Git checkout 由来）

## 採用/保留判断

- pin 追従として **採用**。全正規検証 lane が対象 revision で
  Pass、破壊的 API/Feature/OS 変更・AL 側恒久 workaround 要求は
  検出されなかった。
- upstream 新 API/feature（`ruviform-plugin-dock-test`・
  `ruviform-plugin-test-vst3`・ui-test plugin_dock scenario/
  `PluginHostProbe`・`ruviform-{audio,midi}-device` endpoint
  probe examples・`ruviform-io-e2e` opt-in lanes・
  `ruviform-plugin-core` V9+V11+V12 MIDI event contract）は
  test-only/opt-in/依存木外のため本 slice では **採用しない**。
- rollback: `Cargo.lock` を `c6e4f334...` へ戻す revert +
  `mise run sync-ui-deps` 再実行で復帰可能（製品コード差分
  なし）。
