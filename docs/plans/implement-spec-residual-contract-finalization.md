---
id: "PLAN-034FPmSqSPhY2WlFZJB0VY"
type: "plan"
status: "completed"
title: "Issue 33 確定仕様を正式文書へ適用する"
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
    - "docs/specs/spec-residual-contract-finalization.md"
  implemented-by:
    - "../tasks/spec-residual-contract-finalization.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/spec-residual-contract-finalization.md"
    - "docs/designs/spec-residual-contract-finalization.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/spec-residual-contract-finalization.md"
  references: []
  defers: []
  deferred-by: []
---

# Issue 33 確定仕様を正式文書へ適用する

## Goal and Scope

#33 の今回 approved spec / accepted ADR / approved design を正式仕様に反映する。
変更は文書のみ。既存動作の保持を既存テストで確認する。新依存・StageKind・manifest/resume
変更・sample_rate再実装はしない。旧 done task と旧判断履歴を保持する。

## File Map

- specs/audiolens-overview.md、analysis-dataset.md: 残差・中間音声出力を WAV のみへ。
  Float64値と来歴、decoded.f64le 正本との役割の違いを維持する。
- specs/candidate-event-model.md、overview: fused順序・独立候補artifact・空保存を明記。
- adr/analysis-dataset-storage.md: 現行の音声保存選択と検証を WAV に整合。旧選択は日付付き履歴。
- specs/residual-compliance.md、adr/residual-issue-boundaries.md、designs/residual-fixes.md、
  test-specs/residual-fixes.md、tasks/spec-residual-decisions.md: 2026-10-14 の変更履歴と
  今回契約への参照を追加。旧 done/承認/検証結果は書き換えない。
- 今回 test-spec/task/impl record と各index: 新規1件ずつに契約・実績を集約。
- discovery/remaining-issues-roadmap.md: #33 の今回完了証拠リンクと状態のみ更新。
  元worktreeの未commit整合修正は取り込まない。

## Sequence and Task Breakdown

1. 本 plan を review-protocol でレビューし、Blockerなしなら承認扱いにする。
2. 承認後、今回AC維持の test-spec を1件、正式文書適用の task を1件作成する。
3. task を実行し、上記文書の現在契約を改訂する。旧判断には日付と今回参照を付ける。
4. 下記検証を実行し、結果を今回 impl record に記録する。
5. 完成差分を今回レビューし、修正可能な非Blockerも修正する。
6. 論理変更を Conventional Commit で commit。Graph commit gate を確認する。

依存順序は plan承認→test-spec→task→適用→検証→完成review→commit の1本。
未承認draftからtaskを生成しない。旧done taskを再開しない。

## Verification Matrix

| AC | 確認方法 |
| --- | --- |
| AC-1 WAVのみ・精度/来歴 | 現行 spec/storage ADR の文言監査。`mise exec -- cargo test -p al-locate --test locate residual_child_reopens_the_exact_subtracted_signal_and_lineage` |
| AC-2 fused・候補独立/空保存 | `mise exec -- cargo test -p al-analysis --test expensive` と stage.rs 446–504 の所有境界確認。既存テストの空保存assertを点検し、静的確認と動的確認を区別 |
| AC-3 sample_rate | `mise exec -- cargo test -p al-analysis windowed_records_persist_the_decoded_sample_rate`、`mise exec -- cargo test -p al-dataset feature_record_round_trips`。既存欠落拒否assertも確認 |
| AC-4 文書/履歴/index | 変更したspec/ADR/design/test-spec/task/impl-recordの doc_status audit、`git diff --check`、今回review証拠 |

動的確認が維持契約を保証しない場合は不足を明記し、実装ACを完了扱いにしない。
コード変更が必要と判明した場合は docs-only scope外としてGraphのrepair routeと再reviewへ戻す。

## Risk and Rollback

旧FLAC defer文言は歴史的記録として残すが、現在要件と区別する。
将来の圧縮検討条件は容量・時間・I/O実測と解析精度保持。
文書のみで外部状態の変更はないため、差分を逆適用すると旧文書へ戻せる。
既存未commit編集や旧成果物は変更しない。

## Implementation Handoff and Review

ユーザーはオーケストレータからサブエージェントへの全実作業委譲を承認済み。
実行者1名が文書適用・既存検証を担い、独立reviewerが判断/完成差分を確認する。
未決要件や未承認依存はない。実装コードを変更しないことが前提。
本planのreview対象は作業範囲・維持契約・検証条件であり、検証結果を事前に承認しない。

## Self Review

上流 AC-1〜AC-4 を上表に対応させた。今回だけのchainを選び旧計画を取り込まない。
1件の docs-only task にまとめるため、無関係な文書整備や別Issue実装は含めない。

## 2026-10-14 Plan Review

Reviewer A/B candidates各0、Validator追加0、diff check/upstream整合確認を経てJudge ACCEPT。
coordinatorが計画のみ承認、実行結果は未検証と通知。意味の変わらないtask/test-spec具体化は
今回承認範囲。scope/判断が変わる場合は再reviewする。

## 2026-10-15 Completion Review

今回独立A/B候補各0、Validator追加0、Judge ACCEPT。14 passedのworker証拠と
一時検証による恒久保証なしの限界を受理。元dirty/並行#52をstageしない。
