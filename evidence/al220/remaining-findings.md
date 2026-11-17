# Issue #220 残存 finding 記録と別 Issue 化判断

対象外 finding(本 Issue の共通根因 = `provenance-invalid-source` /
`broken-relation` ではないもの)の件数・内容と、別 Issue 化の要否
判断、同型 stale 参照の再発防止に関する結論を記録する。

## 残存 finding の件数(before → after)

| dir | before findings | after findings | 増分の内訳 |
| --- | --- | --- | --- |
| docs/discovery | 20 | 3 | -20 provenance-invalid-source +3 新規 doc index 系 |
| docs/specs | 34 | 33 | -4 provenance-invalid-source +3 新規 spec index 系 |
| docs/designs | 16 | 19 | +3 新規 design doc index 系 |
| docs/plans | 20 | 23 | +3 新規 plan doc index 系 |
| docs/tasks | 16 | 19 | +3 新規 task doc index 系 |
| docs/test-specs | 10 | 13 | +3 新規 test-spec doc index 系 |
| docs/adr | 3 | 3 | 変化なし(ADR 未作成) |

増分はすべて新規 doc chain 7 件に伴う index 系 finding
(`index-missing-entry` blocking warning + `orphan-index` +
`orphan-navigation`)であり、index 再生成で解消する許容済みのもの。
それ以外の既存 finding は悪化していない(baseline 以下)。

## 非共通根因 finding の内容(修復前 baseline、全件対象外)

- `inconsistent-reciprocal-relation`(specs 10・designs 複数):
  relation の相互リンク不整合。例: `refines` → 相手が `refined-by`
  を持たない、`depends-on` → 相手が `blocks` を持たない。
- `traceability-missing-upstream`(specs 17・designs 2・tasks 1):
  spec/design/task が upstream relation(`implements`/`derives-from`/
  `refines`)を持たない。rev-watch・issue-* 系の spec に集中。
- `invalid-id-prefix`(specs 3・designs 1・tasks 複数・test-specs 1):
  旧形式 ID(`SPEC-034jbVI...` 等)が現行 prefix 規約と不一致。
- `invalid-front-matter`(plans・tasks): `relations.changes` の
  スキーマ不一致(string vs object、array vs object)。
- `invalid-status`(tasks): `approved`/`completed` 等の task status
  が語彙外。
- `self-relation`(designs): 自分自身を指す relation。
- `stale-upstream`(test-specs・adr): in-flight 文書が完了済み
  upstream を derives-from/refines している。
- `plan-missing-test-spec-evidence`(plans): `verified-by` で
  test-spec をリンクしていない plan。
- `test-spec-invalid-verifies-target`・`test-spec-missing-verifies`
  (test-specs): verifies の対象 type 不正・未設定。

## 別 Issue 化の要否判断

**別 Issue 化は不要(現時点)と判断する。** 根拠:

- これらは本 Issue(#220)の根因(stale `source` 参照)とは無関係の
  既存 finding であり、route_graph の blocked 状態には寄与して
  いない(broken-relation は source のみを対象とし、他 relation の
  不整合は graphIssues に含まれない — 修復後 `artifact-graph` が
  消えたことで実証)。
- 大半は rev-watch / issue-* 系 spec の upstream 未配線や旧 ID
  prefix であり、doc chain 全体の棚卸し・一括修復が適する性質。
  単発の修復ではなく、docs 全体の relation 健全性監査として
  まとめて扱う方が効率的。
- 既に同種の棚卸しが今後の docs メンテナンスで扱われる可能性が
  ある。必要になった時点で Issue 起票を提案する。

ただし、件数が 60 件超(specs 33 + designs 19 + plans 23 +
tasks 19 + test-specs 13 + adr 3 の合計から index 系を除いた
概算)あり、放置すると lint 出力のノイズが増大する。Coordinator
判断で「docs relation 健全性の棚卸し」Issue を起票することを
推奨事項として PR 本文に記載する。

## 同型 stale 参照の再発防止

**結論: ツール・CI への追加組み込みは不要。運用 checklist での
予防を推奨。**

根拠:

- 検出は既に 2 系統で機能している:
  - `doc_status.js lint` の `provenance-invalid-source`
    (source が実在ファイルでない)
  - `route_graph.js` の `broken-relation:*`
- 今回の障害は「検出はされていたが、クレート削除 commit 時に
  docs frontmatter の棚卸しが行われず、blocked 状態が長期間
  放置された」ことが実質的な原因。検出器の欠如ではない。
- 再発防止として有効なのは、クレート削除・ファイル移動を伴う
  commit の checklist に「docs frontmatter `source` の棚卸し」を
  含める運用。AGENTS.md/POLICIES.md への明記は本 slice の範囲外
  のため、必要なら別 Issue で提案する。

## index 系 finding について

新規 doc chain 7 件に伴う `index-missing-entry`・`orphan-index`・
`orphan-navigation` は index 再生成で解消する。Coordinator が
統合時に `doc-maintenance` の `index apply` で再生成するため、
本 Worker では対処しない(タスク仕様どおり)。
