---
id: "TASK-034RIK6H6kFmyRQnMiiikj"
type: "task"
status: "completed"
title: "sccache コンパイルキャッシュ導入評価と採用(AL#148)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/148"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/sccache-compile-cache-issue-148.md"
  implemented-by:
    - "docs/impl/ir/sccache-al-148.md"
  depends-on:
    - "../designs/sccache-compile-cache-issue-148.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/329"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "../designs/audiolens-independent-ci.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/sccache-compile-cache-issue-148.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# sccache コンパイルキャッシュ導入評価と採用(AL#148)

## Work

Issue [#148](https://github.com/scarecrow173/audio-lens/issues/148)
(AL-CI-OPT-01)を正本とする。Ruviform
[#329](https://github.com/scarecrow173/ruviform/issues/329)/PR #342 で実測済みの
sccache(GHA cache backend)を、AL の Rust compile lane である
`verify`・`webview-verify`(各 3 OS)へ適用し、Cold/Warm・cache hit・
総 Runner 時間で採否を判断する。

- `Set up sccache` / `Export sccache GHA backend env` /
  `Verify sccache environment` の 3 step を cargo cache restore 直後に、
  `sccache statistics` を compile lane 末尾に追加(両 lane 同形)
- `SCCACHE_GHA_VERSION=audiolens-v1-<RUNNER_OS>` の repo namespace
- `CI_SCCACHE_DISABLE` repo variable kill switch(全 sccache step skip)
- Before/After を ruviform `ci/metrics/collect-ci-metrics.mjs` 同一手順で
  計測・記録。悪化なら不採用として根拠を記録

## Classification

- `normal-plan-task`

## Upstream Documents

- [design: sccache-compile-cache-issue-148](../designs/sccache-compile-cache-issue-148.md)
- [design: audiolens-independent-ci](../designs/audiolens-independent-ci.md)
- [design: ruviform-dependency-ci](../designs/ruviform-dependency-ci.md)
- ruviform `docs/designs/issue-329-sccache-compile-cache.md` /
  `docs/impl/exp/issue-329-sccache-compile-cache.md`(先行実測)
- Issue [#148](https://github.com/scarecrow173/audio-lens/issues/148)

## Dependencies

- Ruviform #329/#342 の先行評価(導入手順・安全解析の参照元)
- `actions/github-script` `mozilla-actions/sccache-action` の SHA pin
  採用(workflow 内に初出。既存 action は tag pin 混在だが、ruviform 側
  採用実績のある SHA pin に揃える)
- 既存 `actions/cache`(cargo registry)key・`cargo fetch --locked`・
  `check-ruviform-rev`/`check-ruviform-lock` の SHA guard は非変更

## Affected Files

- `.github/workflows/ci.yml`(`verify`・`webview-verify` job に sccache
  step 追加のみ。lane の実行構成・条件・`summary` gate は非変更)
- `docs/designs/sccache-compile-cache-issue-148.md`(新規)
- `docs/tasks/sccache-compile-cache-issue-148.md`(新規)
- `docs/impl/ir/sccache-al-148.md` /
  `docs/impl/exp/issue-148-sccache-before-after.jsonl`(新規)
- `docs/{designs,tasks,impl/ir,impl/exp}/README.md`(index 追従)

## Done When

- [x] `verify`/`webview-verify` 3 OS に sccache step が kill switch 付きで
      定義される
- [x] Cold/Warm run と `sccache --show-stats` で hit/write error を
      実測・記録する
- [x] Before/After で job 時間・総 Runner 時間を比較し採否根拠を記録
- [x] `CI_SCCACHE_DISABLE=1` で sccache step 全 skip + job success を検証
- [x] Golden/f64/LFS/Export/headless/WebView lane・`summary` required
      gate・cargo cache/`cargo fetch --locked`/ruviform SHA guard が非変更
- [x] Rollback(kill switch / git revert)が文書化される

## Verification

- [x] `mise exec -- actionlint .github/workflows/ci.yml`
- [x] `mise exec -- prek run --all-files`
- [x] PR run で 3 OS verify/webview-verify + ui-verify + summary が成功
      (run 41822779052・41823749051・41824903670、全 green)
- [x] 計測ツールで Before/After run metrics を収集
- [x] kill switch の repo variable で sccache step skip を確認(検証後削除)
