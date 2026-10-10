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

- [ ] 複素 FFT が現行 radix-2 と数値一致(許容誤差内)
- [ ] 実数 FFT が複素経路の半分以下のコストで動く
- [ ] `bench_dsp` で NumPy pocketfft 同等以上(~3× 現行比)
- [ ] `fft` 往復・`gcc_phat` 等の既存テストがパス

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

- [x] 44.1k×48k ペアがリジェクトされず locate される
- [x] provenance に元/変換後レートとリサンプラ種別が記録される
- [x] レート一致の既存 golden が変わらない

- [x] アンチエイリアス・image 抑制・群遅延補償・overshoot 非クリップ
- [x] 正規化 PCM の identity、複数目標レート、元座標への丸め、lineage
- [x] 別プロセスの reference/expensive が同じ保存 PCM を再取得
- [x] PCM 参照欠落、role/レート/チャンネル/長さ/区間不整合でエラー

実行済みの対象検証と既存 fmt 差分は
[タスクの検証記録](../tasks/locate-resample-normalize.md#検証記録-2026-10-10)
を参照。基盤全体の他タスクに関する未検証項目はこの結果に含めない。

## Verdict Criteria

- 全項目がパス: `implemented`
- golden 差分が許容誤差を超える: `failed`
