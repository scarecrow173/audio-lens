---
id: "TASK-034P6lzwtU7fjoiE2fyPus"
type: "task"
status: "done"
title: "Ruviform Git 依存 SHA 固定・toolchain 検証・Private repo CI 連携(AL#99)"
created: "2026-10-30"
updated: "2026-10-30"
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
  implements:
    - "../designs/ruviform-dependency-ci.md"
  implemented-by:
    - "docs/impl/ir/ruviform-git-sha-toolchain-private-repo-ci.md"
  depends-on:
    - "../designs/ruviform-dependency-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/99"
    - "https://github.com/scarecrow173/audio-lens/issues/95"
    - "../guides/ruviform-private-access-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/ruviform-foundation-migration.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ruviform Git 依存 SHA 固定・toolchain 検証・Private repo CI 連携(AL#99)

## Work

AL#95 移行の依存基盤のみを構築する。解析・メディア実装の差替は
しない(AL#100/#101 のスコープ)。実施内容は
`docs/designs/ruviform-dependency-ci.md` の詳細設計どおり:

- `[workspace.dependencies]` に 8 crate(ruviform-media-core /
  audio-core / video-core / media-io / math / dsp / audio-analysis /
  exec)を `git + rev=73190d7d26b3197b1da36749798b5952499d1b70`
  で固定し、`Cargo.lock` で再現可能にする
- `.cargo/config.toml` の `net.git-fetch-with-cli` で private
  fetch を system git 経由にする
- `mise.toml` を Ruviform pin(Rust 1.99.0 + rustfmt/clippy、
  uv 0.13.5、Node 26.12.1、actionlint 1.7.12)に揃え、
  `verify-toolchain` task で解決バージョンを明示検証する
- `check-cli-headless` task で al-cli / 解析・Job・Dataset 系
  crate の依存木に GUI/WebView/GPU framework crate が無いことを
  検証する(Ruviform #155/#156/#157 と連携する境界)
- `.github/workflows/ci.yml`(workflow_dispatch 限定、3 OS、
  `permissions: contents: read`、read-only Deploy Key +
  url.insteadOf + known_hosts 照合)を新設し、手順は
  `docs/guides/ruviform-private-access-ci.md` に記録する
- 各消費 crate(al-media / al-locate / al-analysis / al-job)に
  基礎型 import の smoke-test を追加する

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/ruviform-dependency-ci.md`(approved、本タスクの
  詳細設計)
- `docs/plans/ruviform-foundation-migration.md`(AL#97、別
  worktree・未マージ)
- `docs/adr/ruviform-foundation-migration.md`(AL#97、同上)

## Dependencies

- Ruviform commit `73190d7d26b3197b1da36749798b5952499d1b70`
  (scarecrow173/ruviform main HEAD、検証済み)
- ローカルの Git Credential Manager(HTTPS)による private fetch
- CI の `RUVIFORM_DEPLOY_KEY` secret(未設定時は workflow が
  認証 step で明示失敗する設計。登録は運用手順ガイド参照)

## Affected Files

- `Cargo.toml` / `Cargo.lock` / `.cargo/config.toml`
- `crates/al-*/Cargo.toml`(publish 継承 + 消費 crate の依存宣言)
- `mise.toml` / `mise.lock`(actionlint 追加、CI/検証 task)
- `.pre-commit-config.yaml`(actionlint hook)
- `.github/workflows/ci.yml`(新設)
- `tools/verify-toolchain.mjs` / `tools/check-headless-deps.mjs`
- `crates/al-{media,locate,analysis,job}/tests/ruviform_deps.rs`
- `docs/guides/ruviform-private-access-ci.md`

## Done When

- [x] 8 crate が同一 `rev` の git 依存として宣言され、
      `Cargo.lock` に `git+...?rev=<sha>` source が記録される
- [x] `publish = false` が workspace と全 member crate に揃う
- [x] `mise run verify-toolchain` / `mise run check-cli-headless`
      が新設・動作する
- [x] 4 crate の ruviform import smoke-test が通る
- [x] `.github/workflows/ci.yml` + 手順ガイドが揃い、秘密情報を
      含まない

## Verification

- [x] `mise run fmt` / `mise run check` / `mise run lint` /
      `mise run test` / `mise run build`(dev profile 既存契約)
- [x] `mise run check-ci` / `lint-ci` / `test-ci` / `build-ci`
      (`--profile ci` + `CARGO_INCREMENTAL=0`)
- [x] `mise run verify-toolchain` / `mise run check-cli-headless`
- [x] `mise exec -- prek run --all-files`(actionlint による
      workflow 静的検証を含む)
- [x] `git ls-remote` による private repo 到達確認(ローカル
      credential 経由)
