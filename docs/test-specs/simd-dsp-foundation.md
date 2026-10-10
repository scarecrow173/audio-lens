---
id: "TSPEC-035iTyUiOpAsDfGhJkLzX"
type: "test-spec"
status: "proposed"
title: "SIMD 算術クレート基盤と DSP カーネル高速化"
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
  implements: []
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
  verifies:
    - "docs/plans/simd-dsp-foundation.md"
    - "docs/adr/simd-arithmetic-pulp.md"
    - "docs/specs/source-localization.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# SIMD 算術クレート基盤と DSP カーネル高速化

## Verifies

- `docs/plans/simd-dsp-foundation.md`
- `docs/adr/simd-arithmetic-pulp.md`
- `docs/specs/source-localization.md`

## Coverage

### al-math 基盤(TASK-035c)

- [ ] `al-math` が pulp のみを外部 SIMD 依存として集約する
- [ ] `Arch::dispatch` 経由で実行時 ISA 振り分けが動く
- [ ] scalar フォールバックと SIMD 経路が同一結果を返す

### FFT(TASK-035d)

- [x] 複素 FFT が現行 radix-2 と数値一致(許容誤差内)
- [x] 実数 FFT が複素経路の半分以下のコストで動く
- [x] `bench_dsp` で NumPy pocketfft 同等以上(~3× 現行比)
- [x] `fft` 往復・`gcc_phat` 等の既存テストがパス

2026-10-10: [FFT task の検証結果](../tasks/al-math-fft.md)を参照。
旧 radix-2/独立 DFT/scalar/dispatch の比較と既存 golden を再生成なしで検証。
実数入力 rfft は同プロセス旧版 270.6 ms に対し 53.1 ms(5.10×)、
NumPy pocketfft 参照値 157 ms 以下。

「複素経路の半分以下」は coordinator の承認により、実信号 consumer の
thin split wrapper(161.8 ms)比で判定する。rfft はその 32.8% で受入条件を満たす。
同条件の係数再利用 c64 経路(99.5 ms)比では 53.4%(1.87×)であり、
2× 未達は **non-blocking の最適化余地**として記録する。複素カーネルと
配置変換/確保を含む wrapper の測定を区別し、全経路 3× とは判定しない。

### 相関/bandpass(TASK-035e)

- [ ] NCC・GCC-PHAT が scalar 結果と一致
- [ ] bandpass が現行比 10× 以上高速(sosfiltfilt 46ms 参照)
- [ ] `bandpass_isolates_band` 等の既存契約を維持

### QR(TASK-035f)

- [ ] ill-conditioned 入力で係数が正規方程式版より安定
- [ ] `condition_number` が引き続き記録される
- [ ] mixer/残差減算の golden がパス

### coarse-to-fine + キャッシュ(TASK-035g)

- [ ] BandNcc が入力単位のフィルタ済み信号を再利用する
- [ ] 長尺ソースで粗探索が間引き信号で動作する

### リサンプル正規化(TASK-035h)

- [ ] 44.1k×48k ペアがリジェクトされず locate される
- [ ] provenance に元/変換後レートとリサンプラ種別が記録される
- [ ] レート一致の既存 golden が変わらない

## Verdict Criteria

- 全項目がパス: `implemented`
- golden 差分が許容誤差を超える: `failed`
