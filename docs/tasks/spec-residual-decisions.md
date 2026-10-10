---
id: "TASK-034dLdqYw5JFCrkMD5dSPk"
type: "task"
status: "todo"
title: "spec #1 との残差適合項目: 候補検出ステージ独立性、FLAC、Resolution.sample_rate"
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
    - "https://github.com/scarecrow173/audio-lens/issues/33"
  defers: []
  deferred-by: []
---

# spec #1 との残差適合項目: 候補検出ステージ独立性、FLAC、Resolution.sample_rate

## Work

#33 の残件を以下の原文棚卸しに沿って確認する。Issue 全体を対応単位とし、修正・文書化・承認済み defer の各結果と証拠を項目ごとに残す。複数の独立修正を実施するときは承認後に task を分割し、この対応を保持する。

## Classification

- `decision-required`
- draft plan のレビュー用候補。依頼により先に文書化するが、通常の承認済み計画由来の実装 task ではない。

## Upstream Documents

- `docs/plans/residual-fixes.md`
- `docs/specs/residual-compliance.md` AC-33
- `docs/designs/residual-fixes.md`
- Issue #33

## Dependencies

spec/ADR はチェーンの受理可能な status を満たし、design/plan と test-spec は承認済みのため status todo。

三項目それぞれの選択肢と承認結果を先に記録する。依存追加が必要なら許可を得る。仕様改訂を選ぶ場合も利用者価値・理由・対応 AC を review し、既存仕様を黙って緩和しない。

## Affected Files

- `docs/specs/audiolens-overview.md`
- `docs/specs/analysis-dataset.md`
- `docs/specs/candidate-event-model.md`
- `crates/al-analysis/src/stage.rs`
- `crates/al-dataset/src/audio.rs`
- `crates/al-dataset/src/types.rs`

## Issue Inventory (2026-10-10)

### 概要

#1(spec)の実コード再照合で確認した、spec 文言への厳密適合上の
残差項目 3 点。いずれも機能的欠落ではなく、機会があれば対応する
低優先項目。

### 1. 候補検出が独立ステージでない

spec のパイプラインは `Reference Analysis → Candidate Detection →
Expensive Analysis` の独立段。実装では `ExpensiveAnalysisStage`
内で候補抽出(`events/candidates` テーブル)→トリガ評価を fused
実行している(`crates/al-analysis/src/stage.rs`)。

順序・成果物は正しいが、「候補抽出のみ実行/再実行」ができない。
独立 StageKind を設けるか、現状を仕様として文書化するかは要判断。

### 2. FLAC エンコーディング未実装

音声中間/残差の保存は WAV のみ(`crates/al-dataset/src/audio.rs`
`WavEncoding`)。spec は "WAV/FLAC as appropriate"。大容量残差・
中間音声を多数保存するワークロードでは FLAC 圧縮が有利。

### 3. `Resolution` に sample_rate フィールドがない

`al-dataset::types::Resolution` は window/hop のみ。sample_rate は
各 record の params や manifest 経由で永続化されており値は失われ
ないが、スキーマの第一級フィールドではない。周波数軸の再解釈時に
params を掘る必要がある。

### 参照

- 監査元: issue #1
- 実装: crates/al-analysis/src/stage.rs, crates/al-dataset/src/audio.rs,
  crates/al-dataset/src/types.rs

## Steps

1. 原文の各項目を実装と既存 spec に照合し、修正か判断かを記録する。
2. 違反する契約を固定する回帰テストを追加する。
3. 対象テストを実行し、期待する失敗を確認する。
4. 所有境界で修正する。判断項目は承認後に着手する。
5. 対象検証を再実行する。
6. 項目ごとの証拠と文書を更新する。
7. 意味のある変更単位でコミットする。

## Done When

- [ ] 候補検出の独立実行、FLAC 保存、Resolution.sample_rate の各項目に、既存 spec の該当文言、実装との差、選択肢、承認結果、実装または仕様改訂先を記録する。判断前に現状を適合済みとせず、選択した契約の回帰テストを承認後に追加する。
- [ ] 原文全項目に修正・文書化・承認済み defer と検証証拠の対応がある。
- [ ] 関連 spec/ADR/design/test-spec が実施内容と一致する。

## Verification

- [ ] `mise run test` が exit 0。原文項目に対応した回帰テストを含む。
- [ ] `mise run fmt` と `mise run lint` が exit 0。
- [ ] 文書と Issue 項目の対応を review する。

現時点では文書整備のみであり、上記実装検証は未実行。
