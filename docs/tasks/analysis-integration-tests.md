---
id: "TASK-034z9nDLirh4B1Howag5Y1"
type: "task"
status: "todo"
title: "analysis パイプライン合成結合テスト"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/analysis-stages.md"
  implemented-by: []
  depends-on:
    - "docs/plans/analysis-stages.md"
    - "docs/tasks/analysis-cli-wiring.md"
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
    - "docs/test-specs/analysis-stages.md"
  references: []
  defers: []
  deferred-by: []
---

# analysis パイプライン合成結合テスト

## Work

合成フィクスチャによる analysis パイプライン結合テスト
(test-spec `analysis-stages.md` の全項目を網羅)。

- capture-only: ingest → cheap 完走、全ファミリの FeatureRecord
- source+capture: locate → reference で R0-R3・伝達関数・
  1/3oct M2 が spec の精度要件を満たす
- ドリフト/スキップ合成信号: ラグ・ドリフト推定が正しい
- イベント抽出: 閾値越えが events ロールに現れ、証拠リンクが
  有効
- expensive トリガ: 各モードの起動/抑制
- resume: 解析ステージ途中失敗 → resume で完了し重複書き込み
  なし
- `open_or_create` 不一致・破損 manifest の明示的失敗

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/analysis-stages.md`
- `docs/test-specs/analysis-stages.md`

## Dependencies

- `docs/tasks/analysis-cli-wiring.md`

## Affected Files

- `crates/al-analysis/tests/`(新規)
- `crates/al-cli/tests/`(E2E)

## Done When

- [ ] test-spec の全項目に対応するテストが存在し緑
- [ ] `cargo test --workspace` / clippy / fmt が clean

## Verification

- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
