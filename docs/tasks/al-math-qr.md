---
id: "TASK-035fPlMnOkJiHgFeDcBaZ"
type: "task"
status: "todo"
title: "al-math: least_squares を Householder QR 化"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/plans/simd-dsp-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/al-math-skeleton.md"
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
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/43"
  defers: []
  deferred-by: []
---

# al-math: least_squares を Householder QR 化

## Work

現行 `dsp::least_squares`(正規方程式+Gaussian elimination+ridge)
を QR 分解(Householder)ベースに置き換え、ill-conditioned な
問題での数値安定性を改善する。

- `al-math` に QR 分解を実装(小行列 N≤8 想定、LAPACK 不要)
- `least_squares` を A^T A 正規方程式ではなく QR 経由に変更
- `condition_number` の記録は継続
- `fit_mix`(mixer フィット)の出力が ill-conditioned ケースで
  安定することを確認(golden: 相関の高いチャンネル)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`

## Dependencies

- `docs/tasks/al-math-skeleton.md`

## Affected Files

- `crates/al-math/src/linalg.rs`(新規)
- `crates/al-locate/src/dsp.rs`(least_squares を al-math 経由に)
- `crates/al-locate/src/mixer.rs`(fit_mix の求解差替え)

## Done When

- [ ] 既知の ill-conditioned ケースで係数が安定
- [ ] `condition_number` が引き続き記録される
- [ ] 既存 golden(mixer/残差減算系)がパス

## Verification

- [ ] `cargo test -p al-math -p al-locate`
