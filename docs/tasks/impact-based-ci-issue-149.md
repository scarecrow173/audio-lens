---
id: "TASK-034R69Ws8M7aodbL3V761y"
type: "task"
status: "in-progress"
title: "変更影響ベース CI・Fail-Closed summary 整合(AL#149)"
created: "2026-11-04"
updated: "2026-11-04"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/149"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/impact-based-ci-issue-149.md"
  implemented-by: []
  depends-on:
    - "../designs/impact-based-ci-issue-149.md"
    - "../designs/audiolens-independent-ci.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "https://github.com/scarecrow173/ruviform/issues/333"
    - "https://github.com/scarecrow173/ruviform/issues/327"
    - "https://github.com/scarecrow173/audio-lens/issues/142"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
    - "https://github.com/scarecrow173/audio-lens/issues/148"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/impact-based-ci-issue-149.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 変更影響ベース CI・Fail-Closed summary 整合(AL#149)

## Work

Issue [#149](https://github.com/scarecrow173/audio-lens/issues/149)
(AL-CI-OPT-02)を正本とする。AudioLens 固有の変更分類(Rust
Core/Analysis/Source/Dataset/CLI/Export/GUI/TS/UI vendor/WebView/
Cargo.lock/CI/LFS Golden)と依存伝播を Owner として定義し、
`summary` required gate が期待集合を fail-closed で検査できる
ようにする。

- `tools/classify-impact.mjs` 新規: PR diff → 分類表 →
  `verify`/`ui_verify`/`webview_verify` の
  `required`/`not_applicable` + 根拠を出力。分類不能・共有基盤・
  非 PR・fetch 失敗は安全側 Full CI
- `ci.yml`: `impact` job(分類・証拠出力)+ `hygiene` job
  (`prek run --all-files` を verify から分離・常時実行化)+
  3 lane の selective `if:` + `summary` の期待集合照合
- Shadow Mode(`CI_IMPACT_SELECTIVE` 未設定が既定)で分類のみ
  記録し、偽陰性なしを示してから選択実行を有効化する

### 2026-11-05 追加合意(macOS Required 分離)

Issue の追加合意節に従い、macOS legs を Required `summary` から
物理分離しつつ継続実行・独立記録を維持する:

- `verify`/`ui-verify`/`webview-verify` の matrix を
  ubuntu/windows に縮小し、macOS leg を `*-macos` job へ分離
  (`summary.needs`・判定対象から macOS を除外。
  `continue-on-error` や result 無視は不可)
- lane step 本体を `.github/actions/<lane>` composite action に
  抽出し、必須 legs と macOS leg で単一定義を共有
- `macos-summary` 非ブロッキング check: lane 別に
  Pass/Fail/NotRun/Unsupported/Pending/NotApplicable を記録。
  runner 滞留を NotApplicable に変換しない・実行済み失敗を
  隠さない(Fail で check 自体は失敗にするが merge 阻害しない)
- `summary` fail-closed 維持: 必須集合(ubuntu/windows legs +
  常時 job)の Failure/Cancelled/NotRun/required skipped を
  reject し続ける
- 分類器の期待集合を 3 tier に分離: PR 必須(ubuntu/windows)/
  非ブロッキング macOS(`<lane>_macos` = observe/not_applicable)/
  Full・リリース Gate。分類不能は Linux/Windows 必須 Full へ安全側
- `main-checkstatus` Ruleset は `summary` 単一 Required のまま
  (macOS 個別 contexts を Required に登録しない)
- リリース条件 runbook: macOS 対応表明の配布/正式互換受入は
  候補 SHA の macOS 実測(Golden/f64/CLI/UI/WebView 該当範囲)を
  別途必須(`docs/guides/release-macos-verification.md`)
- macOS job が queue 滞留しても rerun/cancel が他 queued job を
  殺さない構造(job 独立 + fail-fast: false 維持)

## Classification

- `normal-plan-task`

## Upstream Documents

- [design: impact-based-ci-issue-149](../designs/impact-based-ci-issue-149.md)
- [design: audiolens-independent-ci](../designs/audiolens-independent-ci.md)
- [design: sccache-compile-cache-issue-148](../designs/sccache-compile-cache-issue-148.md)
- Issue [#149](https://github.com/scarecrow173/audio-lens/issues/149)
- ruviform [#333](https://github.com/scarecrow173/ruviform/issues/333)
  (汎用 Impact 契約 Owner、本実装時点で main 未着 — Issue 本文の
  契約仕様に準拠して実装し差異を記録)

## Dependencies

- `main-protect` ruleset の required context は `summary` のみ
  (gh api 実測、2026-11-04)。名称・常時実行を維持し ruleset
  変更を不要にする
- `gh api .../pulls/N/files`(GITHUB_TOKEN、`pull-requests: read`)
  による PR diff 取得。api 失敗は Full CI fallback
- ruviform #333 の汎用契約は未着 — AudioLens 固有規約は本 repo が
  Owner(#333 §8 の Consumer 分担どおり)

## Affected Files

- `tools/classify-impact.mjs`(新規 — 分類表の唯一の正本)
- `.github/workflows/ci.yml`(`impact`/`hygiene` job 追加、
  3 lane の `if:`/`needs`、`summary` 期待集合照合、verify から
  prek step 除去)
- `docs/designs/impact-based-ci-issue-149.md` / 本 task /
  `docs/impl/ir/impact-based-ci-al-149.md` /
  `docs/impl/exp/issue-149-impact-ci-before-after.jsonl`(新規)
- `docs/designs/audiolens-independent-ci.md`(§1 paths filter
  注記・§4 lane 図・証拠語彙の追従)
- `.github/actions/{verify,ui-verify,webview-verify}/action.yml`
  (新規 — lane step 本体の単一定義、2026-11-05 追加合意)
- `docs/guides/release-macos-verification.md`(新規 — リリース
  Gate macOS 実測 runbook)
- `docs/{designs,tasks,impl/ir,impl/exp}/README.md`(index 追従)

## Done When

- [x] 変更分類→必須 lane/OS/feature の Mapping が設計 doc §2 で
      レビュー可能
- [x] `summary` が required の failure/cancelled/skipped/NotRun を
      拒否し、根拠付き NotApplicable のみ許容する
- [x] `ruviform-access`・`verify`・`ui-verify`・`webview-verify`
      (ubuntu/windows legs)を期待集合として検査可能
- [x] Shadow Mode が既定で、skip は `CI_IMPACT_SELECTIVE=1` 設定後
      のみ有効(偽陰性なし確認までは全 lane 実行)
- [x] Golden/LFS/Export/CLI/WebView の必要検証は分類表で省略
      されない(lfs-golden/cli/gui 等 → verify/webview required)
- [x] Cargo.lock/ruviform SHA・UI vendor 変更で全 lane required
      (Consumer 回帰維持)
- [x] main push / workflow_dispatch は Full CI 維持
- [x] macOS legs が `summary` の needs・判定対象から物理分離され、
      `*-macos` job として PR/main で継続実行される(追加合意)
- [x] `macos-summary` が lane 別 Pass/Fail/NotRun/Unsupported/
      Pending/NotApplicable を記録する非ブロッキング check として
      存在し、滞留を NotApplicable に変換しない
- [x] Ruleset `main-checkstatus` が `summary` 単一 Required のまま
      (macOS 個別 context 未登録)
- [x] リリース Gate で候補 SHA の macOS 実測を必須とする条件が
      runbook に記録されている
- [ ] 本 PR CI run が green かつ impact 証跡が出力される
- [ ] Before/After と Rollback が IR に記録される

## Verification

- [x] `mise exec -- actionlint .github/workflows/ci.yml`
- [x] `mise exec -- prek run --all-files`
- [x] classifier の合成 diff 検証(全 class 網羅 + `<lane>_macos`
      output の observe/not_applicable 導出)
- [x] summary Enforce ロジックの stub 検証(6 scenario)
- [x] macos-summary evaluate ロジックの stub 検証
      (queue 滞留=Pending・実行済み失敗=check fail・全成功=pass・
      observe 期待の NotRun=fail)
- [x] workflow topology 構造 assert(summary.needs 非 macOS・
      matrix 2OS・`*-macos` runs-on・composite 共有)
- [ ] PR run で全 job green + impact 証跡 + summary 期待集合表
      + macos-summary 独立記録の実証
