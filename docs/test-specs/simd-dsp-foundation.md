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

- [x] NCC・GCC-PHAT が scalar 結果と一致
- [ ] bandpass が現行比 10× 以上高速(sosfiltfilt 46ms 参照)
- [x] `bandpass_isolates_band` 等の既存契約を維持

2026-10-10: [相関 task の検証記録](../tasks/al-math-correlation.md)を参照。
scalar/dispatch、直接窓内積、旧複素スペクトル、帯域境界・端点と極性反転の
signed score を検証し、既存 golden 16 件を再生成なしで維持した。
同一プロセスの 2.4M 単帯域は 184.0 ms、移行直前 complex wrapper
323.8 ms 比 1.76×、元の radix-2 541.8 ms 比 2.94×。
5 帯域共有は 497.6 ms、移行直前の独立 5 帯域 1633.2 ms 比 3.28×。
10× / 46ms は未達であり、性能チェックは unchecked のまま保持する。
coordinator 承認によりこの未達は **non-blocking の残課題**とし、応答保持を
優先して TASK-035e を done とする。帯域数分の IFFT と後続 TASK-035g の
入力準備層での再利用を含む改善余地を記録し、性能達成とは判定しない。

### QR(TASK-035f)

- [x] ill-conditioned 入力で係数が正規方程式版より安定
- [x] `condition_number` が引き続き記録される
- [x] mixer/残差減算の golden がパス

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
