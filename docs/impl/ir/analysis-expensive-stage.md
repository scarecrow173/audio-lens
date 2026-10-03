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
    `always_run` / `manual_ranges` / `event_window_s` /
    `stft_window` / `stft_hop` をオーバーレイ
  - `plan_regions`: イベントを前後窓で拡張・近接区間を
    マージし、トリガ理由(意味理由 + `candidate:<event id>`、
    または `always_run` / `manual`)を保持した解析区間を計画
  - `analyze_region`: 区間の深掘り特徴を生成 —
    `exp.cutoff_hz` / `exp.dictionary_cutoff_delta_khz`
    (高域ロールオフと辞書期待値との差)、`exp.narrowband_*`
    (狭帯域ピーク/ノッチ)、`exp.echo_delay_s` /
    `exp.cepstrum_prominence`(ケプストラム由来の残響・
    コーム検出)、`exp.src_image_score` /
    `exp.src_candidate_rate`(SRC ミラー相関)、
    `exp.polarity_asymmetry` / `exp.thd_ratio`(非線形)、
    `exp.dynamic_range_db` / `exp.attack_*` / `exp.release_*`
    (ダイナミクス)、`exp.repeat_block_score` /
    `exp.gap_count`(反復・ドロップアウト)、
    `exp.spectral_hole_ratio` / `exp.codec_block_*`
    (コーデック痕跡)、`exp.tonal_peak_count` /
    `exp.hum_harmonic_count`(ノイズ分解)、
    `exp.freq_lr_coherence`(ch 間)、
    `exp.region_support`(解析不能時の InsufficientSupport 明示)
  - `parse_feature_records`: JSON ストアから FeatureRecord
    配列を復元。不正な `records` フィールドはエラーで
    失敗する(暗黙の空リストはデータ破損を「候補なし」に
    変換してしまうため)
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
  2 倍ずれていた。さらにレビューで「フラット/縮退ケプストラム
  でも argmax は必ず存在する」ため、`exp.echo_delay_s` は
  prominence > 0.5 のときだけ Valid とし、それ以外は
  `InsufficientSupport` で NaN を報告するよう改めた
- レビュー修正(確認済み P1 相当):
  - `time_range` を持たない行が全イベントを t=0 の点に
    潰していた → 入力のセグメント長をイベント範囲とし、
    `segment_scope` を params に記録
  - イベント ID が channels / artifact を含まず per-channel
    行やペア間で衝突し得た → ハッシュ入力に追加
  - `parse_feature_records` と dictionary 読み込みの失敗を
    `.ok()` で黙殺していた → 明示エラー化
  - `events` テーブルの provenance(parents = 走査した
    artifact、effective config を params に)とジョブ
    manifest への ArtifactRecord 登録を追加
  - STFT 由来のレコードに `resolution` / `channels` を
    付与。評価不能な測定(SRC 候補なし・両 ch にエネルギー
    なし等)は `InsufficientSupport` で報告
  - 低サンプルレートでの `chunks(0)` パニックを `max(1)`
    で防止、`sample_rate`/`channels` = 0 の decoded_pcm は
    ローダーで明示拒否
  - イベントの validity/confidence が `plan_regions` で捨てられ、
    非 Valid な前提由来の区間でも Valid 出力が保存されていた
    → `TriggeredRegion` が最弱の upstream validity/confidence を
    保持し(`manual`/`always_run` は独立トリガとして
    Valid/High)、`analyze_region` が全レコードへ
    `propagate_validity`/`propagate_confidence` で伝播する
- イベント抽出で tensor ストアを JSON として読む失敗が
  あったため、候補走査は `StoreKind::Json` のみに限定した
- artifact path は `features/cheap/<input>.json` と拡張子を
  含むため、input_id 導出時に `.json` を除去する
- expensive 出力は `features/expensive` ロールで別パスに
  書き、cheap/reference の行は一切書き換えない
  (measurement-over-verdict、下位観測の不変性)
- イベントがゼロでも `events` テーブル(0 行)を書くため、
  `role_produced` で「実行済み/未選択」を区別できる
- `intermediate` に保存される cheap の粗 STFT を再利用せず
  区間内で細かい窓の STFT を再計算するのは、expensive が
  高分解能を要求するため(粗分解能の中間生成物からは
  逆算できない)

## Related Experiments

### Adopted

- なし

### Rejected

- なし

## Validation

- `cargo test -p al-analysis`:
  - 単体: 区間拡張/マージとトリガ理由、無トリガで区間なし、
    always_run/manual_ranges、既知エコー遅延(120 サンプル
    = 2.5ms @48kHz)をケプストラムで回復、セグメント全域
    イベント、チャンネル/artifact でイベント ID 分離
  - 結合(`tests/expensive.rs` 4 件): clipping 候補が
    expensive 区間を起動し cheap 出力を上書きしない、
    抑制時は events 空テーブルのみ、always_run が reference
    なしで全入力を解析、manual_ranges が候補なしでも
    `manual` トリガで区間を生成
- `cargo test --workspace` 全 pass
- `cargo clippy --workspace --all-targets -- -D warnings` clean

## Risks

- ケプストラムの遅延推定はリップルがスペクトル包絡と
  分離できることに依存する。短区間やフラットなスペクトルでは
  ピークが弱い(strength 0 になり得る)

## Follow-ups

- visualization / export ストリーム(task `viz-*`、
  `export-package`)
