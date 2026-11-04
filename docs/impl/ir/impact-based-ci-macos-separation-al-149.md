---
id: "IMPL-MHRCXMYN67HF1BGACD9P"
type: "impl"
status: "completed"
title: "macOS leg の Required 物理分離(AL#149 追加合意)"
created: "2026-11-05"
updated: "2026-11-05"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/149"
  changes:
    added:
      - type: "code"
        path: ".github/actions/verify/action.yml"
      - type: "code"
        path: ".github/actions/ui-verify/action.yml"
      - type: "code"
        path: ".github/actions/webview-verify/action.yml"
      - type: "doc"
        path: "docs/guides/release-macos-verification.md"
      - type: "doc"
        path: "docs/impl/exp/issue-149-macos-separation.jsonl"
    modified:
      - type: "code"
        path: ".github/workflows/ci.yml"
      - type: "code"
        path: "tools/classify-impact.mjs"
      - type: "doc"
        path: "docs/designs/impact-based-ci-issue-149.md"
      - type: "doc"
        path: "docs/designs/audiolens-independent-ci.md"
      - type: "doc"
        path: "docs/tasks/impact-based-ci-issue-149.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/impact-based-ci-issue-149.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/impact-based-ci-issue-149.md"
    - "docs/designs/audiolens-independent-ci.md"
    - "docs/impl/ir/impact-based-ci-al-149.md"
    - "https://github.com/scarecrow173/audio-lens/issues/147"
  refines:
    - "docs/impl/ir/impact-based-ci-al-149.md"
  refined-by: []
  derives-from:
    - "docs/designs/impact-based-ci-issue-149.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
---

# macOS leg の Required 物理分離(AL#149 追加合意)

Issue #149 の 2026-11-05 追加合意に従い、macOS legs を Required
`summary` の needs・判定対象から物理分離し、継続実行・独立記録を
維持する変更を実施した。

## 変更内容

### `.github/workflows/ci.yml`

- `verify`/`ui-verify`/`webview-verify` の matrix を
  `[ubuntu-latest, windows-latest]` に縮小。check context 名は
  従来どおり `<lane> (<os>)`
- `verify-macos`/`ui-verify-macos`/`webview-verify-macos` を新設
  (`runs-on: macos-latest`、needs: ruviform-access + impact、
  `if:` は `needs.impact.outputs.<lane>_macos == 'observe'`
  または impact 失敗時 fallback で実行)
- `macos-summary` job を新設(needs: impact + 3 macos jobs、
  `if: always()`)。lane 別に Pass/Fail/NotRun/Unsupported/
  Cancelled/NotApplicable を分類器期待値 `<lane>_macos` と照合し
  `GITHUB_STEP_SUMMARY` に記録。実行済み failure/cancelled・
  `observe` 期待の NotRun は check を exit 1 にする(非ブロッキ
  ングだが Fail を隠さない)。leg の queue 滞留中は check 自身が
  Pending のまま残る(NotApplicable へ変換しない)
- `summary` の needs は
  `[ruviform-access, impact, hygiene, verify, ui-verify,
  webview-verify, gui-real]` — macOS job を含まない。
  期待集合照合の対象は ubuntu/windows legs + 常時 job のみで
  fail-closed を維持
- lane step 本体を composite action に抽出(`uses:
  ./.github/actions/<lane>`)。必須 legs と macOS leg が同一
  定義を共有し、表現ドリフトを防止

### `.github/actions/{verify,ui-verify,webview-verify}/action.yml`

- lane の step 列を composite action 化。composite 内では
  `secrets`/`vars`/`needs`/`matrix` context を参照できないため、
  `os`/`access_mode`(required)と `deploy_key`/`sccache_disable`
  (optional,default ""/"0")を input で受け取る
- `resolved_sha` output を `steps.guard.outputs.resolved_sha` で
  宣言し、job 側の既存 `steps.lane.outputs.resolved_sha` 参照を
  そのまま維持
- 全 run step に明示 `shell:` を付与(composite では job 側の
  `defaults.run.shell` が効かないため)

### `tools/classify-impact.mjs`

- `<lane>_macos` output を追加: lane が `required` のとき
  `observe`、`not_applicable` のとき `not_applicable`
  (mode=full/fallback/分類不能では全 lane observe)
- 証拠表を tier 別列に拡張:
  `| lane | classified | PR required (ubuntu/windows) |
   macOS leg (non-blocking) | reason |`

## 検証

- `actionlint .github/workflows/ci.yml`: pass
- `prek run --all-files`: pass(biome 自動整形を含む全 hook)
- 分類器合成 diff: docs-only/CI 変更/core/UI-only/未知 path/
  `--selective=0`/非 PR/fallback を網羅、期待集合(3 tier)が
  正しく出力されることを確認
- 構造 assert: `summary.needs` に macOS job を含まない、
  `macos-summary.needs` が impact + 3 macOS jobs、必須 matrix が
  2OS、全 `*-macos` が `runs-on: macos-latest` かつ
  `<lane>_macos` gating、composite 共有と required input 受渡し
- ゲート stub 検証: macOS 滞留(Pending)/macOS 失敗/
  Linux・Windows 失敗/全成功/分類不能を網羅
- **CI 自己防御**: 本 PR は `.github/**` + `classify-impact.mjs`
  変更のため mode=full で全 lane required(全 leg 実行)

## 実 run 観測(After)

- run 41879121142(PR #157 初回、Full 分類): `*-macos` 3 job が
  `macos-latest` で実実行され全 Pass → `macos-summary` が lane 別
  `observe/success → Pass` を独立記録。`hygiene` の prek
  actionlint が SC2006(echo 内未 escape backtick)で失敗し
  Required `summary` は fail-closed で正しく reject(ubuntu
  runner 同梱の shellcheck が有効化される環境差分。修正:
  6bb69dc)
- run 41881190611(修正後): Required 集合が全緑 —
  `ruviform-access`/`impact`/`hygiene`/`gui-real` + 3 lane の
  ubuntu/windows leg 全 success。**`summary` が 3 秒で Enforce
  pass した時点で `*-macos` 3 job は全て queued のまま** —
  要件のシナリオ(a)「macOS 長時間 queued/未開始でも summary が
  待機なしで完了可能」を実 run で実証。`macos-summary` は legs
  未完の間 Pending のまま残り、滞留を NotApplicable に変換しない
  設計どおりの挙動
- impact 証跡: `mode=selective, classes=ci,docs,ci-scripts` —
  `.github/**` + `classify-impact.mjs` 変更で全 lane required が
  正しく強制された(CI 自己防御の実証)

## Rollback

- 構造復旧: 本変更を `git revert` すれば OS 混在 matrix +
  `summary.needs` 旧構成へ戻る。`*-macos`/`macos-summary` job と
  `.github/actions/` は revert で同時に除去される
- 即時部分復旧: macOS leg のみ止めたい場合は各 `*-macos` job の
  `if:` を `false` にするか、分類器の `<lane>_macos` を
  `not_applicable` 固定に戻す(Required gate は影響を受けない)
- Ruleset は変更していないため復旧時の無保護期間は発生しない
  (`summary` 単一 Required が全期間で有効)

## 残件

- なし(NotRun 明記: gui-real は従来どおり not_run placeholder。
  run 41881190611 の `*-macos` legs は queued のまま後続 push の
  concurrency cancel で中断され得る — Required gate に影響なし)
