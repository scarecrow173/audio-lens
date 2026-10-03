---
id: "IMPL-034zk73p9pNrPhGgiCMek4"
type: "impl"
status: "completed"
title: "analysis-expensive-stage"
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
    - "docs/tasks/analysis-expensive-stage.md"
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

# analysis-expensive-stage

## Summary

`al-analysis::expensive` + `ExpensiveAnalysisStage` を実装した。
cheap/reference の FeatureRecord から候補イベントを抽出して
`events` ロールに永続化し、トリガされた時間区間に高分解能の
再解析(カットオフ/トランジェント/ケプストラム残響・反射/
スペクトル詳細)を行い `features/expensive` に書き込む。

## Context

task `docs/tasks/analysis-expensive-stage.md`、plan
`docs/plans/analysis-stages.md`、spec
`docs/specs/expensive-analysis.md` /
`docs/specs/candidate-event-model.md` に従う。

## Implementation

### Changes

- `crates/al-analysis/src/expensive.rs`:
  - `ExpensiveConfig`: `config.analysis.expensive.*` で
    `always_run` / `manual_ranges` / 拡張窓 / 閾値を
    オーバーレイ
  - `plan_regions`: イベントを前後窓で拡張・近接区間を
    マージし、トリガ理由(イベント ID または
    `always_run` / `manual`)を保持した解析区間を計画
  - `analyze_region`: 区間の深掘り特徴を生成 —
    `exp.cutoff_hz`(高域ロールオフ推定)、
    `exp.transient_*`(オンセット密度・最大フラックス)、
    `exp.echo_delay_s` / `exp.echo_strength`(ケプストラム
    由来の残響・コーム検出)、スペクトル/PSD 詳細
  - `parse_feature_records`: JSON ストアから FeatureRecord
    配列を復元する読み出し側
- `crates/al-analysis/src/stage.rs`: `ExpensiveAnalysisStage`。
  `depends_on = [Ingest, CheapAnalysis]`(reference 不在でも
  cheap 候補で起動)。dataset から `features/cheap`・
  `features/reference` の JSON ストアを読み、イベントを
  `events` テーブルに書き込み、区間ごとに
  `features/expensive/<input>__rNNN.json` を出力
- `crates/al-cli/src/main.rs`: `StageKind::ExpensiveAnalysis`
  を配線

### Decisions During Implementation

- ケプストラムは log スペクトルから**平滑包絡を引いて**
  リップルだけ残し、逆 FFT サイズを STFT bin 数に揃えた。
  初期実装は低ケフレンシー成分(包絡)が支配して遅延推定が
  誤り、かつ逆 FFT 長の不一致でケフレンシー→サンプル写像が
  2 倍ずれていた
- イベント抽出で tensor ストアを JSON として読む失敗が
  あったため、候補走査は `StoreKind::Json` のみに限定した
- artifact path は `features/cheap/<input>.json` と拡張子を
  含むため、input_id 導出時に `.json` を除去する
- expensive 出力は `features/expensive` ロールで別パスに
  書き、cheap/reference の行は一切書き換えない
  (measurement-over-verdict、下位観測の不変性)
- イベントがゼロでも `events` テーブル(0 行)を書くため、
  `role_produced` で「実行済み/未選択」を区別できる

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-analysis`:
  - 単体: 区間拡張/マージとトリガ理由、無トリガで区間なし、
    always_run/manual_ranges、既知エコー遅延(120 サンプル
    = 2.5ms @48kHz)をケプストラムで回復
  - 結合(`tests/expensive.rs` 3 件): clipping 候補が
    expensive 区間を起動し cheap 出力を上書きしない、
    抑制時は events 空テーブルのみ、always_run が reference
    なしで全入力を解析
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Risks

- ケプストラムの遅延推定はリップルがスペクトル包絡と
  分離できることに依存する。短区間やフラットなスペクトルでは
  ピークが弱い(strength 0 になり得る)

## Follow-ups

- visualization / export ストリーム(task `viz-*`、
  `export-package`)
