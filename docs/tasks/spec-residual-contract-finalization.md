---
id: "TASK-034FPu36v3CdIedbBEiiFF"
type: "task"
status: "done"
title: "Issue 33 確定仕様を文書に適用し維持契約を検証する"
created: "2026-10-14"
updated: "2026-10-15"
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
  implements:
    - "docs/plans/implement-spec-residual-contract-finalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-spec-residual-contract-finalization.md"
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
    - "docs/test-specs/spec-residual-contract-finalization.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 33 確定仕様を文書に適用し維持契約を検証する

## Work

approved plan の file map に従い正式文書の現在契約を更新し、旧done判断履歴に日付と今回参照を追加する。
コード・依存・StageKind/manifest/resume/schema は変更しない。今回の実績は専用irへ記録する。

## Acceptance Criteria

- [x] spec AC-1〜AC-4 を正式文書へ適用する。
- [x] 既存candidate・sample_rate・Float64 WAV lineage回帰を実行し保証範囲を記録する。
- [x] 旧done履歴を保持、関連文書監査/index/diff checkが成功。
- [x] 完成reviewでBlockerなし、修正可能な非Blockerも対応。
- [x] 論理変更をcommitしてGraph gateを確認。

## Verification

`mise exec -- cargo test -p al-analysis --test expensive`、
`mise exec -- cargo test -p al-analysis windowed_records_persist_the_decoded_sample_rate`、
`mise exec -- cargo test -p al-analysis windowed_expensive_records_keep_the_analysis_rate_even_when_invalid`、
`mise exec -- cargo test -p al-analysis --test reference normalized_alignment_survives_process_boundary`、
`mise exec -- cargo test -p al-dataset feature_record_round_trips`、
`mise exec -- cargo test -p al-locate --test locate residual_child_reopens_the_exact_subtracted_signal_and_lineage`。
変更文書型のdoc_status audit、git diff --check。既存manual-ranges空保存assertを点検。

## Approval and Boundary

2026-10-14のplan Judge ACCEPTを基に、意味の変わらないtest-spec/task具体化は承認範囲とcoordinatorが確認。
新判断やscope変更は再review。元worktreeの未commit差分は取り込まない。

## 実施証拠

[今回ir](../impl/ir/spec-residual-contract-finalization.md)を参照。完成review Judge ACCEPT、検証証拠を含む今回論理変更をcommitする。

## 2026-10-15 Completion Review

今回独立A/B候補各0、Validator追加0、Judge ACCEPT。14 passedのworker証拠と
一時検証による恒久保証なしの限界を受理。元dirty/並行#52をstageしない。
