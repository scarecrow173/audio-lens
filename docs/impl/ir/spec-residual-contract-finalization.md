---
id: "IMPL-034FPwbAz182lxE4eFKdA3"
type: "impl"
status: "completed"
title: "spec residual contract finalization"
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
    - "docs/tasks/spec-residual-contract-finalization.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# spec residual contract finalization

## Summary

#33 の2026-10-14確定事項を正式文書へ適用した。WAVのみの出力仕様、fused候補段、
独立candidate artifactと空保存を維持。sample_rate/Float64 PCM/provenanceの既存契約を再確認した。
製品コード・既存テスト・依存の変更はない。完成reviewは2026-10-15 Judge ACCEPT。commit gateはstatus/evidenceを含むcommit後に確認する。

## Context

今回task/plan/spec/ADR/designに従う。旧 TASK-034dLdqYw5JFCrkMD5dSPk のdoneと
2026-10-11のFLAC defer判断は履歴として保持し、今回の実績をそこへ混同しない。

## Implementation

### Changes

overview/dataset/storage ADR の現在出力契約をWAVへ統一。candidate specで
候補保存→trigger→expensiveの順序、独立参照、manifest/resume維持を明確化。
residual spec/ADR/design/test-spec/旧taskに日付付き新契約参照を追加した。
今回chainは各artifact1件。旧全体lineageによるfocus曖昧性を専用discovery anchorで解消。

### Decisions During Implementation

既存manual-ranges testには空candidate reader assertionがない。製品/既存test変更なしという
scopeを維持して、一時検証testを作成・実行・除去した。coordinatorがこの方針を確認した。
恒久回帰は追加していないため、将来変更でこのassertが自動実行される保証はない。

## Related Experiments

独立した探索実験はない。承認済みの文書適用と既存契約検証のみ。

## Validation

| 実行コマンド（すべて mise exec -- 経由） | 結果 |
| --- | --- |
| cargo test -p al-analysis --test expensive | 8 passed |
| cargo test -p al-analysis windowed_records_persist_the_decoded_sample_rate | 対象1 passed（他targetはfilterで0件） |
| cargo test -p al-analysis windowed_expensive_records_keep_the_analysis_rate_even_when_invalid | 対象1 passed（他targetはfilterで0件） |
| cargo test -p al-analysis --test reference normalized_alignment_survives_process_boundary | 1 passed |
| cargo test -p al-dataset feature_record_round_trips | 対象1 passed |
| cargo test -p al-locate --test locate residual_child_reopens_the_exact_subtracted_signal_and_lineage | 1 passed |
| cargo test -p al-analysis --test spec_residual_verification manual_ranges_analyze_without_any_candidate | 一時検証1 passed、7 filtered out |

対象テストは既存13件と一時検証1件。workspace全体のテスト・fmt/clippyは今回未実行。

### Empty candidate verification

`crates/al-analysis/tests/spec_residual_verification.rs` は作成前に存在しないことをassertし、
既存 expensive.rs をコピー、manual_ranges_analyze_without_any_candidate に下記assertのみ挿入。
quiet sine/manual range の既存fixtureを使い、実際のDatasetReaderで未実行と候補なしを観測した。
実行exit0後に作成したこの1ファイルだけを除去した。

stage実行前:

```rust
assert!(al_dataset::reader::DatasetReader::open(&fx.dir.join("analysis")).unwrap()
    .open_table("events", "candidates").is_err());
```

stage実行後:

```rust
let candidate = recs.iter().find(|r| r.role == "candidate_events")
    .expect("independent candidate artifact");
assert_eq!(candidate.path, PathBuf::from("analysis/events/candidates"));
let reader = al_dataset::reader::DatasetReader::open(&fx.dir.join("analysis")).unwrap();
assert_eq!(reader.open_table("events", "candidates").unwrap().row_count(), 0);
```

これにより候補なしの空テーブルが実際にreaderで開けること、独立artifact recordを返すこと、
未実行時の欠落と区別できることを確認した。既存testへの恒久追加は行っていない。

### Document and graph evidence

probe-to-briefing (audits adr/spec成功) → briefing-to-design (audit design成功) →
design-to-planning (audit design成功) → planning-to-task-graph (audits plan/task/test-spec成功、
build_task_graph issuesなし) → task-graph-to-implementation (task1件runnable、commitGate true)。
mode run-until-yield、maxHops既定10、taskBudgetCount1。各routeは1辺ずつ実行。
briefing/spec・design・planは今回独立review Judge ACCEPT（A/B候補0、Validator追加0）を記録済み。
完成reviewは2026-10-15にA/B候補0、Validator追加0、Judge ACCEPT。既存13件+一時1件の検証証拠と恒久保証なしの限界を受理。

実装edge baselineは HEAD=76c90fba0836cbe9882812e117b3b1785f26bd19 と今回chain/indexの
未commit差分。元worktreeのロードマップ差分はbaselineにも本worktreeにも取り込んでいない。
完成review後のcommitにより新規dirty差分を解消してgateを満たす必要がある。

## Risks

恒久empty-candidate assertionは追加していない。今回の一時検証結果と静的code読取は
継続的回帰保証とは異なる。圧縮形式は将来別Issue、PCM精度保持が採用条件。
#36/#52/#55/#56の実装や承認にこの完了を流用しない。

## Follow-ups

commit後にGraph gate/exit auditを確認する。今回scope外の既存ir locate-argmax-tie-eps-calibration.md は
impl-record auditで必須section欠落5件を持つため、全体ir監査成功とは主張しない。
並行#52 draftの未index登録は別担当の既知状態であり、#33の証拠と分離する。

## 2026-10-15 Completion Review

今回独立A/B候補各0、Validator追加0、Judge ACCEPT。14 passedのworker証拠と
一時検証による恒久保証なしの限界を受理。元dirty/並行#52をstageしない。
