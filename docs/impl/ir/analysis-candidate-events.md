---
id: "IMPL-034zk72Z7IcW9x10eo6U09"
type: "impl"
status: "completed"
title: "analysis-candidate-events"
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
    - "docs/tasks/analysis-candidate-events.md"
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

# analysis-candidate-events

## Summary

`al-analysis::events` に候補イベントモデル(spec
`candidate-event-model.md`)を実装した。cheap/reference の
FeatureRecord 群から閾値越えを抽出し、安定 ID・証拠リンク・
validity/confidence 伝播を持つ CandidateEvent を生成して
dataset の `events` ロール(列指向テーブル)に永続化する。

## Context

task `docs/tasks/analysis-candidate-events.md`、plan
`docs/plans/analysis-stages.md`、spec
`docs/specs/candidate-event-model.md`、ADR
`docs/adr/measurement-over-verdict.md` に従う。

## Implementation

### Changes

- `crates/al-analysis/src/events.rs`:
  - `CandidateEvent`: content-derived の安定 ID
    (`EVT-<alg>-<input>-<ms>-<hash>`。kind・入力 ID・時刻・
    チャンネル範囲・ソース artifact・特徴から導出し、
    ステレオ per-channel 行や異なる source/capture ペア由来の
    行が衝突しないようにする)、kind、時間範囲、strength、
    validity、confidence、`{"artifact","rows"}` 証拠リンク
  - `EventConfig`: 全閾値を `config.analysis.events.*` で
    オーバーレイ可能(`from_job`)
  - `extract_candidates`: spec の閾値ルール表(clip /
    discontinuity / silence-dropout / low-coherence /
    high-residual / lag-drift 等)を FeatureRecord に適用。
    `time_range` を持たないセグメントレベル行は入力の
    セグメント長までをイベント範囲とする(`segment_scope`
    を params に記録)。invalid 行もイベントを起こし、
    その validity をイベントへ伝播する(弱い観測を強い主張に
    格上げしない)。confidence は min 伝播
  - `events_schema` / `events_to_rows`: events テーブルへの
    平坦化(行 = 1 イベント、証拠は JSON 文字列列)
- `crates/al-dataset/src/types.rs`: `Value::Scalar(f64)` の
  `Deserialize` を手動実装し、JSON `null` → `NaN` を復元
  (invalid スカラーが null に serialize されるため)

### Decisions During Implementation

- `above` ルールに一律 `abs()` を掛けていた初期実装は負の
  閾値(noise floor -40dB 等)で常時ヒットするバグだった。
  比較は生値に戻し、双方向量(lag drift)だけ `map: |v| v.abs()`
  で変換するよう Rule に `map` フィールドを追加した
- `noise.floor_dbfs` のイベント kind は当初
  `SilenceDropoutCandidate` としていたが、高ノイズフロアは
  ドロップアウトではないため `ElevatedNoiseFloor` kind を
  追加して割り当て直した
- イベント永続化の責務は expensive ステージ側に置き、
  expensive 未選択時は events ロール自体が無い
  (`not_produced` と区別可能)という plan の分割を維持した
- NaN の JSON 表現は「スカラーを null に落とし、読み出し時に
  NaN に戻す」で往復一貫。互換レイヤーは設けない

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-analysis`: events 単体テスト 4 件
  (閾値越えでリンク付きイベント、閾値未満で無発火、
  invalid 行の validity 伝播、ID 決定性、テーブル平坦化)
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Risks

- ヒューリスティック閾値のため、ホワイトノイズのような
  広帯域信号は noise-floor 候補を立てる(測定上の注目点と
  しては正当だが、「クリーンな信号 = イベントゼロ」ではない)

## Follow-ups

- 抽出されたイベントを消費する `ExpensiveAnalysisStage`
  (task `analysis-expensive-stage`)
