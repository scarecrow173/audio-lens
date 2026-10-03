---
id: "IMPL-034zjJIH7CNhYcfC6oKhy9"
type: "impl"
status: "completed"
title: "analysis-cheap-stage"
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
    - "docs/tasks/analysis-cheap-stage.md"
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

# analysis-cheap-stage

## Summary

`al-analysis::cheap` に常時オン解析の全特徴ファミリと
`CheapAnalysisStage` を実装し、CLI に配線した。E2E で
46 特徴 ID / 83 レコードを生成することを確認済み。

## Context

task `docs/tasks/analysis-cheap-stage.md`、plan
`docs/plans/analysis-stages.md`、spec
`docs/specs/cheap-analysis-features.md` に従う。

## Implementation

### Changes

- `crates/al-analysis/src/cheap.rs`: `analyze_input` が 1 入力の
  全特徴を生成。`level.*`(RMS/integrated/trajectory/crest)、
  `spec.*`(centroid/bandwidth/entropy/flatness/flux/rolloff/
  slope/nb peaks/notches)、`stereo.*`(coherence/ILD/IPD/
  MS ratio/pair corr)、`trans.*`(onset/self-similarity)、
  `mod.*`(AM depth/freq/lag drift)、`noise.*`(floor/hum/
  tonal)、`dig.*`(click/clip/discontinuity/dup block/quant/
  stuck/zero run)の 7 ファミリ。trajectory 系は tensor 化して
  `intermediate/<input>/` に、スカラは `features/cheap/<input>` の
  JSON レコードとして `ReferenceResult` 相当の `CheapResult` で返す
- `crates/al-analysis/src/stage.rs`: `CheapAnalysisStage`。
  decoded PCM を `al_locate::decode::decode_input` で読み、
  `open_analysis_dataset` で検証済み dataset に追記。
  `depends_on = [Ingest]`。source/capture 片方のみのジョブも
  正常終了(`Ok(vec![])` ではなく実データを書く)
- `crates/al-analysis/src/records.rs`: `RecordBuilder` /
  `robust_summary` / `propagate_validity` /
  `propagate_confidence` の共通レコード補助
- `crates/al-cli/src/main.rs`: `StageKind::CheapAnalysis` を
  `CheapAnalysisStage` に配線
- 単体テスト 4 件 + 合成 E2E(`al run` で job 完走、
  46 特徴 ID / 83 レコード確認)

### Decisions During Implementation

- trajectory(flux 等)は tensor store 参照
  (`Value::Tensor { array, region }`)にし、JSON 内に波形を
  展開しない。spectral flux の先頭フレームは前任フレームがない
  ため 0 を入れて全 trajectory のフレーム数を揃えた(tensor の
  行形状を揃えるため)
- validity は計算不能な値を握り潰さず
  `Silence`/`InsufficientSupport` 等で明示。silence 入力でも
  レコードは出す
- stereo 特徴は 2ch 入力のみ。mono では該当レコードを出さない
  (invalid 行も出さず、特徴自体を省略)

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-analysis` 全 pass(既知正弦波のレベル、
  silence/clip 検出、stereo ペア特徴、短入力の
  `InsufficientSupport`)
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- 合成 WAV での `al run` E2E 完走確認
  (`tmp-e2e/jobs/...` に dataset + 特徴出力)

## Risks

- `dig.quant_step_est` / `dig.duplicate_block_candidates` は
  ヒューリスティック推定で、量子化ビット深度が高い素材では
  検出感度が落ちる

## Follow-ups

- `docs/tasks/analysis-reference-stage.md`(済)
- `docs/tasks/analysis-candidate-events.md` /
  `docs/tasks/analysis-expensive-stage.md`
