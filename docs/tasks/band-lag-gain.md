---
id: "TASK-036dBnMqWeRtYuIoPlKj"
type: "task"
status: "done"
title: "reference: 帯域別ラグ補償ゲイン推定(#38)"
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
    - "docs/plans/media-format-analysis-extensions.md"
  implemented-by: []
  depends-on:
    - "docs/plans/media-format-analysis-extensions.md"
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
    - "docs/test-specs/media-format-analysis-extensions.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/38"
  defers: []
  deferred-by: []
---

# reference: 帯域別ラグ補償ゲイン推定(#38)

## Work

現行 `ref.band_gain_m2` は PSD 比ベースで、元ツールチェーン資料の
注意点「非コヒーレントな PSD 比は周波数特性判断に使うな」に
抵触するリスクがある。本来の手法 — 「帯域ごとの最良ラグ探索
(±40ms)→ LS ゲイン → 相関上位半分の中央値」— を実装する。

- 1/3oct(または等価)帯域ごとに ±40ms 窓で最良ラグを探索
  (現行 `lag_max=512` = ±10.7ms @48k より広い)
- 帯域ラグ補償後に LS ゲインを算出(least_squares は
  QR 化後の al-math 経由でも可)
- 帯域相関上位半分の中央値を代表値として出力
- coherence / support を validity メタデータとして付与
- 既存 `band_gain_m2` との関係(置き換え or 併記)を決める —
  pre-release のため置き換えてよい

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/media-format-analysis-extensions.md`
- `docs/specs/reference-analysis.md`
- issue #38

## Dependencies

なし(#40 のフィルタキャッシュと統合すると速いが必須ではない)

## Affected Files

- `crates/al-analysis/src/reference.rs` / `band_gain.rs` / `stage.rs`
- `crates/al-analysis/tests/reference.rs` / `golden.rs`
- `crates/al-locate/src/prepared.rs`

## Done When

- [x] 帯域ごとの最良ラグ+LS ゲイン+中央値が観測として出力される
- [x] coherence-based validity メタデータが付く
- [x] 非コヒーレント帯域で PSD 比由来の過大評価が出ない

## Verification

- [x] `cargo test -p al-analysis`

## Implementation

- `ref.band_gain_m2` は帯域ラグ補償 signed LS の正本とする。
  `ref.band_gain_psd_ratio` は低信頼の二次観測として別 ID に保持する。
- 1/3oct 帯域を `InputCache::prepare_with_bands` でチャンネル単位に準備し、
  `bandpass_many` の共有 FFT と `ncc` (al-math の re-export) を再利用する。
  キャッシュは aligned pair の処理終了時に解放する。workspace 依存の追加はない。
- `config.analysis.reference.band_window_s=0.1`、`band_hop_s=0.05`、
  `band_max_lag_s=0.04` が既定値。有限の `(0,1]` 秒を受け、範囲外は既定値を使う。
  実 rate から標本数へ変換し、設定・rate・帯域・閾値を record と tensor provenance に保存する。
- source の固定長窓と capture の前後探索領域を使い、全候補を同じ標本数で評価する。
  正の lag は capture の遅れ。絶対相関の同点 (FFT 丸め許容 1e-12) は
  小さい絶対 lag、負の lag の順で決める。
- 通常/反転極性を分け、各群の相関上位 ceil(N/2) の signed gain の中央値を保存する。
  偶数個の中央値は中央2値の平均。空の群は unavailable とし、別群で補完しない。
  400Hz ゲインと相対ゲインは同じ推定器の同じ支持された極性群だけを参照する。
- 絶対相関 0.5 未満または Welch magnitude-squared coherence 0.5 未満は
  low validity / low confidence と下限的解釈を付ける。無音、符号不整合、
  探索端 peak、境界不足を記録する。PSD 比の大きさで支持を昇格させない。
- window×band の lag/gain/correlation/support/start_samples をチャンネル別 tensor に保存する。
  帯域ごとに端点除外幅が違うため start_samples が窓の正確な位置を示し、
  欠損セルは NaN (support は 0) とする。alignment/channel/freq_range を record に保持する。

## Regression Evidence

- 8kHz/48kHz、2帯域の 25ms/15ms 遅延、0.5/1.4 gain、2チャンネルの極性反転を検証。
- 上位半分、時間同点、偶数中央値、無音、短入力、探索端 peak、無相関の大振幅雑音を検証。
- Dataset に保存された帯域 tensor と設定/provenance、LS/PSD 観測の分離を integration test で検証。
- 既存 golden 19ケースの期待範囲を維持。帯域検証の3ケースは 100ms 窓と
  ±40ms 探索に足りる長さに延長し、空の極性群と signed gain の新契約を反映。

## Non-blocking

- spectral mask の端点応答は有限支持ではないため、低域端の1周期分を
  filter guard として除外する。より長い guard の感度分析は今回の対象外。
- pair 間での reference 専用キャッシュ共有は行わず、pair 内の全窓・帯域で再利用する。
  locate の既存入力キャッシュと帯域キー・フィルタ実装は共通である。

## Verification Results (2026-10-10)

- `mise exec -- cargo build -p al-locate -p al-analysis -p al-math`: passed。
- `mise exec -- cargo test -p al-analysis -p al-locate -p al-math`: 181 passed / 0 failed。
  al-analysis 89 (golden 19含む)、al-locate 65 (golden 16含む)、al-math 27。
- `mise exec -- cargo fmt --all -- --check`: passed。`bench_dsp.rs` は未変更。
- `mise exec -- cargo clippy -p al-locate -p al-analysis -p al-math --all-targets -- -D warnings`: passed。
- `git diff --check`: passed。
