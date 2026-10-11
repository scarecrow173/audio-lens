---
id: "TASK-035fPlMnOkJiHgFeDcBaZ"
type: "task"
status: "done"
title: "al-math: least_squares を Householder QR 化"
created: "2026-10-09"
updated: "2026-10-10"
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

- [x] 既知の ill-conditioned ケースで係数が安定
- [x] `condition_number` が引き続き記録される
- [x] 既存 golden(mixer/残差減算系)がパス

## Verification

- [x] `cargo test -p al-math -p al-locate`

## 実装と検証結果

- `al_math::linalg::LeastSquares` が Householder 反射と R を所有し、
  `fit_mix` は同じ分解を各キャプチャ rhs に再利用する。
- ridge は `[A; sqrt(lambda) I]` の拡大系。残差 RMS は元の観測のみ。
  有限入力、行幅、観測数、rhs 長、非負有限 lambda を検証する。
- 数値ランク判定は R の対角と `EPSILON * max(行数, 列数) *
  max(abs(拡大行列))` の閾値を使用し、非正則系は `None`。
  診断行列による条件数推定を求解の成否には使用しない。
- 条件数推定と診断用逆行列は従来の演算順序・閾値を保持して
  al-math に移管。記録対象は ridge 前の AᵀA のまま。
- N=1..8、端数長、ridge 有無で従来求解と比較し、係数の絶対誤差
  1e-8、残差 RMS の絶対誤差 1e-10 以内を確認。
  同じカーネルの scalar/SIMD 比較も係数・残差 1e-10 以内。
- 1e-7 の差を持つ高相関列で係数誤差 1e-7 未満を確認。
  mixer の符号・行列方向、複数 rhs、ridge 非依存の診断値も検証。
- `mise exec -- cargo fmt -p al-math --check`: 成功。
- `mise exec -- cargo build -p al-math`: 成功。
- `mise exec -- cargo test -p al-math -p al-locate`: 成功
  (al-math 7、al-locate 単体 32、golden 16、結合 7)。
- `mise exec -- cargo clippy -p al-math --all-targets -- -D warnings`: 成功。
- `mise exec -- cargo clippy -p al-locate --all-targets -- -D warnings`: 成功。
- `git diff --check`: 成功。
