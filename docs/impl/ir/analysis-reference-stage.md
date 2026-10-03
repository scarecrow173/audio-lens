---
id: "IMPL-034zjJ08PnsbhXiLHgMoMf"
type: "impl"
status: "completed"
title: "analysis-reference-stage"
created: "2026-10-03"
updated: "2026-10-03"
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
    - "docs/tasks/analysis-reference-stage.md"
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

# analysis-reference-stage

## Summary

`al-analysis::reference` に受理済み alignment ペアのリファレンス
比較解析(ラグ/ゲイン/帯域/伝達関数/チャンネル応答/段階残差
R0-R3)と `ReferenceAnalysisStage` を実装し、CLI に配線した。

## Context

task `docs/tasks/analysis-reference-stage.md`、plan
`docs/plans/analysis-stages.md`、spec
`docs/specs/reference-analysis.md` /
`docs/specs/analysis-dataset.md` に従う。

## Implementation

### Changes

- `crates/al-analysis/src/reference.rs`:
  `analyze_pair` が 1 ペア分の全特徴を生成。ラグ系
  (`ref.lag_*`: 局所両方向相関による τ(t)、median/MAD/
  percentile、ドリフト、step 候補、ピーク幅、time-scale)、
  ゲイン系(`ref.gain_ls` / `ref.gain_400hz` / `ref.rms_ratio` /
  `ref.src_level_gain_corr`)、帯域系(1/3 オクターブ M2、
  `ref.band_correlation_min`、band coherence、PSD 比)、
  伝達関数系(Welch CSD → 複素 `H(f)`/`|H|`/位相 unwrap/
  群遅延、coherence/support)、ch 系(`ref.mixer_fit`、
  `ref.channel_response`、極性、低域 L/R coherence、M/S、
  crosstalk)、類似度系(`ref.similarity_fullband`、
  `ref.phat_peak`、`ref.envelope_similarity`)、
  段階残差 R0-R3(waveform WAV + STFT + PSD + band energy +
  RMS 比 + source 相関)
- `crates/al-analysis/src/stage.rs`: `ReferenceAnalysisStage`。
  `locate_result` artifact → `result.json` を読み、受理ペアは
  `source_range`/`capture_range` で切り出して解析、未受理ペアは
  `AmbiguousAlignment` の invalid 行を書いて続行。
  `depends_on = [Ingest, Locate, CheapAnalysis]`。出力は
  `features/reference/<src>__<cap>`(JSON)・`intermediate/`・
  `residual/`(WAV + tensor)
- `crates/al-analysis/src/reference.rs` 内テスト 6 件 +
  `crates/al-analysis/tests/reference.rs` 結合テスト 2 件
- `crates/al-cli/src/main.rs`: `StageKind::ReferenceAnalysis` を
  stub から `ReferenceAnalysisStage` に置き換え、USAGE 更新

### Decisions During Implementation

- 段階残差は**カスケード方式**: 各ステージのモデルは前段残差に
  対して fit し、前段残差から差し引く。R1(スカラ LS)・R2
  (ridge mix)は LS 性質で単調減少が保証され、R3(FIR)は
  LS でないためエネルギー実測ゲートを入れた。当初の「全モデルを
  cap から独立に差し引く」設計では R3 が R2 を悪化させ得るため
  変更した
- 残差ラグ推定は `ncc`(片方向)ではなく `corr_at` による
  `[-max_lag, max_lag]` の両方向相関にした。同長セグメントでは
  `ncc` のラグ探索範囲が 0 に縮退するため
- 未受理 alignment は `analyze_pair` に空セグメント +
  `AmbiguousAlignment` を渡し、invalid 行(support 0)を出す
  統一パスとした。ステージは abort しない
- R3 の FIR は Welch H(`E[conj(X)·Y]/E[|X|²]`)の irfft で、
  畳み込み長は 4096 tap に制限

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-analysis`: 23 件全 pass。
  既知ゲイン 0.5/400Hz ゲインの回復、既知残差遅延 7 サンプルの
  群遅延・ラグ回復、R0-R3 エネルギー単調減少(単体・結合)、
  同一信号で残差 ≈0、未受理 alignment の invalid 行、
  短入力でパニックしないこと
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo fmt --all` 適用済み

## Risks

- STFT/R3 畳み込みは素朴実装(O(n·tap))のため長尺ペアで遅い。
  perf が問題になった段階でブロック畳み込みへ改善する
- `ref.group_delay_median_s` は median 統計のため位相が乱れる
  低 coherence 帯域ではノイズに左右される。coherence 重み付けは
  Follow-up

## Follow-ups

- `ExpensiveAnalysisStage` / candidate event 生成
  (task `analysis-expensive-stage` / `analysis-candidate-events`)
