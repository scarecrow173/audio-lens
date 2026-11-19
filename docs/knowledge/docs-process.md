---
id: "KNOW-jX8vWpRzQyTbNmKfHsLd"
type: "knowledge"
status: "approved"
title: "文書運用: status ドリフト・記録規約・文書管理の教訓"
created: "2026-11-19"
updated: "2026-11-19"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 文書運用: status ドリフト・記録規約・文書管理の教訓

docs/plans・tasks・impl の整理過程で観測された文書管理の課題と、それに対処するための運用知見をまとめる。

## 観測された status ドリフト

複数の task 文書で front matter の `status` と実態が乖離していた:

- impl completed なのに task doc の status が `todo`/`in-progress` のまま(AL#98 link task、AL#100、AL#101、AL#103、AL#104、AL#140)。
- `status: done`/`completed` なのに Done When/Verification のチェックボックスが未チェックのまま(AL#98 実装 task の plan 側 Verification)。
- 実体は baseline が存在すること(後続 IR が `baseline/runs` 凍結 golden と比較)で裏付けられるのに、文書上は未完了に見える。

status ドリフトは「文書の状態」と「実装の状態」を別管理にすることの帰結であり、統合時は impl/IR の記録を事実の権威として扱い、task doc の status は drift として記録する方針を採った。

## 記録の抜け・曖昧さ

- Validation 節が空欄のまま残る task doc があった(基盤 task 群)。実行証拠は minimal-cli IR に集中していた。
- impl front matter の `source` が空(Issue リンク欠落)のケース(AL#104)。
- discovery 系の壊れた source relation(doc グラフの pre-existing blocker)は範囲外・別途対応とされた。
- `docs/discovery/remaining-issues-roadmap.md` 等に撤去済み al-viz への参照が残るが、発見時点の記録として保持される。

## 文書分割と管理の教訓

- 「Issue=実装正本、task doc=DAG 辺、plan=順序/ゲート/rollback 正本」の文書層分離(ruviform-foundation-migration)は大規模移行の追跡に有効。
- 「先行変更への帰属」で全差分を機械分類する方法論(surface/path/kind 完全一致で先行差分セットと照合)は baseline 差分管理の基礎。
- classify-diffs を issue 単位で継ぎ足す(#104 基盤→#117 版は stamp `params/ruviform` key・#170 stable_id・rev 一致の直接検証を追加)証跡管理。
- docs index(`docs/**/README.md`)は生成物であり手で編集しない。統合時に Coordinator が整備する。
- 源文書の削除を前提にする場合、出力文書は源パスではなく Issue/PR 番号で provenance を記録し、自己完結した内容とする。

## このコーパス(knowledge/evidence)への反映

本 knowledge/evidence 文書群は以下の原則に従って作成された:

- 源文書(plans/tasks/impl)の内容を正確に要約し、推測・創作を加えない。
- 源が曖昧・矛盾する箇所はその旨を明記する(status ドリフト、checkbox 未チェック、source 空欄等)。
- 「実行されたものだけを Passed と記録」し、NotRun/Unsupported/未実施を区別する。
- 実測と推測を区別し、数値は extraction agent が記録した値をそのまま転記する。
- Issue/PR 番号で provenance を記録し、源文書の相対パス参照は行わない。

関連 Issue/PR: AL#199、AL#202、AL#220、AL#235、#237、#241、#242、#243、#244。
