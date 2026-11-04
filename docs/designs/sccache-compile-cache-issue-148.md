---
id: "DESIGN-034RIK5GkdCeUVI4sbrxPb"
type: "design"
status: "draft"
title: "Issue #148 sccache コンパイルキャッシュ採用設計 — lane 選定・namespace・kill switch・計測"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/148"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
      - type: "file"
        path: "docs/impl/exp/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/329"
    - "https://github.com/scarecrow173/ruviform/pull/342"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "../designs/audiolens-independent-ci.md"
    - "../designs/ruviform-dependency-ci.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/ruviform/blob/main/docs/designs/issue-329-sccache-compile-cache.md"
    - "https://github.com/scarecrow173/ruviform/blob/main/docs/impl/exp/issue-329-sccache-compile-cache.md"
    - "https://github.com/mozilla/sccache/blob/main/docs/GHA.md"
  defers: []
  deferred-by: []
---

# Issue #148 sccache コンパイルキャッシュ採用設計 — lane 選定・namespace・kill switch・計測

## Context

AudioLens 独立 CI(#142)の `verify`/`webview-verify`(各 3 OS)は Rust
compile が支配的な lane(Before 実測: `rust` step lane = 全 Runner 時間の
約 60%。`verify` lane 760s・`webview-verify` lane 491s / run)。Ruviform
#329/PR #342 が同一構成の sccache(GHA cache backend)を verify+ui-verify
に導入済みで、verify lane hit 率 47-62%・job 時間 10-30% 削減・
fail-open・kill switch を実測確認している。本設計はその AL 側適用判断で、
step 構成と安全解析の正本は ruviform 側
`docs/designs/issue-329-sccache-compile-cache.md` を参照し、ここでは
AL 固有の差分(lane 選定・namespace・計測・保護対象との整合)のみを定義する。

## Scope

### In Scope

- `verify`(ubuntu/windows/macos)への sccache 導入 — Issue 本文が指名する
  主対象。`check-ci`/`lint-ci`/`test-ci`/`cargo build -p al-cli --profile ci`
  を含む `--profile ci` lane
- `webview-verify`(3 OS)への sccache 導入 — repo 内のもう一つの Rust
  compile lane(al-gui `webview` feature、dev profile)。ruviform が両
  compiling lane に適用したのと同型の判断。feature/profile 差は
  sccache hash 入力に含まれるため verify と誤共有しない
- `CI_SCCACHE_DISABLE` repo variable kill switch(両 lane 共通)
- Before/After 計測(ruviform `ci/metrics/collect-ci-metrics.mjs` 同一手順)、
  Cold/Warm・cache hit・総 Runner 時間の記録、Rollback 文書化

### Out of Scope

- `ui-verify` — node のみで cargo を呼ばない(sync-ui-deps/npm ci/tsc/
  happy-dom)。sccache を置いても compile 対象がなく、step 追加自体が
  無意味。対象外(NotApplicable)
- `gui-real`(placeholder)、`summary`、`ruviform-access` — 非 compile lane
- WebView/TS/UI/Golden/CLI の実行構成・実行条件の変更(保護対象)
- check/test 削除・統合(等価性未証明の削除は Issue 明令で禁止)
- Ruviform revision の freeze ・共通基盤への製品固有 logic 移管
- S3 等 GHAC 以外の backend、cargo registry cache(`actions/cache`)側の
  変更 — 既存 key/`cargo fetch --locked` 維持

## Components and Boundaries

両 lane で同一の 3 step 群を cargo cache restore 直後に挿入し、compile
lane 末尾に統計 step を置く(ruviform 実装と同形・同 SHA pin):

- `Set up sccache`: `mozilla-actions/sccache-action@
  fc920bf0ec8de6ee65d409111f7ec508035751ba`(# v0.0.11、
  `version: "v0.19.0"`)。`if: vars.CI_SCCACHE_DISABLE != '1'`
- `Export sccache GHA backend env`: `actions/github-script@
  3a2844b7e9c422d3c10d287c895573f7108da1b3`(# v9.0.0)が
  `ACTIONS_RUNTIME_TOKEN`/`ACTIONS_RESULTS_URL`/`SCCACHE_GHA_ENABLED`/
  `SCCACHE_GHA_VERSION`/`RUSTC_WRAPPER=sccache` を `GITHUB_ENV` へ export
- `Verify sccache environment`: version・env・credential presence を log
  出力(credential 値は出さない)
- `sccache statistics`: `sccache --show-stats || true`(観測のみ)

AL 固有差分:

- `SCCACHE_GHA_VERSION = audiolens-v1-<RUNNER_OS>` — repo 識別用
  namespace(cache scope は repo 単位なので ruviform と衝突しない。
  `gh cache list` での帰属識別が目的)
- 統計 step の位置: verify では `check-cli-headless` 後・
  `Cleanup credential` 前、webview-verify では `test-ui-webview` 後・
  `Cleanup credential` 前

`RUSTC_WRAPPER` は job env に置かず `GITHUB_ENV` 経由にする — kill switch
発動時は export step 自体が skip され、変数が「存在しない」状態で素の
rustc が走る(空文字 export という中途半端な状態を作らない。ruviform
設計 D 項と同じ判断)。

## Cache 正しさ(誤共有の構造的排除)

sccache の key は compiler 実行ファイル hash + 完全な cmdline 引数 +
preprocess 済み source + 関連 env の hash。よって以下は全て key 相違:

- OS/arch、toolchain version(mise pin rust 1.99.0)
- profile(`ci` vs dev — webview-verify は dev)
- feature(verify 既定 vs `webview`)、RUSTFLAGS(未設定)
- `Cargo.lock` 更新 → 変更 crate は source hash 変化(部分 hit)、
  未変更は hit 継続
- Ruviform 候補 SHA — git checkout source が変われば entry も変わる。
  `check-ruviform-rev`/`check-ruviform-lock` の SHA guard は変更しない

cache backend は correctness の権威にならない — hit 成果物は sccache が
検証し、破損・不整合は miss → 再 compile。Golden/f64/LFS/Export/
headless 境界の検出 lane(steps)は一切変更しない。

## 計測契約

- ツール: ruviform `ci/metrics/collect-ci-metrics.mjs` を
  `--repo scarecrow173/audio-lens --workflow ci` で実行(同一手順)
- Before: 導入前 run の timing + job log cache 解析
- After: PR 初回 push = Cold(namespace 未存在)、2 回目 push = Warm、
  `sccache --show-stats`(Post step)で hit/miss/write error を採取
- 採否基準: warm run の compile lane job 時間・総 Runner 時間が Before
  比で改善すれば採用。悪化なら不採用として根拠を記録(Issue 実装範囲 5)
- `CI_SCCACHE_DISABLE=1` 設定下の rerun で 4 sccache step 全 skip を検証
  (検証後に variable 削除)

## Risks and Trade-offs

| リスク | 影響 | 対応 |
| --- | --- | --- |
| GHAC rate limit の write error(cold run 集中書込) | cache 未保存で hit 率低下。compile は fail-open で継続 | ruviform 実績でも発生・CI green 維持を確認済。stats で観測 |
| 6 compiling leg(verify+webview-verify × 3 OS)の同時書込 | 上記 rate limit を増幅 | warm run で回復。継続するなら kill switch |
| credential 不在・cache service 障害 | `SCCACHE_GHA_ENABLED=false` → local disk fallback または fail-open | env check step の `::warning::` で可視化 |
| 破損 entry | content hash で誤参照しない。miss → 再 compile | `gh cache delete` / version bump |

## 失敗モードと無効化手順

| 失敗モード | 検知 | 無効化/復旧 |
| --- | --- | --- |
| 一時的な backend 障害 | stats の storage error | 放置(fail-open)。継続なら `CI_SCCACHE_DISABLE=1` |
| 恒久無効化 | — | Settings → Variables `CI_SCCACHE_DISABLE=1`(merge 不要・次 run から sccache step 全 skip・素の rustc) |
| 完全 rollback | — | `git revert` で step 除去。永続形式・依存追加なし(cache は自然失効) |

## Deferred Design Concerns

- ui-verify への適用は compile 対象不在のため恒久対象外。将来 ui-verify
  が Rust compile を持つ構成になった時点で再評価
- RT/GUI/実機 lane への展開は ruviform 側 staged rollout の完了と AL
  側実測を待って follow-up
- `SCCACHE_GHA_CACHE_WRITE` 併用や backend 切替は write error が継続的に
  支配的になった場合の評価余地

## Verification Notes

- [ ] `mise exec -- actionlint .github/workflows/ci.yml`
- [ ] warm run で verify/webview-verify の hit 率と job 時間改善を
      `sccache --show-stats` + 計測ツールで確認
- [ ] `CI_SCCACHE_DISABLE=1` で sccache step 全 skip + job success
- [ ] `summary` required gate・全 lane の実行条件が非変更であること
      (diff で lane step 追加以外の差分がないこと)
