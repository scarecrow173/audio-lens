---
id: "TASK-034P3OPfg52Iv5UK2zrr4F"
type: "task"
status: "todo"
title: "固有処理と汎用カーネルの責務分離(AL#102)"
created: "2026-10-30"
updated: "2026-10-30"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/102"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/ruviform-foundation-migration.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ruviform-foundation-migration.md"
    - "docs/tasks/ruviform-migration-baseline.md"
    - "docs/tasks/ruviform-dsp-kernel-switch.md"
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

# 固有処理と汎用カーネルの責務分離(AL#102)

## Work

Ruviform 汎用カーネルへの切替後、`al-locate`/`al-analysis` に残る
AudioLens 固有意味論と、Ruviform に渡すべきでない判断が正しく
分離されていることを構造的に確定する。Issue
[#102](https://github.com/scarecrow173/audio-lens/issues/102) を正本とする。

- `al-locate` の合議・曖昧性・loop/drift/gap・residual の意味論が
  Ruviform 型へ漏れず AL に残ることを確認・整理
- `al-analysis` の Cheap/Reference/Expensive 制御・
  FeatureRecord/Validity/Confidence/Support・端点除外等の stage
  意味論を Ruviform カーネル呼出から分離
- `al-locate::dsp` facade の存否を責務境界の観点で再判定
- 暫定 adapter/比較フラグを棚卸しし、削除条件付きで残す

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ruviform-foundation-migration.md`
- `docs/designs/ruviform-migration-responsibility-map.md` §3/§4
- Issue [#102](https://github.com/scarecrow173/audio-lens/issues/102)

## Dependencies

- `docs/tasks/ruviform-migration-baseline.md`
- `docs/tasks/ruviform-dsp-kernel-switch.md`
- 外部依存: Ruviform [#152](https://github.com/scarecrow173/ruviform/issues/152)
  /[#153](https://github.com/scarecrow173/ruviform/issues/153)
  の契約確定

## Affected Files

- `crates/al-locate`(methods/consensus/tracking/mixer/residual/
  candidate/dsp.rs の境界整理)
- `crates/al-analysis`(cheap/reference/expensive/records/events/
  band_gain/stft の意味論分離)

## Done When

- [ ] 責務マップの「残す」行に記載した固有処理がすべて AL に
      残り、Ruviform 型へ AudioLens 固有意味が混入していない
- [ ] #102 の完了条件を満たす
- [ ] 暫定 adapter/比較フラグに削除条件と関連 Issue が付記
      されている

## Verification

- [ ] 固有意味論の golden が切替前と一致
- [ ] `mise run test` 相当が通る
