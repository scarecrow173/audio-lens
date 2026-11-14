# Issue #210 検証台帳

実行環境: Windows 11 / ORCA worktree
`C:\Users\akiyoshi\orca\workspaces\audio-lens\al210-ci-fix` /
rustc 1.99.1(環境導入、repo は rust を pin しない)/
mise 管理 tool: prek 0.5.7, biome 2.6.2, actionlint 1.7.12,
node 26.12.1, git-lfs 3.8.0, uv 0.13.5, apm 0.33.0。
`MISE_ENV` 未設定 = ローカル標準 profile(dev/test、
`CARGO_INCREMENTAL` 既定)。CI の `MISE_ENV=ci` +
`mise.ci.toml` pin とは別環境である点に注意。

ブランチ: `bugfix/210-ci-failure-main`(基点 `0e2bfee` =
失敗 run の sha と同一。`git fetch origin main` で
`0e2bfee` 以降の修正が main に入っていないことを確認済み)。

## Passed

| コマンド | 目的 | exit | 結果 |
| --- | --- | --- | --- |
| `mise exec -- prek run --all-files` | CI `hygiene` job 相当 | 0 | 全 hook Pass(trailing-whitespace, end-of-file-fixer, mixed-line-ending, fix-byte-order-marker, check-yaml, check-toml, detect-private-key, check-illegal-windows-names, biome check --write, actionlint, check-ci-workflow, check-ci-tracker-workflows)。修正前は config parse error で hook 未実行 |
| `mise exec -- biome check` | lint-full の biome 部 | 0 | Checked 223 files、診断 0 件。修正前は exit 1(error 1/warning 5/info 13)を再現 |
| `mise run lint-full` | verify ubuntu lane の lint 相当 | 0 | `biome check` + `cargo clippy --workspace --all-targets -- -D warnings` ともに Pass(clippy は dev profile、mbx cache 経由) |
| `mise run test-full` | verify lane の test 相当 | 0 | `cargo test --workspace` 全 test Pass(0 failed) |
| `mise run fmt` | rustfmt check | 0 | `cargo fmt --all -- --check` 差分なし |
| `mise run check-cli-headless` | headless crate の GUI 依存不在 | 0 | no GUI framework deps in 10 headless crates |
| `mise run verify-toolchain` | toolchain 整合 | 0 | rustc/cargo 1.99.1、rustfmt+clippy present |
| `mise run check-ruviform-rev` | Ruviform resolved SHA 整合 | 0 | resolved `fd1d4d5f...`(20 lockfile sources、lockfile-primary) |
| `mise run check-ruviform-lock` | Cargo.lock 正本の rev 検証 | 0 | `fd1d4d5f4de57092b96ad1cd309b85fe47cbef38`(20 packages) |
| `mise run check-ci-workflow` | ci.yml 構造契約 | 0 | Full CI 構造契約 OK |
| `mise run check-ci-tracker-workflows` | tracker workflow 構造契約 | 0 | tracker workflow 構造契約 OK |
| `node tools/check-ci-workflow.mjs` | 同上(script 直実行) | 0 | OK(semantics 保存の確認) |
| `node tools/check-ci-tracker-workflows.mjs` | 同上(script 直実行) | 0 | OK(semantics 保存の確認) |
| `git diff` レビュー | unsafe fix の semantics 確認 | — | `indexOf`/`?.`/template literal/format の全変更が semantics 保存であることを確認。照合文字列リテラルは無変更 |

## NotRun

| 項目 | 理由 |
| --- | --- |
| `verify-macos` / `verify (macos-latest)` leg | macOS はローカル実行不可。失敗 run で当該 leg は success であり、今回の変更は OS 依存箇所(Rust/OS 固有コード・workflow)に触れない |
| `ui-verify-macos` / `webview-verify-macos` / `gpui-verify-macos` | 同上(macOS leg)。当該 run で success |
| `gui-real` | 設計上の not_run placeholder(実機 GUI 受入は別 lane) |
| 3OS matrix の Windows/Linux 以外 leg | macOS leg に同じ。Windows leg は本 worktree(Windows)で lint-full/test-full 相当を実走済みだが、CI の `verify (windows-latest)` leg 固有の step 一式は未実行 — merge 後 CI 証跡で確認 |
| `*-msrv` 系タスク | daily-CI scope。ユーザー指定により実行しない |
| `MISE_ENV=ci` + `mise.ci.toml` pin toolchain での `*-full` | CI 固有 pin はローカルで強制しない(POLICIES.md §5)。merge 後の対象 SHA CI が統合証拠 |
| LFS 実体が必要な資産検証 | 本タスクの変更範囲に LFS 資産依存の検証は含まれない |
| merge 後 main SHA の `ci` run 全必須 job success | merge と CI 証跡確認は Coordinator が実施 |

## 補足

- `mise install --locked` は user-global mise 設定
  (`~/.config/mise/config.toml`)の未 pin tool(pnpm, ffmpeg,
  python, rust, codex, ccusage, mr-boxington)を理由に失敗する。
  repo 必須 tool は全て install 済みで `mise exec` が解決する
  ことを個別に確認済み。環境起因であり repo の問題ではない。
- `cargo clippy` 実行中に mbx cache の build-script UTF-8
  warning が出るが、clippy/rustc の lint warning ではなく
  キャッシュ層の notice。`-D warnings` 下で exit 0。
