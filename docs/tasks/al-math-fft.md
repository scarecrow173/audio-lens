---
id: "TASK-035dQwErTyUiOpAsDfGhJk"
type: "task"
status: "done"
title: "al-math: FFT カーネル(複素/実数)"
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
    - "docs/tasks/al-math-skeleton.md"
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
    - "https://github.com/scarecrow173/audio-lens/issues/42"
  defers: []
  deferred-by: []
---

# al-math: FFT カーネル(複素/実数)

## Work

現行 `al-locate::dsp::fft`(手書き radix-2 複素 FFT、twiddle 逐次
再帰、SIMD なし)を `al-math` に SIMD 化して移行する。

- pulp の `Simd`/`c64xN` でバタフライをベクトル化
- 実数信号用に rfft(2-for-1 または専用実数 FFT)を用意し、
  「実数を複素 FFT に掛ける」現行の 2 倍無駄を解消
- pow2 以外の長さの対応方針を決める(混合基数 or 現行どおり
  次の pow2 パディング — まずは現行互換で良い)
- `al_locate::dsp::fft` の呼び出し元を `al-math` に差し替え
  (dsp.rs の fft は thin wrapper or 直接置き換え)
- `bench_dsp` で NumPy pocketfft(157ms/4M)同等以上を確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/simd-dsp-foundation.md`
- `docs/adr/simd-arithmetic-pulp.md`

## Dependencies

- `docs/tasks/al-math-skeleton.md`

## Affected Files

- `crates/al-math/src/fft.rs`(新規)
- `crates/al-math/src/lib.rs`、`crates/al-math/tests/fft.rs`
- `crates/al-locate/src/dsp.rs`(呼び出しを al-math に差替え)
- `crates/al-locate/Cargo.toml`、`Cargo.lock`(workspace 内依存のみ)
- `crates/al-locate/examples/bench_dsp.rs`
- `crates/al-analysis/src/stft.rs`(共役ピークの丸め差に依存したテストを修正)
- `docs/test-specs/simd-dsp-foundation.md`、`docs/tasks/README.md`

## Done When

- [x] 現行 radix-2 結果と数値一致(許容誤差内)
- [x] `bench_dsp` の FFT が現行比 3× 以上高速(実数入力の rfft: 5.10×)
- [x] 既存テスト(fft 往復、gcc_phat 等)が全てパス

## Verification

- [x] `cargo test -p al-math -p al-locate`(al-analysis も含めて実行)
- [x] `cargo run --release -p al-locate --example bench_dsp`

## 実装と検証結果(2026-10-10)

`Fft` は係数を変換長単位で保持し、radix-2 二段を融合した DIF
butterfly を `Simd` に対する単一ソースで実行する。部分変換と
bit reversal をタイル化し、出力は自然順序、forward は負符号、
inverse は正符号と 1/N 正規化を維持する。公開演算は共有 `dispatch()`
を一度だけ通る。`dsp::fft` は split 配列の配置変換だけを行う thin wrapper。
非 pow2 長は従来どおり caller が next_power_of_two とゼロ埋めを所有する。

`RealFft` は偶数/奇数サンプルを半長の複素入力にまとめ、rfft/irfft の
DC/Nyquist と共役対を集約する。非負周波数 N/2+1 bins が正本であり、
N=1 は 1 bin。係数/scratch は caller 所有のコンテキストで再利用し、
グローバルキャッシュは設けない。4M 時の複素係数は約 64 MiB、実数
コンテキストの係数/scratch は約 96 MiB(入出力バッファを除く)。

- al-math: 3 unit + 9 integration tests が通過。N=1..65536 の旧 radix-2
  比較、独立 DFT、往復、無音/DC/Nyquist、scalar/dispatch、長さ不正、
  決定性、共有 plan の並列利用を検証した。旧 twiddle 再帰の丸め蓄積を
  含む比較許容差は `max(1e-9, N*1e-12)*(1+|expected|_1)`、往復は 1e-10。
- `mise exec -- cargo fmt -p al-math --check`、`cargo build -p al-math`、
  `cargo test -p al-math`、`cargo clippy -p al-math --all-targets -- -D warnings`
  が通過(各 cargo コマンドは `mise exec --` 経由)。
- `mise exec -- cargo test -p al-math -p al-locate -p al-analysis` と
  `mise run test` が通過。locate golden 16 件、analysis golden 19 件は再生成なし。
  STFT の正周波数ピーク判定テストは共役側との丸め差に依存していたため、
  非負周波数を調べるよう修正した。製品の STFT 定義・出力は変更しない。

### 性能の受入と既知の最適化余地

同一 Windows x64 CPU・release・入力(2.4M を 4M にゼロ埋め)で、
`bench_dsp` に旧演算の参照を併設して比較した。各計測は 1 warmup、
旧版/wrapper は 3 回、係数再利用経路は 7 回の平均。ベンチ参照は製品から呼ばない。

| 経路 | 平均時間 | 条件 |
| --- | --- | --- |
| 旧 radix-2 | 270.6 ms | split 入出力を毎回確保 |
| 新 thin split wrapper | 161.8 ms | 係数と c64 バッファを毎回確保 |
| 新複素 FFT | 99.5 ms | 係数/入出力バッファ再利用、実数から c64 へのコピーを含む |
| 新 rfft | 53.1 ms | 係数/scratch/出力再利用、実数パックと spectrum 復元を含む |

実数入力の rfft は同プロセス旧版比 5.10×、NumPy pocketfft 参照値
157 ms 以下。複素カーネルの初期測定では旧版 386.8 ms に対し 111.1 ms
(3.48×)だったが、最終の同プロセス比較では 2.72×。thin wrapper 自体は
最終比較で 1.67×であり、全経路について 3× を達成したとはしない。

coordinator は実数 FFT の「複素経路の半分以下」を、consumer が実信号処理で
実際に払う thin split wrapper 比で判定することを承認した。
53.1 ms は 161.8 ms の 32.8% で受入条件を満たす。同条件の係数再利用
c64 経路比では 53.4%(1.87×)であり、2× 未達は **non-blocking の最適化余地**。
後続 consumer の直接 rfft/irfft 移行ではこの差を区別し、wrapper の
配置変換/再確保コストをカーネルの速度向上と混同しない。
