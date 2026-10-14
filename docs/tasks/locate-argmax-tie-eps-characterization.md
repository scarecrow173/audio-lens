---
id: "TASK-034Fo9cS08WsbNrLoq6t6a"
type: "task"
status: "done"
title: "argmax 同点検出: 受理系 fixture マージンと scalar↔dispatch 偏差の特性計測"
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
  implements:
    - "docs/plans/implement-locate-argmax-ambiguity.md"
  implemented-by:
    - "docs/impl/ir/locate-argmax-tie-eps-calibration.md"
  depends-on:
    - "docs/plans/implement-locate-argmax-ambiguity.md"
  blocks:
    - "docs/tasks/locate-argmax-ambiguity-detection.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/locate-argmax-tie-eps-calibration.md"
  references: []
  defers: []
  deferred-by: []
---

# argmax 同点検出: 受理系 fixture マージンと scalar↔dispatch 偏差の特性計測

## Work

plan T1(実装ゲート)。**コードを変更する前に**特性計測テストを
追加し、`TIE_REL_EPS`/`TIE_ABS_FLOOR` の実現可能窓を実測で
確定する(design §7 / spec Req 6-7)。計測は2面ある。

**(a) al-locate 側: fixture の per-method マージン分布**

各 fixture について、メソッド毎の raw 相関曲線に対し
`M − max_{i≠argmax}|corr[i]|`(rank-1 の大きさと全サンプル中の
第二順位大きさの差 — argmax 自身のショルダーを含む)を計測・
記録する characterization test を追加する。対象は2群に分かれる:

受理系(margin 下限の計測対象):

- golden 群(`crates/al-locate/tests/golden.rs`):
  `pin-periodic-control`(非周期対照)、`case_repeated_source`、
  `case_loop_material`
- `tests/locate.rs` 受理系:
  `polarity_inverted_source_is_still_located`(:420)、
  `different_rates_locate_without_false_drift`(広帯域チャープ、
  ~:488)、`repeated_source_reports_second_occurrence`
  (ノイズ埋込み、~:440) — 後二者は `spectral_envelope` の
  慢性曖昧リスクの主要観測点

拒否系/混同リスク計測対象(受理系ではないが、同点構造と
マージンを記録して eps の拒否側挙動を確認する):

- golden 群: `pin-periodic`(周期拒否)、
  `pinned_low_coherence_no_inflated_lag`、
  `pinned_silence_no_false_peak`
- `tests/prepared.rs` の fixture(厳密同点 `template`/`−template`
  2出現と周期 sin テンプレート — 周期 fixture での同点構造の
  実形確認も兼ねる)

**再現上の制約**: `band_ncc` の累積曲線のマージン計測に必要な
`bands_for_rate` は `pub(crate)`(`prepared.rs:16`)のため、
計測は crate 内 `#[cfg(test)]` に置くか、`pub` の
`InputCache::prepare().bands()`(内部で `bands_for_rate` を
呼ぶ同一帯域信号)経由で行う。`spectral_envelope` の hop 疎化
曲線は T3 未着地のため `methods.rs:539-547` の**現行逐次式**
(pre-T3 意味論)で再現する — `dsp::corr0` 置換後の曲線とは
浮動小数点丸めが異なり得る

あわせて以下を記録する:

- ショルダーデルタ曝露: 帯域制限ソース
  (`case_long_source_linear_drift` の fc=1kHz/48kHz Biquad
  lowpass ノイズは素朴な推定式 `1−cos(2π·fc/fs)` で ~8.6e-3、
  実効帯域が狭い仮定下で ~1e-4 級、~35Hz 帯域制限は ~1e-6 級
  と推定 — いずれも未検証)が eps 候補レンジと重なるか
- `spectral_envelope` が noise 系入力(ホワイトノイズ・チャープ・
  広帯域)で準同点 |T| 大となる頻度(慢性曖昧リスク)

**(b) al-math 側: scalar↔dispatch 偏差(実運用 FFT サイズ)**

al-math の**既存 `#[cfg(test)]` モジュール内**に characterization
test を追加し、`dispatch(kernel)` と `pulp::Scalar.vectorize(kernel)`
の A/B で偏差上限を実測する:

- `correlation.rs` の `mod tests`: `Correlation { template, signal,
  max_lag, phat }` を phat false/true 両方で。既存テスト
  (`correlation.rs` 絶対 `1e-10`、信号 ≤ 382)は小入力中心のため、
  実運用サイズ(本番 NCC は `(signal.len()+template.len()-1)
  .next_power_of_two()` の RealFFT、golden 規模 ~64k-point)を
  直接カバーする
- `filter.rs` の `mod tests`: `Envelope { samples, fft_size, bins }`
  を実運用 fft_size で

`dispatch` は `pub(crate)`(`lib.rs:28`)で `pulp::Scalar` は crate
内テストから到達可能(`lib.rs` テスト `pulp::Scalar.vectorize`
先例)。al-locate 側からは到達不能 — seam 新設は対象外。

**eps 確定**: 計測結果から `max(dispatch 間偏差) << rel_eps <<
min(マージン)` を満たす窓を示し、`TIE_REL_EPS`/`TIE_ABS_FLOOR`
の値を確定して実装記録(IR ドラフト)へ実測データと共に記録する。
**窓が存在しない、または `spectral_envelope` の慢性曖昧が示された
場合は T2 に進まず**、spec Req 7 のエスカレーション(E-3 拡張
または Option C、spec AC 再検討を含む spec 改訂)へ回す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-locate-argmax-ambiguity.md`(T1)
- `docs/designs/locate-consensus-argmax-ambiguity.md` §7(T1 計測表・
  prepared.rs の意味論シフト分析)
- `docs/specs/locate-consensus-ambiguity-contract.md` Req 6-7
  (dispatch 不変 eps・退化ケース意味論・緊急時方針)
- `docs/adr/locate-argmax-ambiguity-representation.md`(Confirmation:
  eps 意味論の normative 要求)
- 検証仕様: `docs/test-specs/locate-argmax-tie-eps-calibration.md`

## Dependencies

- 先行タスクなし。T2 は本タスクの eps 確定に依存する

## Affected Files

- `crates/al-locate` テスト/計測コード(特性計測テストの追加。
  fixture 駆動のマージン計測を行う `#[cfg(test)]` / tests 追加 —
  本番コードは変更しない)
- `crates/al-math/src/correlation.rs` `mod tests`(実運用サイズの
  scalar↔dispatch 偏差計測を追加)
- `crates/al-math/src/filter.rs` `mod tests`(同上、`Envelope`)
- `docs/impl/ir/<record>.md`(実測値・確定 eps の記録ドラフト)

## Done When

- [ ] al-locate 側の特性計測テストが全対象 fixture(golden 群の
  受理系・拒否系、`tests/locate.rs` 受理系、`tests/prepared.rs`)
  のメソッド毎
  `M − max_{i≠argmax}|corr[i]|` 分布を記録する
- [ ] al-math 側の characterization test が `Correlation`
  (phat false/true)と `Envelope` の実運用 FFT サイズ
  scalar↔dispatch 偏差を実測する
- [ ] `spectral_envelope` の noise 系入力での曖昧頻度と、周期
  fixture の同点構造(分離ピーク vs plateau)が記録される
- [ ] `max(dispatch 偏差) << rel_eps << min(マージン)` の窓が
  示され `TIE_REL_EPS`/`TIE_ABS_FLOOR` が確定・記録される —
  または窓不存在/慢性曖昧が示されて spec Req 7 へ
  エスカレーションされたことが記録される
- [ ] 計測テストは現行挙動を変えず、実装(T2)コミットに先行する

## Verification

- [ ] `mise exec -- cargo test -p al-locate` が green(新規特性計測
  テストを含む)
- [ ] `mise exec -- cargo test -p al-math` が green(新規
  characterization test を含む)
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] IR ドラフトに fixture 毎のマージン・dispatch 偏差上限・
  採用 eps(またはエスカレーション判断)が記録されている(目視)
