---
id: "IMPL-PI4qVm7gfY7jqKGhX5TTGv"
type: "impl"
status: "completed"
title: "AudioLens 独立 PR/Push CI・dual-mode 依存アクセス・UI/Golden 回帰 Gate"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/142"
  changes:
    added:
      - type: "code"
        path: "tools/check-ruviform-rev.mjs"
      - type: "code"
        path: ".github/workflows/ci.yml"
    modified:
      - type: "code"
        path: "mise.toml"
      - type: "data"
        path: "mise.lock"
      - type: "code"
        path: "crates/al-gui/assets/app.js"
      - type: "doc"
        path: "docs/guides/ruviform-private-access-ci.md"
      - type: "doc"
        path: "docs/designs/ruviform-dependency-ci.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/audiolens-independent-ci-issue-142.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/audiolens-independent-ci.md"
    - "docs/designs/audiolens-independent-ci.md"
    - "docs/plans/implement-audiolens-independent-ci.md"
    - "docs/designs/ruviform-dependency-ci.md"
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# AudioLens 独立 PR/Push CI・dual-mode 依存アクセス・UI/Golden 回帰 Gate

## Summary

Issue #142 に従い、dispatch 専用だった `ci.yml` を
`pull_request`/main `push`/`workflow_dispatch` で起動する
AudioLens 独立 CI へ再構成した。Ruviform `consumer-verify.yml`
の convention(access mode 解決・証拠表・scoped mise install)を
踏襲し、重複 Runner は作っていない。

## Context

上流は `docs/specs` → `docs/designs` → `docs/plans` →
`docs/tasks/audiolens-independent-ci-issue-142.md` のチェーン。
旧設計 `docs/designs/ruviform-dependency-ci.md` §10 の
dispatch-only 制約と Deferred の自動トリガ項目を解消した
(同書に追従注記を記入)。

## Implementation

### Changes

- **trigger/権限/同時実行**: `pull_request`(main)/`push`(main)/
  `workflow_dispatch`、`contents: read`、concurrency で PR は
  stale run cancel・main push は履歴保全(`cancel-in-progress:
  github.ref != 'refs/heads/main'`)。paths filter は設けない
  (prek hygiene が全ファイルを検査するため doc-only skip は
  違反を潜伏させる)
- **`ruviform-access` job**: `RUVIFORM_DEPLOY_KEY` 設定済み →
  `deploy_key`、未設定かつ匿名 ls-remote 成功 → `anonymous`、
  両方不可 → `unavailable`(下流 gate step が `::error::` で
  明示 fail。secret は job `if:` で参照不可のため preflight
  output 経由)
- **verify**(3 OS): LFS checkout → `mise install --locked rust
  git-lfs node prek uv biome actionlint` → verify-toolchain →
  mode 別 credential/probe → `cargo fetch --locked` →
  `check-ruviform-rev` → prek → fmt/check-ci/lint-ci/test-ci
  (Golden/実メディア/Export を含む)→ al-cli ci build →
  check-cli-headless
- **ui-verify**(3 OS): `mise install --locked node` → probe →
  `sync-ui-deps` → `setup-ui` →
  `check-ruviform-rev.mjs --require-vendor`(REV == lock SHA)→
  `check-ui`/`test-ui`
- **webview-verify**(3 OS): ubuntu は WebKitGTK dev headers
  (ruviform ui-verify と同一構成)→ fetch/guard →
  `check/lint/test-ui-webview`
- **gui-real**: 実 Window/OS 自動試験は Ruviform #312/#313
  未接続のため、常時 `not_run` を記録する placeholder
- **summary**: 全 lane の job result・access mode・resolved SHA
  を集約表にし、Owner 切り分け指針(Consumer 回帰 → AL 側、
  pin 更新直後のみの失敗 → ruviform #294 へ報告、Export →
  #141)を表示。verifiable lane が `success` 以外なら run を
  失敗にする(skipped/cancelled/not-run は pass とみなさない)
- **`tools/check-ruviform-rev.mjs` + `mise run check-ruviform-rev`**:
  Cargo.lock の ruviform git source が単一 `#<sha>` に解決、
  manifest rev(存在時)が lock と一致、混在/0 件/分裂は
  fail-closed、`--require-vendor` で `ui/vendor/ruviform/REV`
  一致も要求。manifest rev 有り/無し(#140 両方式)で動作
- **`mise.lock`**: `mise lock` で actionlint の 7 platform entry
  を補完(`--locked` scoped install が成立するように)。
  `pipx:serena-agent` は backend 上 entry を生成できない既知
  gap で、CI scope 外のため設計書 Deferred に記録

### Deferred / 既知制約

- workflow 本体は merge まで実実行で検証できない
  (actionlint + step のローカル再現で軽減)
- `pipx:serena-agent` の lockfile platform entry 欠落は残存
  (CI scope 外。全量 `mise install --locked` は依然不可)
- Browser Harness/IPC/Visual Golden の AL fixture は
  Ruviform #310/#311/#314 の提供待ち(ui-verify が受け皿)
- 実 GUI 試験は gui-real placeholder を opt-in lane へ置換する
  拡張点として残す

## Verification

実施済み(ローカル、Windows worktree):

- [x] `mise exec -- actionlint .github/workflows/ci.yml` — pass
- [x] `node tools/check-ruviform-rev.mjs` —
      `resolved 073a3ff… (15 lockfile sources, manifest-pinned)`
- [x] `node tools/check-ruviform-rev.mjs --require-vendor` —
      vendor sync 後に `REV match` で pass
- [x] fail 経路(一時改変で検証・復元済み): manifest revs 分裂、
      manifest rev ≠ lock、混在 pinned/unpinned、lockfile SHA
      分裂、REV 不在 + `--require-vendor` — すべて exit 1
- [x] lockfile-primary 模擬(manifest から全 rev 除去)→
      `lockfile-primary` mode で pass(復元済み)
- [x] `mise exec -- prek run --all-files` — 全 hook pass
- [x] `mise run verify-toolchain` / `mise run fmt` — pass
- [x] `mise run check-ci`(workspace typecheck) — pass
- [x] `mise run lint-ci`(biome + clippy `-D warnings`) — pass。
      途中 `crates/al-gui/assets/app.js` の pre-existing biome
      error(文字列連結・`isFinite`・未使用引数、main 上 a5f683e
      由来で CI 未検証だったもの)を `biome --unsafe` の機械的修正で
      解消 — 新 lint lane の初回実行を赤にしないための付帯修正
- [x] `mise run test-ci`(workspace tests、Golden/LFS fixture/
      stage resume/Export を含む 46 suite) — 全 pass
- [x] `mise exec -- cargo build -p al-cli --profile ci` — pass
- [x] `mise run check-cli-headless` — pass(10 headless crates)
- [x] `mise run sync-ui-deps` / `mise run setup-ui` — pass
- [x] `mise run check-ui`(sync --check + tsc) — pass
- [x] `mise run test-ui`(happy-dom 112 tests) — 全 pass
- [ ] 実 workflow 実行 — 未検証。workflow は merge まで e2e
      検証不可(旧設計 §10 と同制約)。初回 PR/run で確認
- [ ] deploy_key 経路 — 未検証。secret 未設定かつ repo public
      のため anonymous が有効経路(手順は旧構成を継承)
- [ ] 3 OS 実測 — 未検証(同上)
