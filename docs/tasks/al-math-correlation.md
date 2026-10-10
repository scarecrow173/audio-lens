---
id: "TASK-035eXcVbNmAsDfGhJkLzQ"
type: "task"
status: "done"
title: "al-math: 相関/NCC/bandpass カーネル"
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
    - "docs/plans/simd-dsp-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/al-math-fft.md"
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
    - "docs/test-specs/simd-dsp-foundation.md"
  references:
    - "https://github.com/scarecrow173/audio-lens/issues/40"
    - "https://github.com/scarecrow173/audio-lens/issues/42"
  defers: []
  deferred-by: []
---

# al-math: 相関/NCC/bandpass カーネル

## Work

`al-locate::dsp` の相関・フィルタ系カーネルを `al-math` に移行し
SIMD 化する。

- `ncc`(窓相関)、FFT 相関(fft_ncc 相当)、`gcc_phat` を
  al-math に移設
- `bandpass` の再設計: 承認済み設計に従い spectral-mask の応答を保持し、
  実数 FFT を 1 回行って帯域ごとに IFFT する。IIR への変更は行わない。
  目標は現行比 10× または `sosfiltfilt` 相当(46ms)に近づけること
- `spectral_envelope` 等の補助カーネルも移行対象
- 既存テスト(bandpass_isolates_band 等)の挙動を維持

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`

## Dependencies

- `docs/tasks/al-math-fft.md`

## Affected Files

- `crates/al-math/src/correlation.rs`(新規)
- `crates/al-math/src/filter.rs`(新規)
- `crates/al-math/src/fft.rs`(選択済み ISA を共有する内部入口)
- `crates/al-math/src/lib.rs`
- `crates/al-math/tests/correlation_filter.rs`(新規)
- `crates/al-math/examples/bench_correlation.rs`(新規)
- `crates/al-locate/src/dsp.rs`
- `crates/al-locate/src/methods.rs`
- `docs/tasks/README.md`
- `docs/test-specs/simd-dsp-foundation.md`

## Done When

- [ ] bandpass が現行比 10× 以上高速(または sosfiltfilt 同等)
- [x] 相関・NCC が scalar 結果と一致
- [x] 既存 golden(long_source, skip_and_resume, loop 系)が全て
      パス

## Verification

- [x] `mise exec -- cargo build -p al-math -p al-locate`
- [x] `mise exec -- cargo test -p al-math -p al-locate`
- [x] `mise exec -- cargo fmt -p al-math -p al-locate --check`
- [x] `mise exec -- cargo clippy -p al-math -p al-locate`
- [x] `mise exec -- cargo clippy -p al-math -p al-locate --all-targets -- -D warnings`
- [x] `mise exec -- cargo run --release -p al-locate --example bench_dsp`
- [x] `mise exec -- cargo run --release -p al-math --example bench_correlation`

## 実装・検証記録 (2026-10-10)

`correlation.rs` に符号付き NCC (`fft_ncc` は同じ入口) と GCC-PHAT、
`filter.rs` に spectral mask と spectral envelope を集約した。
公開演算で一度 dispatch し、実数 FFT と積・平方根・正規化を同じ
`Simd` カーネルで実行する。累積エネルギーと包絡の和は標本順に加算する。
`dsp.rs` は再 export と既存 FFT wrapper / deinterleave のみを保持する。
BandNcc は同じ窓区間・FFT 長で素材とキャプチャの forward FFT を
それぞれ帯域間で共有し、素材にない帯域を引き続きスキップする。
入力全体への帯域キャッシュや coarse-to-fine は導入していない。

al-math は unit 13 件・相関/フィルタ integration 5 件・FFT integration 9 件、
al-locate は unit 33 件・golden 16 件・locate integration 9 件が通過した。
scalar 強制経路との比較、直接窓内積、従来の複素スペクトル参照、
空/短入力・無音・端数・DC/Nyquist・帯域の両端・非べき乗長を検証した。
PHAT が NCC の窓エネルギー判定を使わないことも、非ゼロ cross-spectrum と
素材二乗の underflow が共存する入力で失敗→修正→成功を確認した。
FullNcc / BandNcc / GccPhat の極性反転テストでは、絶対値最大の lag を
選び signed score が負のまま保存されることを確認した。
既存 golden は再生成していない。上記 build/test/fmt/clippy はすべて通過し、
`bench_dsp.rs` は編集していない。

### 性能と non-blocking の残課題

最終版で再実行した同一プロセス・CPU・2.4M 標本・48kHz、warm-up 後 3 回平均。
単帯域は `[200, 800)` Hz、5 帯域は BandNcc と同じ帯域分割。
新しい benchmark は元の radix-2 と移行直前の dispatched complex wrapper を
別々の参照として保持する。参照との最大波形絶対誤差は `1.371e-11`。

| 測定対象 | 時間 | 比較 |
| --- | ---: | --- |
| 元の radix-2 単帯域 | 541.8 ms | SIMD 基盤導入前の参照 |
| 移行直前 complex wrapper 単帯域 | 323.8 ms | 今回の移行前の参照 |
| 新しい実数 FFT 単帯域 | 184.0 ms | 元の radix-2 比 2.94×、移行直前比 1.76× |
| 移行直前の独立 5 帯域 | 1633.2 ms | 各帯域で forward / inverse |
| forward 共有 5 帯域 | 497.6 ms | 移行直前比 3.28× |
| NCC 2.4M × 8k | 267.6 ms | 新しい正規化済み公開入口 |
| GCC-PHAT 2.4M × 8k | 273.8 ms | 新しい公開入口 |

指定の `bench_dsp` でも単帯域 170.3 ms、GCC-PHAT 253.2 ms を確認した。
その例の「NCC (fft corr)」489.1 ms は旧 split-complex による分子のみの
手組み計算であり、新しい公開 NCC の測定と混同しない。

**10× / 46ms は未達**で、上の性能チェックを unchecked のまま残す。
coordinator は 2026-10-10 の blocking ask への返信で、設計の応答保持と
golden 維持を優先し、この未達を non-blocking として記録する条件で
task の done を承認した。帯域数分の IFFT が残るため、残余ギャップの
改善は入力準備層の帯域再利用 (`TASK-035g`) に依存する可能性がある。
今回の速度を 10× 達成や `sosfiltfilt` 同等と判定しない。
メモリ peak と TASK-035g の全入力キャッシュ同等性は今回測定・保証していない。

読み取り専用の独立 reviewer 起動は Orca の
`Agent startup blocked: agent-hooks-review-prompt` で失敗した。
失敗 Dispatch の残 terminal は release 済みで、独立レビュー実施済みとは
判定しない。coordinator のマージ前レビューは別途必要。
