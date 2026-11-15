---
id: "DESIGN-034Y19nInNC42C6kYwz7tm"
type: "design"
status: "approved"
title: "Issue #127 RF#628 consumer oracle: file-based f64/長尺/gap oracle 設計"
created: "2026-11-15"
updated: "2026-11-15"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/127"
    - "https://github.com/scarecrow173/ruviform/issues/628"
  changes:
    added:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/al-analysis/Cargo.toml"
      - type: "file"
        path: "crates/al-analysis/tests/rf628_oracle.rs"
      - type: "file"
        path: "docs/designs/issue-127-rf628-consumer-oracle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "ui/vendor/ruviform"
  implements:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ruviform-ac10a2f7-rev-watch.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 RF#628 consumer oracle: file-based f64/長尺/gap oracle 設計

## Context

AudioLens の file-based 解析は `decoded_pcm` artifact（f64 LE
インターリーブ）を `decode_input`（`al-locate`）経由で
`DecodedInput` に復元し、`CheapAnalysisStage`/`LocateStage`/
`ReferenceAnalysisStage`/`ExpensiveAnalysisStage` が
`Manifest`/`ArtifactRegistry`/`StageContext` 経由で消費する。
既存テスト（`tests/golden.rs`・`reference.rs`・`expensive.rs`・
`avsync.rs`）は `al-testkit` の決定的 f64 信号でこの経路を駆動する。

`scarecrow173/ruviform#628` の `ruviform-audio-test` は
`publish = false` の test-only crate で、deterministic PCM
fixture（`PcmFixture`/`PcmPattern`/`PcmScenario`/`PcmSegment`）と
独立 oracle（`oracle::pcm_reference_block`/`PcmCompare`/
`pcm_assert_eq`/`pcm_max_abs_diff`/`pcm_bits_eq`）を提供する。
fixture は絶対 frame index の純粋関数で、サンプルは f32 固定
（libm 非依存の整数・f64 算術。Sine/Sweep のみ oracle 側が libm
`f64::sin` を使い `PcmCompare::AbsEpsilon` で比較）。
`PcmScenario` は非重複・ソート済み `PcmSegment` 列で、segment 間の
frame 欠落が gap（dropped/missing frames）を表す。

pin は `ac10a2f7` → `abb9f372d3ee95ee824b3c4f7dc68e8083ce2774`
（ruviform main HEAD、`crates/ruviform-audio-test/src/pipeline/`
存在確認済み）へ lockfile-primary で bump する。lockfile の
ruviform package は 20 → 26 に増加（`ruviform-audio-test` 本体と
その推移的依存 `ruviform-audio`・`ruviform-graph`・
`ruviform-plugin-core`・`ruviform-plugin-host`・
`ruviform-audio-plugin-bridge`・`ruviform-io-symphonia`・
`symphonia` 系・`shared_memory` 等が新規解決）。

## Scope

### In Scope

- `crates/al-analysis` dev-dependency への `ruviform-audio-test`
  （+ `ruviform-audio-core`、`SampleRate` 構築用）追加と pin bump。
- `crates/al-analysis/tests/rf628_oracle.rs` 新設（3 脚を 1 ファイルの
  test module 群で実装。helper はファイル内 private）。
- fixture → f64 widen → `pcm_f64le` → `Manifest`/`ArtifactRegistry` →
  `StageContext` → 実 stage の file-based 経路。
- `oracle::pcm_reference_block` による fixture 検証と参照統計量の
  期待値導出、segment 範囲由来の validity map の独立導出。
- rev-watch 文書セットと本 slice の文書チェーン。

### Out of Scope

- 製品コード変更（`al-analysis` src の最小適応を除き原則なし）、
  AL-4 clock legs（Deferred）、AL-5 device-pipeline legs
  （NotApplicable）、ingest/decode 側 PTS/gap、CLI/GUI/Export、
  3OS CI・cross-consumer・実機検証、Issue #127 の Close。

## Components and Boundaries

- **Fixture 生成側（ruviform、被験ではない）**:
  `PcmFixture::new(SampleRate, channels, PcmPattern)` →
  `fixture.block(start_frame, frames)` / `fixture.fill(...)` /
  `fixture.sample_at(channel, frame)` で planar `[channel][frame]`
  f32 を得る。`PcmScenario::new(segments)` → `segments()`/
  `is_contiguous()`/`segment_at(frame)`。
- **独立 oracle（ruviform、期待値の出典）**:
  `oracle::pcm_reference_block(&fixture, start, frames, channel)`
  が spec の独立再実装で f32 ブロックを返す。`pcm_assert_eq` で
  fixture 出力（`block.data` の channel slice）と照合する
  （Exact: Silence/Constant/Impulse/Ramp/Square/Noise、
  AbsEpsilon: Sine/Sweep）。この照合は **fixture 入力データの
  妥当性検証**であり、AL の Pass 判定には使わない。
- **file artifact 化（al-testkit、既存）**:
  `pcm_f64le(&[f64])` で decoded_pcm bytes を生成。
  `f32 → f64` widen は `f64::from(f32)` で exact。
  planara `[ch][frame]` → インターリーブ f64 への変換はテスト側で
  行う（ch0..chn の frame-major 並べ替え）。
- **解析駆動（al-job/al-analysis/al-locate、被験）**:
  `Manifest::new` + `input_artifact` + `register_artifact`
  （`role: "decoded_pcm"`, `params: {sample_rate, channels}`）→
  `ArtifactRegistry::new(job_dir)` → `StageContext` →
  `CheapAnalysisStage::new().run(&ctx)` 等。出力は
  `analysis/features/cheap/<input_id>.json`（`{"records":[...]}`）
  と dataset tensor（`DatasetReader` で `read_tensor`）。
- **期待値導出（テスト側、独立）**:
  (a) `pcm_reference_block` 出力を `f64::from` で widen し、
  平均・二乗平均平方根・最大絶対値・20log10 を f64 で計算。
  (b) 解析的参照: `Constant{value}` → dc=|value|・peak=|value|・
  rms_dbfs=20log10(|value|)、`Sine{freq_hz}` →
  `noise.tonal_peak_hz ≈ freq_hz`（STFT bin 分解能内）、
  `Noise{seed,amplitude}` → oracle 統計量、
  `PcmScenario` segment 幾何 → level window/hop ごとの
  有限/NaN 期待 map。
- **NaN gap 表現**: `PcmScenario` の非連続区間（`is_contiguous()
  == false`、`segment_at(frame) == None` の frame）を `f64::NAN`
  として decoded_pcm に書く。`decode_input` は decoded_pcm の
  non-finite を拒否しない（拒否は `read_normalized_record` の
  locate 正規化経路のみ）ため、NaN は `channel_samples` へ届く。

## Data and Control Flow

1. **dep/pin**: root `Cargo.toml` `[workspace.dependencies]` に
   `ruviform-audio-test`（`rev` なし git dep）追加、
   `crates/al-analysis/Cargo.toml` `[dev-dependencies]` に
   `ruviform-audio-test.workspace = true` と
   `ruviform-audio-core.workspace = true`（`SampleRate` 用）追加。
   `node tools/update-ruviform-rev.mjs abb9f372...` →
   `mise run sync-ui-deps` → `cargo fetch --locked` →
   `check-ruviform-lock`/`check-ruviform-rev`。
2. **AL-1**: pattern ごとに fixture → block → oracle 照合
   （`pcm_assert_eq`）→ f64 widen → interleave → `pcm_f64le` →
   単一入力 manifest → `CheapAnalysisStage` → feature JSON を読み、
   oracle 統計量・解析的参照と照合。ステレオ `Noise` で
   channel 別 selector の record を照合。
3. **AL-2**: contiguous `PcmScenario`（複数 pattern segment、
   48kHz・数十秒〜数分）→ 全 frame を `sample_at`/`block` で
   f64 widen レンダリング → source artifact、gain 済みを bed に
   embed した capture artifact → `LocateStage` →
   `CheapAnalysisStage` → `ReferenceAnalysisStage` →
   `ref.gain_ls`/`ref.rms_ratio`/lag/similarity が golden 許容帯域、
   cheap record・tensor 形状が長尺に対し健全であることを確認。
4. **AL-3**: 非連続 `PcmScenario` → gap frame を NaN とした
   flat f64 配列 → decoded_pcm → `CheapAnalysisStage` →
   `level.trajectory` tensor の NaN 窓位置・`support_count`・
   `dc_offset` null・`rms_dbfs` floor・peak 系有限性・
   `params.ruviform` stamp を、segment 幾何から独立導出した
   期待 validity map と照合。

## NaN/gap の観測意味論（実装確認済み、テストで pin する）

`measure_level`/`measure_digital`/`measure_spectral` の NaN 伝播:

- `peak`/`true_peak`: `f64::max` 畳み込みは NaN を無視 → 有限
  （valid サンプルのみの最大）。
- `dc`・`rms_all`: `sum`/`sum_sq` は NaN を伝播 → `dc_offset` は
  JSON null、`rms_dbfs` は `db(NaN)` = `20*log10(1e-12)` = -240.0
  （floor）、`crest_factor` は `NaN > 0.0` 不成立で 0.0。
- `level.trajectory` tensor: windowed rms は NaN を含む窓で NaN。
  `robust_summary` は非有限を除外し `support`=有限窓数、
  `valid_fraction`=有限/全窓 を計算（record の `support_count`
  に反映）。
- `level.momentary/short_term_dbfs`: `max(db(rms))` で NaN 窓は
  -240 floor に沈み有限最大が残る。`support` は全窓数。
- `level.integrated_dbfs`: momentary ブロック和が NaN 伝播で
  -240.0。`level.loudness_range_db`: 短窓 db 系列の p95-p05 で
  -240 floor が混入し得る（観測して pin）。
- spectral 系: NaN を含む STFT フレームは全 bin NaN →
  `mean_row` が全 NaN 化 → `argmax`（`total_cmp` で NaN 最大）
  により `noise.tonal_peak_hz` が縮退値を取り得る。`spec.*`
  trajectory は有限フレームのみの robust_summary で record 化
  （全滅時は record 自体が出ない）。`spec.slope` 等は NaN→null。
- `dig.*`: NaN サンプルは比較をすり抜けるが分母（`len`）には
  含まれる。`dig.quant_step_est` は NaN 差分を除外して有限値。
- `params.ruviform` stamp: 全 record の `params.ruviform.rev` が
  `abb9f372...`、`precision` が `"f64"`。

これらは既存 AL の意味論であり本 slice では変更しない。テストは
「観測された契約」を pin し、解釈が必要な縮退（tonal peak 等）は
コメントと PR 発見事項に記録する。

## AL-2 長尺規模の選定

`PcmScenario` の総フレーム数は debug プロファイルの CI 実行時間で
決める。目安: source ≈ 90 秒（48kHz・4.32M frame・約 35MB f64
mono）+ capture（gain 0.5 embed）。`LocateStage` の FFT 相関と
`ReferenceAnalysisStage` の band 別 LS が支配的。実測でテスト
時間が許容を超える場合は 30〜60 秒へ縮小し、根拠を test-spec に
記録する。「数分規模」の要件に対し、frame 数・tensor チャンク・
軌跡長が golden fixture（〜0.3 秒）より 2〜3 桁大きいことをもって
長尺非回帰の検出力とする。

## Risks and Trade-offs

- `ruviform-audio-test` の推移的依存（`symphonia`・`shared_memory`・
  `ruviform-plugin-*` 等）が lockfile を +6 package 増やす →
  dev-dep 専用で製品配布に影響なし。`check-ruviform-lock` で単一
  SHA を確認。
- Sine/Sweep は oracle（libm sin）と generator（poly近似 + 固定
  小数位相）でサンプルが厳密一致しない → feature レベルでは
  AbsEpsilon 相当の許容差（tonal 周波数は bin 分解能、rms/peak は
  ~1e-4 相対）で吸収。fixture 検証は `PcmCompare::AbsEpsilon`。
- 長尺テストの debug 実行時間 → 実測で規模を決定し test-spec に記録。
- NaN gap で spectral 系が縮退値を返す既存意味論 → pin して記録、
  修正は別 Issue の判断。
- rollback: `git checkout -- Cargo.lock` + manifest から dep 削除 +
  `mise run sync-ui-deps` で `ac10a2f7` へ復帰（生成物のみ、
  製品コード差分なし）。

## Deferred Design Concerns

- AL-4 clock legs | live clock consumer 不在 | clock consumer 採用
  slice 時 | clock 契約の見落とし | test-spec に Deferred 記録
- AL-5 device-pipeline legs | device boundary 非消費 |
  device-pipeline consumer 採用 slice 時 | device 境界回帰の
  見落とし | test-spec に NotApplicable 記録
- spectral 系の gap 縮退（`argmax` NaN 規則）| 既存意味論・本
  slice 対象外 | spectral gap 対応 Issue 起票時 | gap 混在入力の
  誤値 | PR 発見事項・test-spec に記録

## Verification Notes

- [ ] `mise run check-ruviform-lock`/`check-ruviform-rev`:
  単一 SHA `abb9f372`・vendor REV 一致
- [ ] `mise run fmt`/`check al-analysis`/`lint al-analysis`/
  `test al-analysis`: 対象 revision で Pass
- [ ] `mise run check-cli-headless`: headless 境界 Pass
- [ ] `mise exec -- prek run --all-files`: Pass
- [ ] `cargo fetch --locked`: Pass（補助証拠）
