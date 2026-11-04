---
id: "DESIGN-034Dlsvf9IuZSuybyJHn8w"
type: "design"
status: "approved"
title: "SIMD 算術クレート基盤と DSP カーネル高速化"
created: "2026-10-10"
updated: "2026-10-12"
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
  verified-by:
    - "docs/test-specs/simd-dsp-foundation.md"
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
(追認: 本設計の approved 後に採択された ADR
[al-math-api-fft-split-re-im](../adr/al-math-api-fft-split-re-im.md)
により、「このクレートへの公開集約」はさらに「バックエンド型を
公開しない」へ引き締められた。将来のバックエンド移行を `al-math`
内部の変更に閉じ込めるための字義ドリフトであり、上記の記述は
当初方針の記録として残す)
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
フィルタ応答と端点挙動の保持を要件とするが、全入力キャッシュでの
同等性は P2 / F5 の未決事項として後述する。入力の forward
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
   下記の正規化 PCM 成果物として保存する。元/変換後レート、方式、
   係数・パラメータ、遅延補償を成果物の `params` に残す。
2. 正規化済みのチャンネルと素材区間を入力準備層で作る。元ソース区間は
   下記の座標対応から元レートの座標へ戻し、キャプチャ座標と混同しない。
   alignment は使用した PCM の artifact id と区間を保存し、analysis は
   その参照から同じ正規化データと時刻対応を再取得する。
3. 長尺探索ではアンチエイリアス付き間引き信号で粗候補を列挙し、
   全レートで近傍を再探索する。粗探索は候補提案に限定し、粗探索の
   offset/score や近傍だけの統計を最終証拠にしない。
   間引き率、探索範囲、近傍幅を記録し、周期信号の複数候補を保持する。
   最終証拠は、間引き前の全レートと、その手法・素材窓に対して元々
   指定された全探索域で計算する。背景中央値は全 lag の絶対値、
   競合ピークと順位は全域の局所最大、半値幅は全域内で左右の半値到達点
   または探索域端までを対象とし、`methods.rs::peaks_from_correlation`
   の背景下限・端点・同値順位・候補数の規則を保持する。
   近傍外の候補欠落、中央値の変化、半値幅の切り詰めがないことを保証
   できない場合は、全レート・全探索域の相関曲線を計算する全探索へ
   fallback する。現設計には近傍のみでこの保証を行う手段がないため、
   最終証拠生成は全探索を必須とする。情報不足・曖昧な粗候補も同じ
   fallback を適用し、`consensus.rs::evaluate` の背景比による受理契約を保つ。
4. NCC、BandNcc、GCC-PHAT、spectral envelope の各証拠を保持し、
   現行の独立手法合議へ渡す。mixer QR と残差探索にも同じ算術を適用する。

### 正規化 PCM の永続化と再取得

以下は後続実装で追加する契約であり、現行 `decode.rs::decode_input` は
元の `decoded_pcm` と `ingest_metadata` を復元する入口として残す。

- locate 入力準備層は、新設 role `normalized_pcm` の `ArtifactRecord` を
  `StageContext` の manifest に登録する。保存先は job 相対の
  `locate/normalized/<artifact_id>.f64le` とし、`fsutil::publish_bytes`
  (relaxed 等級、`Durability::BestEffort`)で保存する。
  形式はフレーム順・チャンネル順のインターリーブ済み little-endian f64。
  `decode_input` のスケール正規化後の値を保存し、再読込で再度 scale を
  適用しない。リサンプルによる振幅の overshoot はクリップしない。
  `params` に `input_id`、`sample_rate`、`channels`、フレーム数、
  保存形式と振幅スケールを記録し、チャンネル順序を元 PCM と揃える。
- 再利用の識別キーは、元 `decoded_pcm` と `ingest_metadata` の artifact id、
  目標 sample-rate、変換 algorithm id/version、変換設定の組とする。
  変換設定は元レート、整数レート比、polyphase の係数またはその完全な
  再生成パラメータ、アンチエイリアス設定、端点拡張、出力長の丸め規則、
  遅延と補償量を含む。設定の正本は成果物の `params` とし、キーはその
  正本から構成する。input id だけでレートや設定の異なる成果物を共用しない。
- `ArtifactRecord.inputs` は元 `DecodedInput.artifact_ids` を保持し、
  `produced_by` は `StageKind::Locate` とする。レート一致でも identity
  変換として同じ保存形式・参照契約を用い、元成果物への lineage を残す。
- 座標対応の正本も `params` に保存する。元レートを Rs、正規化レートを
  Rn、補償後に残る時刻原点差を t0 秒とし、正規化フレーム n の元座標を
  `Rs * (n / Rn + t0)` と定義する。t0 は有理数で保存し、通常は遅延補償
  により 0 とする。区間は `[start, end)`、元区間への変換は開始を floor、
  終了を ceil とし、元フレーム数内に制限する。元/正規化フレーム数と
  レートを保存し、丸めで失われる情報を秒だけの記録に依存させない。
- `crates/al-locate/src/result.rs` の `LocateResult` に、使用 PCM を指す
  source/capture の artifact id と正規化 source 区間を後続実装で追加する。
  `source_range` は元素材座標、`capture_range` はキャプチャ座標とし、
  正規化 source 区間は参照先 PCM の座標とする。capture も identity
  正規化 PCM を明示参照する。`locate_result` の `ArtifactRecord.inputs`
  に両 PCM を含め、`alignment_id` から使用データと変換を追跡できるようにする。
- 共有の成果物読込境界を `crates/al-locate/src/decode.rs` に追加し、
  artifact id で manifest を解決して `checked_job_path` 経由で復元する。
  role、レート、チャンネル数、バイト長とフレーム数、区間の整合を検証する。
  `crates/al-analysis/src/stage.rs` の reference と expensive の alignment
  比較はこの入口と正規化区間を使う。元 source を input id で引く現在の
  `decode_input` / decoded map の経路は alignment 比較では置き換える。
  cheap 等の元入力観測は現行経路を使う。参照欠落・不整合はステージ
  エラーとし、元 PCM への暗黙の fallback や別設定での再リサンプルはしない。
  後段成果物の lineage に alignment と両 PCM の artifact id を含める。

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

P2 / F5 は未決事項として残す。BandNcc の全入力キャッシュは、窓単位の
フィルタと比べて FFT 長と周波数 grid、および端点応答を変え得る。
後続のキャッシュ実装前に、対象区間、ゼロ埋めを含む FFT 長、帯域境界の
bin 選択、端点拡張・切り戻し規則を固定し、これらをキャッシュキーに
含める設計を確定する。golden に対する波形・定位証拠の許容差も未決であり、
閾値近傍を含む検証で不一致の場合は従来の区間単位フィルタへ戻す。
許容差と同等性が確定するまで全入力キャッシュの応答保持は保証しない。

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
- [ ] F4: locate と別プロセスの reference/expensive が alignment の PCM
  参照から同じサンプルを復元し、元区間・正規化区間・lineage が一致する。
  identity、丸め境界、複数目標レート、参照欠落・不整合も検証する。
- [ ] F3: 全探索参照と背景中央値・競合ピーク・半値幅・候補順位・合議を
  比較する。近傍外の強いピーク、広い半値幅、周期信号、閾値近傍を含め、
  契約保証がない近傍探索が全レート・全探索域へ fallback することを確認する。
- [ ] long_source、skip_and_resume、loop 系および analysis/golden を実行し、
  証拠・候補位置・合議の一致と許容誤差を確認する。
- [ ] `mise exec -- cargo test -p al-math -p al-locate -p al-analysis` と
  `mise run test` を実行する。
- [ ] `mise exec -- cargo run --release -p al-locate --example bench_dsp` で
  旧版と新版を同一入力・CPU・条件で比較し、FFT/bandpass/NCC の時間と
  scratch/キャッシュを含むメモリを記録する。
