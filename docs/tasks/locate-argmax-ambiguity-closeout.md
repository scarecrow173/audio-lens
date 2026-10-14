---
id: "TASK-034Fo9fbMBjYA189SBNMGL"
type: "task"
status: "done"
title: "argmax 曖昧性: 回帰計測・Implementation Record・closeout"
created: "2026-10-13"
updated: "2026-10-14"
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
    - "docs/plans/implement-locate-argmax-ambiguity.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-locate-argmax-ambiguity.md"
    - "docs/tasks/locate-spectral-envelope-corr0.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/locate-argmax-ambiguity.md"
    - "docs/test-specs/locate-argmax-tie-eps-calibration.md"
  references: []
  defers: []
  deferred-by: []
---

# argmax 曖昧性: 回帰計測・Implementation Record・closeout

## Work

plan T4(計測・IR・closeout)。T1–T3 の成果を回帰計測し、
Implementation Record(`docs/impl/ir/`)へ確定記録して plan を
完了させる。

- **回帰計測**: `cargo test -p al-locate`・`cargo test -p al-math`
  の全 green を確認し、関連 bench(al-locate の `bench_*` examples
  規約に存在するもの)で `argmax_tie_count` の O(n) 走査2回が
  NCC 本体に比し無視できることを確認する(design Non-Functional
  Constraints「性能」)。golden 全件の pass を再確認
- **Implementation Record 確定**: T1 の実測(fixture 毎の
  メソッド別 `M − max_{i≠argmax}|corr[i]|` マージン分布、
  scalar↔dispatch 偏差上限、採用した `TIE_REL_EPS`/`TIE_ABS_FLOOR`
  と根拠、不安定帯 `eps ± dispatch_deviation` の帯幅)、T2 の
  意図的契約変更(reject_reason 文言・`tests/prepared.rs` の
  意味論シフト・version `"2"` バンプ)、T3 の corr0 置換差分
  (ゼロノルム扱い等)と構造的拒否の確認結果を記録する。
  ファイル名・分割は既存 IR 規約(`docs/impl/ir/` の既存
  レコード)に倣う
- **doc 更新**: 変更に追随が必要な文書の整理(残存許容の記録を
  持つ `docs/tasks/al-locate-vec-migration.md` ・
  `docs/impl/ir/al-locate-dsp-re-export-vec-migration.md` への
  解消済み注記など、status/記録更新が妥当なもの)
- **closeout**: `git log` で T1(特性計測)→ T2(実装)→ T3(corr0)
  のコミット順序が計画通りであることを確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-locate-argmax-ambiguity.md`(T4)
- `docs/designs/locate-consensus-argmax-ambiguity.md` §8(T4 輪郭)
- `docs/specs/locate-consensus-ambiguity-contract.md`(全 AC の
  最終確認)
- 検証仕様: `docs/test-specs/locate-argmax-ambiguity.md`、
  `docs/test-specs/locate-argmax-tie-eps-calibration.md`

## Dependencies

- `docs/tasks/locate-spectral-envelope-corr0.md`(T3): 全実装の
  完了が前提

## Affected Files

- `docs/impl/ir/<record>.md`(Implementation Record 確定)
- 追随が必要な関連 doc(残存許容記録の解消注記等)

## Done When

- [ ] 全テスト・golden が green で、回帰計測(bench)に
  `argmax_tie_count` 起因の有意な劣化がない
- [ ] Implementation Record に T1 実測値・eps 確定値・不安定帯・
  T2/T3 の変更内容と意図的契約変更の理由が記録される
- [ ] 残存許容(`methods.rs` 逐次維持決定)が解消済みとして
  記録に反映される
- [ ] plan の各 task が git 履歴上 T1→T2→T3 の順に land している

## Verification

- [ ] `mise exec -- cargo test --workspace` が green
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `mise exec -- cargo test -p al-locate --test golden` が green
- [ ] Implementation Record が `docs/impl/ir/` に存在し実測値を
  含む(目視)
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type
  all --cwd .` が findings なし(文書整合の最終確認)
