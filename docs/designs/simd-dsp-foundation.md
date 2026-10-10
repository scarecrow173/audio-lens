---
id: "DESIGN-034Dlsvf9IuZSuybyJHn8w"
type: "design"
status: "draft"
title: "SIMD 算術クレート基盤と DSP カーネル高速化"
created: "2026-10-10"
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
  derives-from:
    - "docs/specs/source-localization.md"
    - "docs/adr/simd-arithmetic-pulp.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# SIMD 算術クレート基盤と DSP カーネル高速化

## Context

[定位仕様](../specs/source-localization.md)の独立した 4 手法と証拠保存を
保ちながら、[pulp 採用 ADR](../adr/simd-arithmetic-pulp.md)を具体化する。
[承認済み plan](../plans/simd-dsp-foundation.md)の #37/#40/#42/#43 全体を
対象にする。本設計は draft であり、実装入力としての承認は別途レビューで行う。

現行 `crates/al-locate/src/dsp.rs` は radix-2 FFT、FFT 相関、帯域ごとの
FFT/IFFT、正規方程式による least_squares を所有する。`methods.rs` の
BandNcc は呼び出しごとに素材とキャプチャを帯域別にフィルタし、
`stage.rs` は異なる sample-rate を棄却する。`al-analysis` の STFT、
reference、expensive 処理も `al_locate::dsp` に依存している。
算術の所有を `al-math` に移し、入力準備と定位判断の所有を分離する。

## Scope

### In Scope

- `al-math` の pulp dispatch、f64 算術、複素/実数 FFT、NCC/GCC-PHAT、
  bandpass、spectral_envelope、Householder QR と既存利用箇所の移行。
- locate の入力単位帯域キャッシュ、間引き coarse-to-fine 探索、
  sample-rate 正規化と provenance。
- scalar/SIMD 等価性、既存 golden、ベンチによる性能確認。

### Out of Scope

- GPU、nightly/AVX-512、外部 DSP/線形代数ライブラリの採用。
- 合議の閾値・独立手法数・validity 語彙の変更、結果形式の互換層。
- 混合基数 FFT とベクトル超越関数の先行実装。

## Components and Boundaries

| 所有者 | 責務 | 所有しないもの |
| --- | --- | --- |
| `al-math` | 算術型、dispatch、FFT/相関/フィルタ/QR/リサンプル | input id、合議、dataset、provenance の永続化 |
| locate 入力準備層 | レート/スケール統一、帯域キャッシュ、時刻対応 | ISA 検出、算術の複製 |
| locate 手法・stage | 窓選択、候補探索、独立証拠、合議、保存 | pulp 型・レーン幅への依存 |
| mixer / analysis | ミキシング意味、残差、STFT 窓・計測定義 | FFT と least_squares の別実装 |

`al-math` は `pulp::Arch::new()` をプロセス共有で保持し、各演算の
公開入口で `Arch::dispatch(WithSimd)` を行う。演算本体は `Simd` に
ジェネリックな単一ソースとし scalar フォールバックも同じ実装を使う。
内側のループで繰り返し dispatch しない。f64xN/c64xN に相当する
pulp の型・関連型の公開はこのクレートに集約し、通常の呼び出し元は
f64 slice とクレート所有の結果表現を使う。ISA 固定幅を API に埋め込まない。
端数レーンも同じ演算規則で処理する。exp/log は当面スカラ演算とする。

`fft.rs` / `correlation.rs` / `filter.rs` / `linalg.rs` に算術責務を置く。
`al-locate` と `al-analysis` は直接 `al-math` を利用し、既存 dsp の
移行対象実装を削除する。deinterleave 等の入力配置処理や定位固有の
validation は適切な既存境界に残す。算術規則の二重管理はしない。

## Data and Control Flow

### FFT と相関

複素 FFT は当面 2 のべき乗長を受け、長さ一致、forward の負符号、
inverse の正符号と 1/N 正規化、自然順序の出力を現行と揃える。
呼び出し側の next_power_of_two とゼロ埋めを保持する。twiddle を
変換長単位で準備し、独立した butterfly を SIMD 化する。scratch と
係数は処理コンテキストで再利用し、無制限のグローバルキャッシュは作らない。

実数入力は専用 rfft/irfft を用意し、非負周波数 N/2+1 bins を正本に
する。DC/Nyquist と共役対称性の復元は `al-math` に集約する。
STFT の窓・hop・PSD 正規化は analysis に保持する。

NCC の分子は `conj(template_fft) * signal_fft` の逆変換で求める。
循環混入を防ぐ長さは signal_len + template_len - 1 以上とし、
lag は 0..=min(max_lag, signal_len-template_len) を返す。
窓エネルギーと素材ノルムによる正規化を保持し、ゼロエネルギーは 0、
空素材/短い signal は空結果にする。GCC-PHAT は同じ lag 方向で
非ゼロ cross-spectrum を振幅で割り、ゼロ bin はゼロを返す。
低情報窓の validity 判定は手法側に残す。

### Bandpass とキャッシュ

現行のゼロ位相 spectral mask を採用し、IIR への変更は行わない。
これはフィルタ応答と端点挙動を保持するためである。入力の forward
FFT を 1 回行い、各帯域 `[lo_hz, hi_hz)` の mask と IFFT に分岐する。
帯域境界、Nyquist による上限切り詰め、元長への切り戻しを保持する。
素材に存在しない帯域は現行 BandNcc 同様にスキップする。

入力準備層で、入力識別子、チャンネル、正規化レート、信号版、帯域、
フィルタ設定をキーとするフィルタ済み信号を保持する。素材と
キャプチャの同一入力を複数窓・手法実行で再利用し、並列処理では
準備済みの不変データを共有する。ジョブ内の入力処理単位で解放する。
残差探索で信号が変わった場合は別の信号版として扱い、元信号の
キャッシュを誤用しない。窓を切ってからフィルタする経路は、全入力を
フィルタしてから切る経路と端点が異なるため、golden で明示的に検証する。

### Householder QR

現行の `(AᵀA + lambda I)x = Aᵀb` を形成して求解する代わりに、
観測行列 A を Householder QR 分解して Qᵀb と R の後退代入で解く。
ridge は `[A; sqrt(lambda) I]` と `[b; 0]` の拡大系として表す。
有限で非負の lambda、行幅、観測数、rhs 長を演算入口で検証し、
不正入力や解けない非正則系は失敗結果にする。小列数 N≤8 を主対象に
反射・内積を SIMD 化し、負の mixer 係数を保持する。
残差 RMS は正則化項を含めず、元の観測行列と rhs から算出する。

`mixer.rs::fit_mix` は同一素材行列の分解をキャプチャ各チャンネルの
rhs に再利用する。`condition_number` は引き続き ridge 前の正規行列の
診断値として記録する。現在の `dsp::condition_number(&ata)` の意味を
A の条件数へ暗黙に変えず、求解はこの診断行列から独立させる。
特異入力の INFINITY、matrix の向き、fit_quality の定義を保持する。

### al-locate への適用経路

1. decode 後、素材をキャプチャレートへ正規化する。一致時は無変換。
   `al-math` のアンチエイリアス付き polyphase リサンプルを用い、
   元/変換後レート、方式、係数・パラメータ、遅延補償を provenance に残す。
2. 正規化済みのチャンネルと素材区間を入力準備層で作る。元ソース区間は
   秒による対応から元レートの座標へ戻し、キャプチャ座標と混同しない。
   analysis の後段比較にも同じ正規化データと時刻対応を渡す。
3. 長尺探索ではアンチエイリアス付き間引き信号で粗候補を列挙し、
   全レートで近傍を再探索する。粗探索の offset/score を最終証拠にしない。
   間引き率、探索範囲、近傍幅を記録し、周期信号の複数候補を保持する。
   情報不足・曖昧な粗候補では全レート探索を行い、偽受理を防ぐ。
4. NCC、BandNcc、GCC-PHAT、spectral envelope の各証拠を保持し、
   現行の独立手法合議へ渡す。mixer QR と残差探索にも同じ算術を適用する。

## Non-Functional Constraints

- f64 を共通表現にし、scalar/SIMD の差は既存契約の許容誤差内とする。
  同一入力・設定・実行環境では決定的に再実行できることを要求する。
- FMA と加算順序による ISA 間差を bit 一致と仮定しない。固定した
  reduction 順序を採用し、閾値付近の順位・合議・validity まで検証する。
- ベンチ目標は現行比 FFT 3×以上、bandpass 10×以上、NumPy/SciPy
  同等以上。実数 FFT は複素経路の半分以下のコストを確認する。
  これらは測定で判定する目標であり、dispatch 採用だけで保証しない。
- 入力・scratch の寿命を明示し、長尺データの全帯域保持量を測定する。
  必要な帯域のみ準備し、入力処理完了後に解放する。

## Risks and Trade-offs

- FFT 共有でも IFFT が帯域数分必要である。目標未達なら測定箇所を
  特定して同じ応答のカーネルを改善し、応答変更を速度だけで正当化しない。
- 全入力キャッシュと coarse-to-fine は境界・候補欠落のリスクがある。
  長尺、ループ、周期信号、窓境界の fixture で全レート参照と照合する。
- QR は ill-conditioned 入力で従来の係数から変わり得る。真値との誤差と
  残差を評価し、golden を無条件に再生成して差分を消さない。

## Deferred Design Concerns

新たな deferred artifact は設けない。混合基数 FFT、AVX-512、
ベクトル超越関数は今回の scope 外とし、必要性を実測できた時点で
別の設計入力として扱う。

## Verification Notes

検証契約は [test-spec](../test-specs/simd-dsp-foundation.md) に従う。
以下は後続実装の受入検証であり、この文書作成時の実行済み結果ではない。

- [ ] scalar 強制経路と dispatch 経路を同じ入力で比較する。端数レーン、
  FFT 往復・DC/Nyquist、NCC lag 方向、無音、短い入力、帯域境界を含む。
- [ ] QR の既知係数、ridge、特異/高相関チャンネル、condition_number と
  mixer/残差 golden を検証する。
- [ ] 44.1k/48k の定位、元区間との時刻対応、provenance を確認し、
  レート一致の golden は従来どおりであることを確認する。
- [ ] long_source、skip_and_resume、loop 系および analysis/golden を実行し、
  証拠・候補位置・合議の一致と許容誤差を確認する。
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis` と
  `mise run test` を実行する。
- [ ] `mise exec -- cargo run --release -p al-locate --example bench_dsp` で
  旧版と新版を同一入力・CPU・条件で比較し、FFT/bandpass/NCC の時間と
  scratch/キャッシュを含むメモリを記録する。
