---
id: "TASK-bU0xiGWOFmFF7eVry1T2OZ"
type: "task"
status: "in-progress"
title: "Ruviform Git 依存を Cargo.lock 正本へ移行(AL#140)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/ruviform-lockfile-source-of-truth.md"
    - "../designs/ruviform-lockfile-source-of-truth.md"
  implemented-by: []
  depends-on:
    - "../designs/ruviform-lockfile-source-of-truth.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/ruviform-dependency-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/219"
    - "https://github.com/scarecrow173/ruviform/issues/294"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/ruviform-lockfile-source-of-truth.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/audio-lens/issues/99"
  defers: []
  deferred-by: []
---

# Ruviform Git 依存を Cargo.lock 正本へ移行(AL#140)

## Work

Issue [#140](https://github.com/scarecrow173/audio-lens/issues/140)
の作業単位 1〜6 を、
`docs/designs/ruviform-lockfile-source-of-truth.md` の設計どおり
実施する。Ruviform の解決 SHA は `073a3ff…` のまま変えない
(正本の移し替えであり、追従更新ではない)。

- `Cargo.toml` の ruviform-* 14 entry から `rev` を除去し、
  `Cargo.lock` の `source` を `git+<url>#<sha>` に正規化する
- `tools/ruviform-lock-rev.mjs` を新設し、lockfile の
  `[[package]].source` から ruviform resolved revision を
  fail-closed で取得・検証する
- `tools/sync-ruviform-ui.mjs` の正本を Cargo.lock へ移し、
  `ui/vendor/ruviform/REV` 整合は従来契約どおり `--check` で検証
- `tools/update-ruviform-rev.mjs` で candidate SHA への明示
  追従操作を提供し、rollback(`git checkout -- Cargo.lock`)と
  provenance 追跡を文書化する
- `mise.toml` に `check-ruviform-lock` を追加し、`ci.yml` に
  lock 検証 + UI lane(setup-ui→sync-ui-deps→check-ui→test-ui)
  の step を追加する
- `docs/designs/ruviform-dependency-ci.md`・ガイド・
  `.worktreeinclude`・各コメントの「manifest rev 必須」記述を
  更新する

## Classification

- `normal-plan-task`

## Upstream Documents

- [docs/specs/ruviform-lockfile-source-of-truth.md](../specs/ruviform-lockfile-source-of-truth.md)(Issue #140)
- [docs/designs/ruviform-lockfile-source-of-truth.md](../designs/ruviform-lockfile-source-of-truth.md)(approved)
- [docs/plans/ruviform-lockfile-source-of-truth.md](../plans/ruviform-lockfile-source-of-truth.md)(approved)
- [docs/test-specs/ruviform-lockfile-source-of-truth.md](../test-specs/ruviform-lockfile-source-of-truth.md)(検証基準)
- [docs/designs/ruviform-dependency-ci.md](../designs/ruviform-dependency-ci.md)(改訂対象)

## Dependencies

- Ruviform resolved SHA `073a3ff698c5bf9002fe21c7547b04917601e058`
  (現行 lockfile 値を保持)
- ローカルの Git Credential Manager による private fetch、
  CI の `RUVIFORM_DEPLOY_KEY`(従来契約)
- #142(CI 独立化)との `.github/workflows` 競合は
  coordinator の逐次 merge に委譲

## Affected Files

- `Cargo.toml` / `Cargo.lock`
- `tools/ruviform-lock-rev.mjs`(新規)/ `tools/update-ruviform-rev.mjs`(新規)/
  `tools/sync-ruviform-ui.mjs`
- `mise.toml` / `.github/workflows/ci.yml` / `.worktreeinclude`
- `docs/designs/ruviform-dependency-ci.md` /
  `docs/guides/ruviform-private-access-ci.md` / 関連コメント類

## Done When

- [ ] manifest に固定 `rev` が無く、`cargo fetch --locked` が
      lockfile 不変で成功する
- [ ] `check-ruviform-lock` が単一 SHA を検証し、ゼロ件/分裂/
      未知形式で fail-closed する
- [ ] `sync-ui-deps`→`check-ui`→`test-ui` が通り、
      `ui/vendor/ruviform/REV` == lockfile SHA
- [ ] `update-ruviform-rev.mjs` による candidate 更新と
      `git checkout -- Cargo.lock` による rollback が再現できる
- [ ] 「manifest rev 必須」記述が設計・ガイド・コメントから除去
      され、更新/復旧手順が文書化される

## Verification

- [ ] `cargo fetch --locked` / `mise run verify-ci`
      (`fmt`→`check-ci`→`lint-ci`→`test-ci`)
- [ ] `mise run check-ruviform-lock` / `check-cli-headless` /
      `verify-toolchain`
- [ ] `mise run setup-ui` / `sync-ui-deps` / `check-ui` / `test-ui`
- [ ] `cargo metadata --locked` / `cargo tree --locked` による
      単一 source の確認
- [ ] 異常系: lockfile 改変による分裂検出、REV 不一致検出
