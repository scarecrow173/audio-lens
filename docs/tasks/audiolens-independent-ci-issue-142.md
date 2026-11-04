---
id: "TASK-lfTPHtxfqyQTx3piqn0UgP"
type: "task"
status: "done"
title: "AudioLens 独立 PR/Push CI・dual-mode 依存アクセス・UI/Golden 回帰 Gate(AL#142)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/142"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-audiolens-independent-ci.md"
  implemented-by:
    - "../impl/ir/audiolens-independent-ci-issue-142.md"
  depends-on:
    - "../designs/audiolens-independent-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/audio-lens/issues/140"
    - "https://github.com/scarecrow173/audio-lens/issues/141"
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/294"
    - "../guides/ruviform-private-access-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../plans/implement-audiolens-independent-ci.md"
    - "../designs/audiolens-independent-ci.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# AudioLens 独立 PR/Push CI・dual-mode 依存アクセス・UI/Golden 回帰 Gate(AL#142)

## Work

Issue [#142](https://github.com/scarecrow173/audio-lens/issues/142)
を正本とする。`docs/designs/audiolens-independent-ci.md` の設計に
従い、dispatch 専用 `ci.yml` を AudioLens 独立の継続検証 CI へ
拡張する:

- `pull_request`(main 向け)/`push`(main)/`workflow_dispatch`
  トリガ、`contents: read` 最小権限、PR では stale run cancel・
  main では履歴保全の concurrency 方針
- `ruviform-access` preflight job で deploy_key / anonymous /
  unavailable を解決。unavailable は下流 gate step が `::error::`
  で明示 fail、not-run を pass と偽らない
- `MISE_ENABLE_TOOLS` + `mise install --locked <tools>` で CI 必要
  ツール(rust/git-lfs/node/prek/uv/biome/actionlint)へ scope。
  `mise.lock` に actionlint の platform entry を `mise lock` で
  補完する
- `verify`(3 OS: fmt/check-ci/lint-ci/test-ci + LFS fixture +
  al-cli build + check-cli-headless + fetch 後 SHA ガード)、
  `ui-verify`(3 OS: sync-ui-deps + npm ci + REV 整合 + typecheck
  /test)、`webview-verify`(3 OS: al-gui webview feature、
  ubuntu は WebKitGTK dev 導入)、`gui-real`(not-run 記録
  placeholder)、`summary`(証拠集約 + Owner 切り分け +
  all-not-run/cancelled fail)
- `tools/check-ruviform-rev.mjs`: lockfile resolved SHA 単一性 +
  manifest rev(存在時)+ UI vendor `REV` 整合を両依存方式で検証
- ガイド・旧設計書(#99)の追従更新

## Classification

- `normal-plan-task`

## Upstream Documents

- [spec: audiolens-independent-ci](../specs/audiolens-independent-ci.md)
- [design: audiolens-independent-ci](../designs/audiolens-independent-ci.md)
- [plan: implement-audiolens-independent-ci](../plans/implement-audiolens-independent-ci.md)
- [design: ruviform-dependency-ci](../designs/ruviform-dependency-ci.md)
  (§10/Deferred を本件で更新)
- Issue [#142](https://github.com/scarecrow173/audio-lens/issues/142)

## Dependencies

- Ruviform `consumer-verify.yml` の convention(ruviform #294)
- 現在両 repo は public → anonymous 経路が主経路。Deploy Key は
  private 復帰時の fallback として維持
- #140 未完了のため manifest `rev` 有り/無しの両方式で動く設計

## Affected Files

- `.github/workflows/ci.yml`(再構成)
- `tools/check-ruviform-rev.mjs`(新規)
- `mise.toml`(`check-ruviform-rev` task 追加)/ `mise.lock`
  (actionlint platform entry)
- `crates/al-gui/assets/app.js`(pre-existing biome error の機械的
  修正 — 新 lint lane を成立させる付帯修正)
- `docs/guides/ruviform-private-access-ci.md`(dual-mode 追従)
- `docs/designs/ruviform-dependency-ci.md`(§10/Deferred 追従)
- `docs/specs`/`docs/designs`/`docs/plans`/`docs/tasks`/
  `docs/impl/ir` + 各 `README.md`

## Done When

- [x] PR/push/dispatch の 3 トリガと concurrency 方針が ci.yml に
      定義される
- [x] anonymous / deploy_key / unavailable の 3 mode が
      `ruviform-access` で解決され、unavailable が明示 fail になる
- [x] `mise install --locked` が CI scope ツールで成立し、
      actionlint の lockfile gap が補完される
- [x] 3 OS の verify/ui-verify/webview-verify lane と gui-real
      not-run 記録、summary の証拠表 + Owner 切り分けが定義される
- [x] `check-ruviform-rev` が lockfile 単一 SHA・manifest rev・
      REV 整合を両方式で検証する
- [x] ガイドが dual-mode に追従される

## Verification

- [x] `mise exec -- actionlint .github/workflows/ci.yml`
- [x] `node tools/check-ruviform-rev.mjs`(正常系 + fail 系)
- [x] `node tools/check-ruviform-rev.mjs --require-vendor`
- [x] `mise exec -- prek run --all-files`
- [x] `mise run verify-toolchain` / `mise run fmt`
- [x] `mise run check-ci` / `mise run lint-ci` / `mise run test-ci`
- [x] `mise exec -- cargo build -p al-cli --profile ci`
- [x] `mise run check-cli-headless`
- [x] `mise run check-ui` / `mise run test-ui`(112 tests)
      (既存 task 意味の非破壊)
- [ ] 実 workflow 実行(merge 後の初回 PR/run で確認。本 PR 上
      では検証不可のため未検証として記録)
