---
id: "TASK-034dLdqYvyaMjl7hTgg6IT"
type: "task"
status: "todo"
title: "al-locate: 2ソース残差探索の残差PCMを保存する"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  implements:
    - "docs/plans/residual-fixes.md"
  implemented-by: []
  depends-on:
    - "docs/plans/residual-fixes.md"
    - "docs/designs/residual-fixes.md"
    - "docs/adr/residual-issue-boundaries.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/residual-fixes.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/32"
  defers: []
  deferred-by: []
---

# al-locate: 2ソース残差探索の残差PCMを保存する

## Work

#32 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `new-feature`
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-32
- `docs/designs/residual-fixes.md`
- Issue #32

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

## Affected Files

- `crates/al-locate/src/stage.rs`
- `crates/al-locate/src/residual.rs`
- `crates/al-dataset/src/audio.rs`
- `crates/al-locate/tests/locate.rs`

## Issue Inventory (2026-10-10)

### 概要

ロードマップ監査(#5 照合)で検出。`al-locate` の two-source 残差探索
(`crates/al-locate/src/stage.rs` の `residual_search` 呼び出し)は
`residual_<parent>.json` として以下を保存している:

- parent_source / parent_alignment / child locate result
- channel_transform(符号付き混合行列)
- 削減前後エネルギー / confidence

しかし減算後の**残差 PCM 自体は保存されず破棄**される。issue #5 の
「残差は反復(iteration)とレビューのために保持する」要件に対し、
メタデータは満たすが残差信号の再聴取・再解析ができない。

reference 解析側では staged residual R0–R3 の音声が `residual` ロールに
保存済み(`crates/al-analysis/src/reference.rs`)なので、保存機構は
既存の `residual` ロール + audio store を再利用する形で足せる。

### スコープ

- residual_search 後の残差 channel samples を `residual` ロールの
  WAV/audio artifact として保存(親ソース ID 連動の命名)
- search JSON から残差 audio artifact への参照を lineage/params に追加
- 残差 child alignment は探索に実際に使用した減算後残差 PCM を
  独立した signal version として保存し、`capture_pcm` で明示参照する。
  `read_alignment_pcm` 等の再取得経路が探索時と同一信号を返すようにし、
  減算前の元キャプチャ normalized_pcm を使用 PCM として参照しない。
  参照欠落・不整合はエラーとし、元 PCM への暗黙 fallback を禁止する
- 必要なら残差の簡易スペクトル特性(PSD/帯域)を observation として併記

### 参照

- 監査対象: issue #5(残差検索・混合フィット)
- 実装: crates/al-locate/src/stage.rs, crates/al-locate/src/residual.rs
- 既存機構: crates/al-dataset/src/audio.rs, al-analysis residual 保存

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [ ] two-source 探索の残差 PCM を保存して再読出し・再聴取できる。search JSON から音声 artifact と親 source/alignment、混合行列の来歴を辿れ、保存前後のサンプル・エネルギーが一致する。
- [ ] 残差 child alignment の `capture_pcm` が探索に実際に使用した減算後残差の signal version を明示参照する。dataset 再 open 後も `read_alignment_pcm` 等が探索時と同一信号を返し、参照欠落・不整合では fail-closed でエラーになる。
- [ ] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [ ] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [ ] `mise exec -- cargo test -p al-locate -p al-dataset` が exit 0。原文項目に対応した回帰テストを含む。
- [ ] `mise run fmt` と `mise run lint` が exit 0。
- [ ] 文書と Issue 項目の対応を review する。

現時点では文書整備のみであり、上記実装検証は未実行。
