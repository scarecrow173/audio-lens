# Issue #210 root cause 分析

対象 run: <https://github.com/scarecrow173/audio-lens/actions/runs/43540885602>
(workflow_dispatch, sha `0e2bfee57ead58510ead95212a2540540efc430b`
= PR #209「ci: Linuxに共通Lintを集約し3OSのNative検証を維持」の
merge commit)。失敗 job は `hygiene` と `verify (ubuntu-latest)`
のみ。他 job(verify-macos・ui-verify・webview-verify・gpui-verify・
gui-real 等)は success。`summary`/`ci-summary` は必須 job 失敗に
伴う派生失敗。

## root cause A: `hygiene` — `.pre-commit-config.yaml` の YAML parse error

### 一次ログ

`gh run view 43540885602 --repo scarecrow173/audio-lens --log-failed`
の hygiene job で、`prek run --all-files` が hook を 1 件も実行せず
起動段階で失敗:

```
line 53 column 9: invalid indentation in multiline quoted scalar
```

### 原因

PR #209 内の commit `0ea1ad6`(docs(ci): align lint-scope guidance
and protect composite action contract)が `.pre-commit-config.yaml`
のみを変更(同 commit の `--stat` で確認: 1 file changed)。
`check-ci-workflow` hook の `files:` 行更新時に:

1. single-quoted scalar の閉じ `$'` が脱落
   (`^tools/check-ci-workflow\.mjs` で終わり、`'` が無い)。
2. `check-ci-tracker-workflows` hook ブロック全体(コメント 4 行 +
   hook 定義 6 行)が誤って重複挿入され、元の
   `pass_filenames: false` 行が stray として残った。

`git diff 0ea1ad6~1 0ea1ad6 -- .pre-commit-config.yaml` で確認した
とおり、修正前は `files: '^\.github/workflows/ci\.yml$|
^tools/check-ci-workflow\.mjs$'` で閉じていた。

### 影響

prek は config 全体を先に parse するため、parse error で
`prek run --all-files` が exit 2。CI `hygiene` job とローカルの
pre-commit hook の双方が全 hook 未実行で死ぬ。

### 修復

- L52 の `files:` を
  `'^\.github/workflows/ci\.yml$|^\.github/actions/(verify|
  webview-verify|gpui-verify)/action\.yml$|^tools/check-ci-workflow
  \.mjs$'` に修正(`$'` を復元。PR #209 が意図した composite
  action path 追加は保持)。
- 重複した `check-ci-tracker-workflows` ブロック(stray
  `pass_filenames: false` + コメント + hook 定義)を除去し、
  hook 定義を 1 件に戻す。

hook の id/entry/files/pass_filenames 契約は変更していない。

## root cause B: `verify (ubuntu-latest)` — `lint-full` の `biome check` exit 1

### 一次ログ

同 run の verify (ubuntu-latest) job、`mise run lint-full` の
`biome check` step:

```
Checked 223 files in 377ms. No fixes applied.
Found 1 error.
Found 5 warnings.
Found 13 infos.
```

exit 1。診断の内訳(ローカル biome 2.6.2 で同一再現):

| 重要度 | 診断 | 箇所 |
| --- | --- | --- |
| error | format drift | `tools/check-ci-workflow.mjs` L427-430 |
| warning | `lint/suspicious/noNonNullAssertion` | `tools/cinematic-sword-cutscene/src/three/vfx/ParticleSystem.tsx:34` |
| warning | `lint/suspicious/noTemplateCurlyInString` ×3 | `tools/check-ci-tracker-workflows.mjs:42,228,232` |
| warning | `lint/complexity/useOptionalChain` | `tools/check-ci-workflow.mjs:359` |
| info | `lint/complexity/useIndexOf` ×2 | `check-ci-workflow.mjs:59`, `check-ci-tracker-workflows.mjs:59` |
| info | `lint/style/useTemplate` ×11 | `check-ci-workflow.mjs:375,376,399,412,414(×2),424,425,428,429,434` |

### なぜ今回顕在化したか

- Biome は commit `28ada9c`(chore: introduce biome lint/format
  checks)で導入済みだが、`lint-full` は PR #209 で Linux 専用
  lane に集約されるまで ubuntu leg で `biome check` が走って
  いなかった。蓄積していた診断が ubuntu leg で初めて fail した。
- `noNonNullAssertion`・format drift が error 相当、
  `noTemplateCurlyInString`/`useOptionalChain`/`useIndexOf`/
  `useTemplate` が warning/info として集計された。

### 修復方針と根拠

- `ParticleSystem.tsx:34`: `c.getContext("2d")!` →
  `const ctx = c.getContext("2d"); if (!ctx) throw new Error(...)`。
  canvas 2d context は null を返し得る。`!` でも後続の
  `createRadialGradient` で TypeError となるため、明示 Error は
  診断可能性の向上。silent fallback(テクスチャ無し継続)は
  描画を壊すため不採用。
- `check-ci-workflow.mjs`: `biome check --write --unsafe` で
  biome 提案の fix を適用し、diff 全件レビューで semantics
  保存を確認(`indexOf` 等価、`?.` は null/undefined で同値、
  template literal は同一文字列)。formatter drift も同時解消。
- `check-ci-tracker-workflows.mjs`: `useIndexOf` は同上。
  `noTemplateCurlyInString` ×3 は `${{ github.repository }}`・
  `${{ github.event.repository.default_branch || 'main' }}`・
  `${{ secrets.RUVIFORM_DEPLOY_KEY }}` を**照合対象リテラル
  として保持する必要がある**ため template literal 化しない
  (JS 式展開で `github` 等が未定義参照となり検査意味が壊れる)。
  rule off ではなく、理由付き `biome-ignore` コメントで当該
  リテラルのみ限定抑制。照合文字列は 1 文字も変更していない。

## 同種問題の棚卸し

- `.pre-commit-config.yaml`・`.github/workflows/*.yml`・
  `.github/actions/*/action.yml` の `files:`/`run:` 等の
  quote 閉鎖: `git grep "files: '[^']*$"` で unclosed single
  quote なし。`prek run --all-files` の `check-yaml` が全 YAML
  を parse して Pass。
- TS/TSX の `!` non-null assertion: repo 全体の `biome check`
  が診断 0 件で exit 0(biome の `files.includes` が
  `**/*.ts(x)` を網羅)。`git grep` でも `getContext("2d")!`
  等の残存なし。
- PR #209 の他ファイル(workflows/composite action/
  check-ci-workflow.mjs 追加節)は actionlint・check-yaml・
  両契約 script・biome で全て Pass を確認。

## ローカル再現

- 修正前: `mise exec -- biome check` → exit 1
  (error 1/warning 5/info 13、CI と同一診断)。
- 修正前: `prek run --all-files` は config parse error で
  hook 未実行(CI ログの exit 2 と同じ経路)。
- 修正後: 両コマンドとも exit 0(詳細は `verification.md`)。
