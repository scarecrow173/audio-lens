---
id: "TSPEC-034Y19pTojHQhGnOiVrawz"
type: "test-spec"
status: "approved"
title: "Issue #127 RF#628 consumer oracle: file-based oracle テスト仕様"
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
        path: "docs/test-specs/issue-127-rf628-consumer-oracle.md"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
    - "docs/designs/issue-127-rf628-consumer-oracle.md"
  derived-by: []
  verifies:
    - "docs/specs/issue-127-rf628-consumer-oracle.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Issue #127 RF#628 consumer oracle: file-based oracle テスト仕様

## Purpose

`ruviform-audio-test`（RF#628）の deterministic PCM fixture/oracle
を使い、AudioLens file-based 解析の f64 fidelity・長尺非回帰・
gap/Validity を **AL 独自の独立 oracle** で判定する統合テストの
保証内容を固定する。ruviform 側 self-test の結果は証拠に数えない。

## Feature

RF#628 fixture 駆動の file-based 解析 oracle:
`PcmFixture`/`PcmScenario` → f32→f64 exact widen → `pcm_f64le`
artifact → `Manifest`/`ArtifactRegistry`/`StageContext` → 実
`CheapAnalysisStage`/`LocateStage`/`ReferenceAnalysisStage` →
feature record / dataset tensor が、独立導出の期待値と一致する。

## Rules

- **R1 fixture 検証**: `fixture.block()`/`fill()` の出力は
  `oracle::pcm_reference_block` の独立再実装と `pcm_assert_eq` で
  一致する（Exact: Silence/Constant/Impulse/Ramp/Square/Noise、
  AbsEpsilon: Sine/Sweep）。この照合は入力データの妥当性確認で
  あり AL の Pass 判定ではない。
- **R2 f64 fidelity（AL-1）**: 計測 feature（`level.dc_offset`・
  `level.sample_peak`・`level.rms_dbfs`・`level.crest_factor`・
  `level.true_peak_est`・`dig.*`・`spec.*`・`noise.*`）が、
  oracle 参照ブロックを f64 widen した統計量および解析的参照
  （sine 周波数・constant level・seed noise 統計）と許容差内で
  一致する。f32→f64 widen は exact であり、期待値は oracle 側
  f32 を f64 へ widen して導出する。
- **R3 ステレオ分離**: multichannel fixture は channel 別の
  record（`channels` selector）を持ち、channel 独立 stream
  （Noise の per-channel hash、Sine の channel/16 位相）を
  oracle 参照と照合できる。
- **R4 長尺非回帰（AL-2）**: 複数 `PcmSegment` の大フレーム
  `PcmScenario`（48kHz、採用規模は後述）を dataset 形状
  artifact として流し、`LocateStage`→`CheapAnalysisStage`→
  `ReferenceAnalysisStage` の参照再計算が既存 golden 許容帯域
  （`ref.gain_ls`/`ref.rms_ratio` abs_rel(0.03,0.1)・lag≈0・
  similarity 高位）で非回帰であり、長大 trajectory/tensor が
  破綻しない。
- **R5 gap validity（AL-3）**: 非連続 `PcmScenario` の gap frame
  を `f64::NAN` として decoded_pcm に展開したとき、
  segment `[frame_start, frame_start+frame_len)` 範囲と level
  window/hop（既定 960/480 samples @48kHz）から独立導出した
  期待 validity map と AL 出力が一致する:
  level trajectory tensor の NaN 窓位置、summary record の
  `support_count`（有限窓数）、`dc_offset` の JSON null、
  `sample_peak`/`true_peak_est` の有限性（valid 区間の最大）、
  `rms_dbfs` の -240 floor、全 record の `params.ruviform`
  stamp（rev=解決 SHA・precision=f64）。
- **R6 NaN 伝播の pin**: `dc`/`rms_all` は NaN 伝播、
  `peak`/`true_peak` は `f64::max` の NaN 無視で有限、
  `robust_summary` は非有限を除外して `support`/`valid_fraction`
  を計算、spectral 系は NaN STFT フレームを有限フレーム統計で
  吸収（全滅時 record 非出力）という既存意味論を観測値として
  pin する。`argmax` の `total_cmp` NaN 規則による
  `noise.tonal_peak_hz` 縮退も観測して pin する。
- **R7 正直な verdict**: 実 OS GUI/実 pixel/実 device/実 plugin/
  macOS/Linux/cross-consumer/CLI/Export は NotRun・
  NotApplicable・Deferred として記録し、headless/semantic の
  Pass を実機 Pass に繰り上げない。

## Examples

- **AL-1 Constant**: `PcmPattern::Constant{value:0.25}`・48kHz・
  mono・~1s。`pcm_assert_eq(Exact)` 後、`level.dc_offset`≈0.25
  （f64 統計量、abs 1e-6 — f32 累積なら ~1e-4 で検出可能）、
  `level.sample_peak`=0.25、`level.rms_dbfs`=20log10(0.25)、
  `dig.quant_step_est` が NotApplicable（単一水準）。
- **AL-1 Sine**: `Sine{freq_hz:1500,amplitude:0.5}`（2048 window
  で bin 64 に整列）。`noise.tonal_peak_hz`≈1500±bin_hz、
  `level.rms_dbfs` が oracle 参照 rms の 20log10 と一致。
- **AL-1 Noise**: `Noise{seed:42,amplitude:0.8}`。oracle 参照
  ブロックの dc/rms を f64 で計算し `dc_offset`/`rms_dbfs` と
  照合（統計量は有限標本の近似のため緩めの許容差）。
- **AL-1 Square**: `Square{amplitude:0.5,period_frames:48}`
  （1000Hz fundamental）。rms=amplitude、dc=0、
  `noise.tonal_peak_hz`≈1000Hz。
- **AL-1 Stereo Noise**: `channels:2` の `Noise{seed,amplitude}`。
  ch0/ch1 で独立 oracle ブロック（`pcm_reference_block(.., ch)`）
  の統計量と各 channel record を照合。
- **AL-2 long-run**: contiguous `PcmScenario`（例: Noise 10s +
  Sine 10s + Square 10s、48kHz mono）。source=そのまま、
  capture=0.5 gain embed。Locate→Reference で
  `ref.gain_ls`≈0.5・`ref.rms_ratio`≈0.5・lag≈0・
  similarity>0.9。cheap feature record と trajectory tensor
  長が duration/hop と整合。**採用規模**: 48kHz・30 秒
  （Noise→Sine→Square 各 10s、計 1,440,000 frames、
  trajectory 2999 windows）。golden fixture(~0.3s)の ~2000 倍。
  debug 実測 60s→150s / 45s→130s / 30s→~85s で支配コストは
  locate 相互相関（ほぼ線形）。tensor chunk 境界と trajectory
  長の非回帰検出力は 30s で十分と判断（実測根拠:
  `docs/impl/exp/issue-127-rf628-long-scenario-scale.jsonl`）。
- **AL-3 gap**: `PcmScenario` = `[0, 2s) Sine` + gap 0.5s +
  `[2.5s, 4.5s) Constant{0.4}`（非連続）。gap frame を NaN で
  decoded_pcm に展開。期待 map: level window/hop=960/480 で
  gap と重なる窓 = NaN、内側窓 = 有限。trajectory tensor の
  NaN index 集合・`support_count`=有限窓数・`dc_offset`=null・
  `sample_peak`≈max(0.5,0.4)=0.5・`rms_dbfs`=-240・
  `params.ruviform.rev`=lockfile SHA。

## Guarantees

- fixture 出力と独立 oracle の一致（入力データの決定性）。
- AL の f64 計測が f32 相当に縮退しないこと（dc_offset の
  1e-6 級許容差が f32 累積誤差 ~1e-4 を排除）。
- 長尺 scenario で locate/reference の参照再計算が golden
  許容帯域を維持し、tensor/trajectory が長大入力で破綻しない。
- gap 混在入力で NaN・Validity・Provenance が segment 幾何由来の
  期待 map と一致し、欠測が silent に握り潰されない
  （dc_offset=null・trajectory NaN・support_count 減少として
  観測可能）。
- `params.ruviform` stamp が全 record に解決 SHA と
  `precision:"f64"` を記録する。

## Non-goals

- AL-4 deterministic clock drift: live clock consumer がなく
  **Deferred**（`clock::VirtualClock*` は文書参照のみ）。
- AL-5 device-pipeline legs（`backend::*`/`pipeline::*`/`midi::*`/
  `ScriptedBackend`）: device boundary 非消費のため
  **NotApplicable**（`Unsupported` ではない）。
- PTS/タイムスタンプ・ingest/decode 側の gap 検出、CLI/GUI/
  Export、3OS CI・cross-consumer・実機検証。
- `noise.tonal_peak_hz` 等 spectral 系の gap 縮退の「修正」
  （観測 pin のみ。改善は別 Issue）。
- ruviform 側実装の変更、Issue #127 の Close。

## Risk

- 長尺テストが debug CI で遅い → 規模を実測で決定し test-spec
  に根拠を記録。超過時は縮小。
- Sine/Sweep の oracle-generator 差が feature 許容差を超える
  → AbsEpsilon/解析的参照（周波数・統計量）で吸収、超過時は
  fixture 側の既知差分として記録。
- gap 縮退の観測値が環境依存 → 決定的 fixture のため再現性あり。
  `argmax` NaN 規則の帰結はプラットフォーム非依存。
