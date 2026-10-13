---
id: "TSPEC-034Fo8X1B0qsgQldLzy9C2"
type: "test-spec"
status: "draft"
title: "argmax 同点 epsilon 較正の特性計測"
created: "2026-10-13"
updated: "2026-10-13"
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
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
  derived-by: []
  verifies:
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# argmax 同点 epsilon 較正の特性計測

## Purpose

`docs/specs/locate-consensus-ambiguity-contract.md` Req 6-7 と
`docs/designs/locate-consensus-argmax-ambiguity.md` §7 が要求する
「実装に先行する特性計測」の検証仕様。`TIE_REL_EPS`/`TIE_ABS_FLOOR`
の実現可能レンジを**実測**で確定し、曖昧性検出が CPU dispatch
(scalar / SIMD)に対して不変となるよう eps を較正する根拠を
残す。理論推定(「~1e-13 級」)ではなく実測を根拠とする —
既存の scalar↔dispatch 等価アサーションは小入力中心
(`correlation.rs` 絶対 `1e-10`・信号 ≤ 382、`filter.rs` は
fft_size ≤ 256、`fft.rs` は n ≤ 2^10・workspace 経路
2^14..2^16)で、`Correlation`/`Envelope` カーネル全体を実運用
FFT サイズで直接アサートするテストは存在しない。

本仕様が保証するのは「測定が行われ、その結果が `TIE_REL_EPS`/
`TIE_ABS_FLOOR` の確定とエスカレーション判定に使われた」という
プロセスの契約であり、測定値そのものではない(値は実装記録
`docs/impl/ir/` へ)。廃止してよいのは、判定式が eps 窓方式を
やめた場合(E-3 / Option C への spec 改訂)、または計測対象の
カーネル構成が変わって測定が陳腐化した場合 — その場合も
superseded/deprecated で記録を残す。

## Feature

実装前特性計測: 受理系 fixture の per-method マージン分布と
al-math カーネルの scalar↔dispatch 偏差実測から、
`max(dispatch 間偏差) << rel_eps << min(意味ある分離)` を満たす
epsilon 窓の存在を実証し、`TIE_REL_EPS`/`TIE_ABS_FLOOR` を確定
する。

## Rules

### Rule A: al-locate 側マージン計測

- Rule: 計測対象は受理系と拒否系の2群 — 受理系(margin 下限の
  計測対象)は golden 群の非周期対照 `pin-periodic-control`・
  `case_repeated_source`・`case_loop_material` と
  `tests/locate.rs` の受理系ケース(
  `polarity_inverted_source_is_still_located` :420、
  `different_rates_locate_without_false_drift` の広帯域チャープ
  ~line 488、`repeated_source_reports_second_occurrence` の
  ノイズ埋込み ~line 440)。拒否系/混同リスク計測対象は golden
  群の `pin-periodic`(periodic rejection)・
  `pinned_low_coherence_no_inflated_lag`・
  `pinned_silence_no_false_peak`、および `tests/prepared.rs` の
  fixture(厳密同点・周期同点構造の実形確認)を含む
- Rule: 計測量はメソッド毎の `M − max_{i≠argmax}|corr[i]|`
  (rank-1 の大きさと全サンプル中の第二順位大きさの差 —
  argmax ピーク自身のショルダー/隣接サンプルを含む)。
  「rank-1 と次位の別位置ピークの分離」ではなく、raw 曲線の
  全サンプル走査に対応するマージン定義を使う
- Rule: `min(意味ある分離)` はこのマージンの fixture 群下限として
  実測する。帯域制限ソースのショルダーデルタ(素朴な推定式
  `1−cos(2π·fc/fs)` — `case_long_source_linear_drift` の
  fc=1kHz/48kHz Biquad lowpass ノイズで ~8.6e-3、実効帯域が
  狭い仮定下で ~1e-4 級、~35Hz 帯域制限で ~1e-6 級。いずれも
  未検証の推定値)が eps 候補レンジ(1e-9..1e-6)と重なり
  得るため、ショルダーデルタ曝露を明示的に確認する
- Rule: `spectral_envelope` の慢性曖昧リスク(32-bin log 振幅
  包絡 NCC がホワイトノイズ・チャープ・広帯域 SFX で殆どの lag
  が準同点)を noise 系入力で実測し、曖昧となる頻度を記録する
- Rule: 周期 fixture では厳密同点・近同点の実形(分離ピーク
  vs plateau)を確認して記録する

### Rule B: al-math 側 dispatch 偏差計測

- Rule: 計測は al-math の既存 `#[cfg(test)]` モジュール内に
  characterization test として追加する — `dispatch` は
  `pub(crate)`(`lib.rs:28`)で `Correlation`/`Envelope` カーネルは
  非公開だが、`pulp::Scalar` は crate 内テストから到達可能
  (`lib.rs` テストの `pulp::Scalar.vectorize` 先例)。dispatch
  override seam の新設は対象外
- Rule: `ncc`/`gcc_phat`(=`Correlation` カーネル phat
  false/true)と `spectral_envelope`(=`Envelope` カーネル)の
  scalar↔dispatch 偏差を**実運用 FFT サイズ**で測る —
  本番 NCC は `(signal.len()+template.len()-1).next_power_of_two()`
  の RealFFT で、golden 規模では ~64k-point。小入力のみの
  既存アサーションの延長ではなく実運用サイズを直接カバーする
- Rule: 計測は `dispatch(kernel)` vs `pulp::Scalar.vectorize(kernel)`
  の A/B で行い、偏差の最大値(絶対・必要に応じ相対)を記録する

### Rule C: eps 確定とエスカレーション

- Rule: `max(dispatch 間偏差) << rel_eps << min(マージン)` を
  満たす eps が存在することを実測で示し、`TIE_REL_EPS`/
  `TIE_ABS_FLOOR` の最終値を実測データと共に実装記録へ記録する
  (候補レンジ: `rel_eps` 相対 1e-9..1e-6、`abs_floor` ~1e-9)
- Rule: 窓が存在しない、または `spectral_envelope` の慢性曖昧が
  実世界の広い入力クラスで示された場合は、eps 再較正で押し
  切らず spec Req 7 のエスカレーション(E-3 拡張または Option
  C、spec AC「受理系 fixture では全メソッドの merged 証拠が
  非曖昧」の再検討を含む spec 改訂)へ進む — 実装(T2)は
  開始しない
- Rule: 判定反転の irreducible 不安定帯 `eps ± dispatch_deviation`
  の帯幅は計測値から確定し、契約の記述として残る

## Examples

- Example(margin 測定): `pin-periodic-control`(非周期埋込み、
  受理必須)で各メソッドの raw 相関曲線から `M` と第二順位
  大きさを計測し、マージンが eps 窓を許すか判定材料にする
- Example(ショルダー): `case_long_source_linear_drift` の帯域
  制限ノイズで argmax 隣接サンプルの |corr| が M に極めて近い
  (ショルダーデルタは未検証推定で ~1e-2..1e-6 級)ことを
  計測で曝露し、eps の実質上限を確定する
- Example(慢性曖昧): `different_rates_locate_without_false_drift`
  のチャープ入力で `spectral_envelope` の |T| が大きくなる
  ことを計測し、曖昧頻度として記録する
- Example(dispatch 偏差): `Correlation { template, signal,
  max_lag, phat }` を `dispatch` と `pulp::Scalar.vectorize` で
  実運用サイズ(例: template ~4k・signal ~60k で ~64k-point
  RealFFT)に走らせ、最大 |a−b| を記録する
- Example(厳密同点構造): `pin-periodic` 相当の周期入力で全 lag
  が厳密 1.0 となる形状(分離ピーク集合)を観測する

## Guarantees

- `TIE_REL_EPS`/`TIE_ABS_FLOOR` が理論推定ではなく実測に基づき
  較正されること — dispatch 間の判定反転が
  `eps ± dispatch_deviation` の不安定帯を除いて起こり得ない
  ことが計測値で裏付けられる
- 受理系 fixture で曖昧性の false positive が出ないことを、
  実装投入**前**にマージン分布で裏付けること
- `spectral_envelope` の慢性曖昧や帯域制限ショルダーのような
  実入力リスクが、実装後の golden 破壊ではなく計測段階で
  表面化すること
- 測定の生データ(fixture 毎のマージン・偏差上限・採用 eps)が
  実装記録に残り、将来のカーネル変更時に再較正の根拠として
  使えること

## Non-goals

- 実運用サイズ以外のカーネル等価性の再検証(既存の小入力
  アサーションで担保済み)
- `vec` プリミティブ(dot/sum_sq 等)単体の偏差計測 —
  `vec.rs` テストが `1e-12·max(1,‖a‖,‖b‖)` 相対で ~1M まで
  カバー済み。対象は `Correlation`/`Envelope` の複合カーネル
- al-math への dispatch override seam 新設(`pub(crate)` 境界を
  越える到達手段の追加はスコープ外)
- golden 判定意味・受理結果の検証 — 契約の行動保証は
  `locate-argmax-ambiguity.md` の責務
- eps の一意な「正解」の証明 — 保証するのは窓の存在と確定値の
  記録であり、窓内の選択は実装判断

## Risk

- この計測を省くと eps が根拠なしに選ばれ、(a) 受理系 fixture
  で慢性 false positive(ショルダー/包絡準同点)、(b) dispatch
  間で判定が反転する不安定帯が未記述のまま残る、(c) eps 窓
  不存在の場合に実装後に手戻りする、のいずれかを招く
- 測定値が記録されないと、将来のカーネル変更(#42 FFT 変更等)
  で eps の再較正根拠を再構築する術を失う
