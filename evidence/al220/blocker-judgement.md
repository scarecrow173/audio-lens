# Issue #220 route_graph blocker 判定

## baseline(blocked 状態)の blocker 構成

`node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
--json`(main `7afabbc`、2026-11-17 実測):

```json
{
  "status": "blocked",
  "blockers": [
    "artifact-graph",
    "broken-relation:<24 件>",
    "focus-required"
  ],
  "delegate": null,
  "taskGraph": null
}
```

## 各 blocker の判定

### `broken-relation:*`(24 件)→ 解消

全件 `relations.source` が削除済み `crates/al-math/**`・
`crates/al-locate/src/dsp.rs` を指す。同一根因であり、
frontmatter を pinned blob URL へ置換して 0 件化。
`broken-relation-inventory.md` に棚卸しと分類を記録。

### `artifact-graph`→ 解消(派生 blocker)

`artifact-graph` は graphIssues(= broken-relation 等の文書 graph
問題)が非空のときに発火する派生 blocker。broken-relation 24 件が
解消され graphIssues が空になったため連動して消滅した。
route_graph のコード上、graphIssues 非空 → `artifact-graph`
blocker 付与の構造であり、個別の修復対象ではない。

### `focus-required`→ 残存(期待 terminal)

`--focus` 未指定の bare 実行では、doc-driven-dev-graph Skill の
仕様上「explicit focus が渡されるまで hard stop」が意図された
terminal である。`focus-required` は異常ではなく、focus を渡した
呼び出しで route が返ることが正常系の証拠となる。

修復後の実測:

- bare: `route_graph.js --json` → `status: "blocked"`、
  blockers = `["focus-required"]` のみ
- focus 付き:
  `route_graph.js --json --focus docs/specs/locate-plan-cache.md`
  → `status: "edge"`、`edgeId: "probe-to-briefing"`、
  `delegate: "briefing-flow"`、`taskGraph` 非 null(6 task nodes、
  全て done)、blockers = `[]`

## 結論

AC「route_graph が blocked でなくなり、正常に route を返す」は、
`--focus` 指定で `status: edge` + taskGraph が返ることで充足。
bare 実行の `focus-required` は仕様上の意図された terminal であり、
blocker ではない。これを「期待 terminal」として記録する判断は
タスク仕様の AC 解釈に従う。

## 補足

- CI workflow に route_graph job は存在しない。`status: blocked`・
  `delegate: null` が返り続ける間は doc-driven-dev-graph フロー
  依存の Worker 作業が repo 全体で止まる状態だったため、本修復で
  フローが再開可能になった。
- route_graph は `.agents/` 配下の APM 生成物であり、修復は
  正本である docs frontmatter 側で行った(AC「生成物の直接編集で
  なく正本側」を充足)。

実測値の詳細は `verification.md` を参照。
