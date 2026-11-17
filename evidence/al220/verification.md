# Issue #220 検証台帳

実行環境: Windows 11 / Orca worktree
`G:\orca\workspaces\audio-lens\al220-doc-refs` /
node(ローカル導入)/ docs-only 変更のため Rust toolchain 検証は
対象外(POLICIES.md §5 変更範囲比例原則)。

ブランチ: `fix/220-stale-doc-refs`(基点 `7afabbc` =
`origin/main`。`git fetch origin` で最新 main と一致を確認済み)。

## Passed

| コマンド | 目的 | exit | 結果 |
| --- | --- | --- | --- |
| `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json`(before) | baseline | 0 | `status: blocked`、blockers = `artifact-graph` + `broken-relation:*` 24 件 + `focus-required` |
| `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json`(after) | route 復帰 | 0 | `status: blocked`、blockers = `["focus-required"]` のみ(broken-relation 0・artifact-graph 消滅。focus-required は期待 terminal) |
| `node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --json --focus docs/specs/locate-plan-cache.md` | route 返却 | 0 | `status: "edge"`、`edgeId: "probe-to-briefing"`、`delegate: "briefing-flow"`、`taskGraph` 返却(6 nodes 全 done)、blockers = `[]` |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/discovery`(before) | baseline | 0 | 25 docs / 20 findings / 20 blocking(全件 `provenance-invalid-source`) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/discovery`(after) | lint 回復 | 0 | 26 docs / 3 findings(新規 discovery doc の `index-missing-entry` blocking warning + `orphan-index` + `orphan-navigation`。`provenance-invalid-source` 20→0) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/specs`(before) | baseline | 0 | 72 docs / 34 findings / 24 blocking(`provenance-invalid-source` 4 + `inconsistent-reciprocal-relation` 10 + `traceability-missing-upstream` 17 + `invalid-id-prefix` 3) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/specs`(after・frontmatter 最終状態で再実行) | lint 回復 | 0 | 73 docs / 33 findings / 21 blocking(`provenance-invalid-source` 4→0。新規 spec の index 系 3 件のみ残存。`derives-from: [discovery]` 配線済みで `traceability-missing-upstream` は発生せず) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/designs` | 他 dir 悪化なし | 0 | 77 docs / 19 findings / 6 blocking(baseline 16→19 は新規 design doc の index 系 3 件のみ) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/plans` | 他 dir 悪化なし | 0 | 62 docs / 23 findings / 20 blocking(baseline 20→23 は新規 plan doc の index 系 3 件のみ) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/tasks` | 他 dir 悪化なし | 0 | 195 docs / 19 findings / 17 blocking(baseline 16→19 は新規 task doc の index 系 3 件のみ) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/test-specs` | 他 dir 悪化なし | 0 | 69 docs / 13 findings / 5 blocking(baseline 10→13 は新規 test-spec doc の index 系 3 件のみ) |
| `node .agents/skills/doc-status/scripts/doc_status.js lint --dir docs/adr` | 他 dir 悪化なし | 0 | 32 docs / 3 findings / 0 blocking(baseline と同一。ADR は作成せず) |
| `rg '\.\./\.\./crates/(al-math\|al-locate/src/dsp)' docs/` | stale 残存確認 | 0 | 0 件 |
| `git diff` 全対象 9 文書 | 修復内容確認 | — | `source` の stale エントリ 24 件のみ置換。本文・他 relation・他 path(al-locate 存続 path 含む)は無変更 |
| `mise run fmt` | 軽量整形チェック | 0 | `cargo fmt --all -- --check` 差分なし(docs-only) |

## 注記: 新規 doc の index 系 finding

新規 doc chain 7 件(discovery/specs/designs/plans/tasks/test-specs/
impl-ir)は index 未登録のため、各 dir の lint で以下が残る:

- `index-missing-entry`(blocking warning)
- `orphan-index`(info)
- `orphan-navigation`(info)

いずれも docs index(`docs/**/README.md`)の再生成で解消する。
index の手編集・再生成・`index apply` はタスク仕様で禁止されており、
Coordinator が統合時に `doc-maintenance` で再生成する。index 未同期は
許容済み。

## NotRun

| 項目 | 理由 |
| --- | --- |
| `mise run check/lint/test/build <package>` | docs-only 変更。`crates/**` 未変更のため POLICIES.md §5 により対象外 |
| `*-full` 系タスク | 日常の必須検証に含めない(タスク仕様・共通契約どおり) |
| `prek run --all-files` | docs のみの変更。pre-commit hook は trailing-whitespace/EOF/YAML/TOML 等の軽量検査で、`mise run fmt` で代替可能な範囲。必要なら統合時に CI で確認 |
| merge 後 main SHA の CI 結果 | merge と CI 証跡確認は Coordinator が実施 |

## 補足

- `provenance-invalid-source` は graph の broken-relation と同根
  (source が実在ファイルでない)であり、frontmatter 修正で
  discovery 20→0・specs 4→0 を確認。
- 残存する非共通根因 finding の件数・内容・別 Issue 化要否は
  `remaining-findings.md` を参照。
