---
id: "PLAN-034GNwfwp4fXb0aKjI0LOC"
type: "plan"
status: "completed"
title: "local_lag FFT 厳密再現化の実装計画"
created: "2026-10-16"
updated: "2026-10-16"
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
    - "docs/specs/local-lag-overlap-corr-fft.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/local-lag-overlap-corr-fft.md"
    - "docs/designs/local-lag-fft.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/local-lag-overlap-corr.md"
  references: []
  defers: []
  deferred-by: []
---
# local_lag FFT 厳密再現化の実装計画 Implementation Plan

## Goal

`local_lag` の ±`lag_max` 全走査を、同一推定量を返す
`OverlapCorrPlan`/`OverlapCorrWorkspace`(FFT 分子 + prefix 二乗和
分母)に置き換え、reference_analysis の支配コストを削減する。

## Scope

単一ストリーム。al-math の新カーネル → dsp facade → al-analysis
呼出までを1本で実装する。

## Upstream Documents

- `docs/specs/local-lag-overlap-corr-fft.md`
- `docs/adr/lag-al-math-plan.md`

## Design Inputs

- `docs/designs/local-lag-fft.md`

## File Map

- `crates/al-math/src/correlation.rs` — `OverlapCorrPlan`/
  `OverlapCorrWorkspace`/`overlap_corr_curve` の追加と
  scalar↔dispatch テスト
- `crates/al-locate/src/dsp.rs` — 新規型の re-export(facade)
- `crates/al-analysis/src/reference.rs` — `local_lag` を新カーネル
  経由に置換、plan を窓ループ外で構築
- `crates/al-analysis/tests/`(既存パターン)または in-file テスト —
  `corr_at` 逐点比較、`ref.lag_*` 契約の維持
- `crates/al-analysis/examples/` または bench — 窓サイズ別の
  直接 vs FFT 交差計測

## Implementation Sequence

- [ ] S1: `OverlapCorrPlan`/`OverlapCorrWorkspace`/`overlap_corr_curve`
  を `al_math::correlation` に追加(借用曲線・dispatch・shape 検証)
- [ ] S2: `corr_at` 逐点比較テスト + scalar/dispatch 偏差テストを追加
- [ ] S3: `dsp` facade に re-export、`reference.rs` の `local_lag` を
  plan 再利用形に切替え
- [ ] S4: 窓サイズ別 bench で直接法との交差を実測し、小窓 dispatch
  の要否を記録(必要なら閾値を導入)

## Dependencies and Risks

- `RealFftPlan`/`RealFftWorkspace`/`dispatch`/`WithSimd` — 既存基盤
- 近接 peak の argmax 入替リスク — peak 一致+相対誤差の評価規約
  (#59 tie 処理に委譲)
- 小窓交差は推測でなく S4 の実測で判定

## Task Breakdown

- T1: OverlapCorr カーネル実装(S1)
- T2: 逐点比較・dispatch 同等性テスト(S2)
- T3: local_lag 移行と plan 再利用(S3)
- T4: 小窓計測と閾値判定、検証記録(S4)

## Verification

- [ ] `cargo test -p al-math`(新規 OverlapCorr テスト含む)
- [ ] `cargo test -p al-analysis`(`ref.lag_*` 契約維持)
- [ ] `cargo test --workspace`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo fmt --all -- --check`
- [ ] 小窓交差の実測結果を記録(閾値採用なら根拠と共に)

## Open Questions / Gaps

- 小窓閾値の有無と値 — S4 の実測で解決

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none(S1→S4 は順次)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- `OverlapCorrPlan` の借用契約は `CorrelationPlan` と同型。呼出側が
  曲線を保持したまま ws を再利用しないことを型で保証する
