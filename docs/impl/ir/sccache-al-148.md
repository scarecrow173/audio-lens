---
id: "IMPL-034RIKarT1xed13n8pfaNy"
type: "impl"
status: "in-progress"
title: "sccache コンパイルキャッシュ導入評価(AL#148)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/148"
  changes:
    added:
      - type: "doc"
        path: "docs/designs/sccache-compile-cache-issue-148.md"
      - type: "doc"
        path: "docs/tasks/sccache-compile-cache-issue-148.md"
      - type: "doc"
        path: "docs/impl/exp/issue-148-sccache-before-after.jsonl"
    modified:
      - type: "code"
        path: ".github/workflows/ci.yml"
      - type: "doc"
        path: "docs/designs/README.md"
      - type: "doc"
        path: "docs/tasks/README.md"
      - type: "doc"
        path: "docs/impl/ir/README.md"
      - type: "doc"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/sccache-compile-cache-issue-148.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/sccache-compile-cache-issue-148.md"
    - "docs/designs/audiolens-independent-ci.md"
    - "https://github.com/scarecrow173/ruviform/issues/329"
    - "https://github.com/scarecrow173/ruviform/pull/342"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/sccache-compile-cache-issue-148.md"
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

# sccache コンパイルキャッシュ導入評価(AL#148)

## Summary

Issue #148 (AL-CI-OPT-01)。Ruviform #329/PR #342 で実測済みの
sccache GHA cache backend を、AL の Rust compile lane
(`verify`・`webview-verify`、各 3 OS)へ適用し採否を実測で判断する。
ui-verify は cargo を呼ばないため対象外。

## Context

`docs/tasks/sccache-compile-cache-issue-148.md` の実装。
`docs/designs/sccache-compile-cache-issue-148.md` に lane 選定・
namespace・kill switch・計測契約を定義。step 構成と cache 正しさ解析は
ruviform 側 `docs/designs/issue-329-sccache-compile-cache.md` と同型。

## Implementation

### Changes

- `.github/workflows/ci.yml` の `verify` job: cargo cache restore 直後に
  `Set up sccache`(mozilla-actions/sccache-action v0.0.11 SHA pin・
  sccache v0.19.0)/`Export sccache GHA backend env`(actions/github-script
  v9.0.0 SHA pin で credentials+`RUSTC_WRAPPER=sccache`+
  `SCCACHE_GHA_VERSION=audiolens-v1-<RUNNER_OS>` を GITHUB_ENV へ)/
  `Verify sccache environment` の 3 step、compile lane 末尾に
  `sccache statistics` を追加
- `webview-verify` job: 同じ 3+1 step を同位置に追加
  (`webview` feature + dev profile。fingerprint 差で verify と誤共有しない)
- 全 sccache step を `if: vars.CI_SCCACHE_DISABLE != '1'` で gate —
  repo variable `CI_SCCACHE_DISABLE=1` が kill switch(YAML 変更不要・
  RUSTC_WRAPPER 未生成で素の rustc)
- lane の実行 step・条件・`summary` required gate・cargo cache key・
  `cargo fetch --locked`・ruviform SHA guard は非変更(diff は追加のみ)

### Decisions During Implementation

- ui-verify は cargo 非使用のため sccache 対象外と判断(NotApplicable)
- webview-verify は repo 内 2 つ目の Rust compile lane であり、ruviform
  が両 compiling lane に適用したのと同型の判断で対象に含めた
- `SCCACHE_GHA_VERSION=audiolens-v1-<RUNNER_OS>`: repo 識別 namespace
  (cache scope は repo 単位。`gh cache list` の帰属識別用)

## Related Experiments

`docs/impl/exp/issue-148-sccache-before-after.jsonl` — Before baseline
収集済み(run 41793241364 他: rust-verify 760s・webview-verify 491s・
wall 253-345s)。Cold/Warm・kill switch の After 実測は PR run で採取予定。

## Validation

実施済み(ローカル):

- [x] `mise exec -- actionlint .github/workflows/ci.yml` — pass
- [x] `mise exec -- prek run --all-files` — 全 hook pass
- [x] ci.yml diff が sccache step 追加のみであることを確認
      (削除行 0)
- [ ] PR run: verify/webview-verify 3 OS + ui-verify + summary 成功
- [ ] After 実測: Cold/Warm・`sccache --show-stats` hit 率・総 Runner 時間
- [ ] kill switch `CI_SCCACHE_DISABLE=1` で sccache step 全 skip 検証

## Risks

- GHAC rate limit の write error(cold run)は ruviform でも 36-100% 発生。
  fail-open で CI green 維持・warm run で回復するが、AL でも発生しうる
- workflow 本体は PR run まで e2e 検証不可(#142 と同制約)

## Follow-ups

- write error が継続的に支配的なら `SCCACHE_GHA_CACHE_WRITE` 併用や
  backend 切替を評価(設計 Deferred)
- RT/実機 lane への展開は ruviform staged rollout 完了後に再評価
