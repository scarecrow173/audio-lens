---
id: "TSPEC-034Fo8Wnl0C3P71Q2uPUWP"
type: "test-spec"
status: "draft"
title: "locate 合議 argmax 曖昧性契約の検証"
created: "2026-10-13"
updated: "2026-10-14"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# locate 合議 argmax 曖昧性契約の検証

## Purpose

`docs/specs/locate-consensus-ambiguity-contract.md`
(SPEC-034FlHW8GmjrKvTJlf0UWw)の Acceptance Criteria と
`docs/designs/locate-consensus-argmax-ambiguity.md`
(DESIGN-034Fn25qh0mE8cz5ymYKfD)の実装分岐を、検証可能な
保証に分解する検証仕様。保証対象は「argmax が統計的に一意でない
証拠を合議が構造的に検出・除外・分類し、決定的カーネル
(`dsp::corr0`)下でも周期入力の偽受理が再現されない」こと —
現行の拒否が逐次 NCC の浮動小数点丸め誤差による偶然の散乱に
依存している状態を契約で置き換える。

判定式・分岐順序・分類優先順位・reject_reason 文言の権威は spec
Req 1-11 と design §1-§6 にあり、本文と乖離した場合はそちらを
正とする。`TIE_REL_EPS`/`TIE_ABS_FLOOR` の具体値は
`docs/test-specs/locate-argmax-tie-eps-calibration.md` が記録する
T1 実測で確定し、本仕様は値そのものではなく判定の構造
(同点集合の定義・計数単位・分岐順序)を pin する。

廃止してよいのは、曖昧性契約そのものが spec 改訂で撤廃された
場合、または `Validity` 表現が B-2(新 variant)へ移行して本仕様の
記述が陳腐化した場合のみ。その場合は書き換えではなく
superseded/deprecated で歴史を残す。

## Feature

locate の4メソッド証拠と合議が、raw 相関曲線上の argmax 同点集合
(`T = { i : |corr[i]| >= M − tie_window }`、`|T| >= 2` で曖昧)を
検出し、曖昧証拠を候補保持のまま `AmbiguousAlignment` にマーク・
投票から除外し、曖昧起因の不受理を `AmbiguousAlignment` に分類
する — メソッド証拠・合議・チャンネルペア統合・ウィンドウ
アンカーの4層で同一契約が効くこと。

## Rules

### Rule A: 同点集合判定(証拠レベル)

- Rule: 判定は `evidence_from_corr` 内で **raw 相関曲線**に対して
  行う。`peaks_from_correlation` 内部では検出せず、同関数および
  `propose_coarse`/`refine_coarse` 経路は不変
- Rule: `|T|` はサンプル(index)数として数える。連続する等値
  plateau を run として1位置に潰さない — plateau はそれ自体が
  位置の非一意性であり、単一 plateau のみの曲線は曖昧となる
- Rule: 比較は `|corr[i]|` の大きさで行い、全体最大値
  (rank-1)にアンカーする — 候補間 pairwise ではない。符号反転
  ペア(+1.0 / −1.0)は大きさで同点となり曖昧と判定される
- Rule: 曖昧性判定は **Valid となるはずの証拠にのみ**適用する —
  no-peak・非有限/非正・p2b 不足の各 reject 分岐の後に評価し、
  弱さで棄却済みの曲線は `LowCorrelation` のままとし曖昧へ
  格上げしない
- Rule: 曖昧証拠は `candidates` を `peaks_from_correlation` の出力
  のまま保持し、`reject_reason` に固定マーカー句
  `"indistinguishable"` を含む(推奨文言 `"top correlation peaks
  indistinguishable within tolerance (K positions)"`、K は
  サンプル数)
- Rule: argmax の決定的タイブレーク(最小 lag / index 順)を受理・
  confidence の根拠にしない。stable sort 順序は candidates の
  表示順としてのみ許容される

### Rule B: 合議の投票除外と分類優先順位

- Rule: 投票フィルタの順序は (1) validity 除外
  (`ComputationFailed` **および** `AmbiguousAlignment`)→
  (2) メソッド毎 dedup(先着1件)→ (3) `candidates.first()` 存在 →
  (4) `min_score`/`min_peak_to_background` 閾値、の通り
- Rule: 投票が空かつ `AmbiguousAlignment` 証拠が ≥1 あれば、結果
  validity は `AmbiguousAlignment` で `reject_reason` は
  `"all voting methods produced ambiguous (tied) peaks"`
  ("method(s) agree" を含まない)。この判定は既存の
  `any_valid → LowCorrelation`/`InsufficientSupport` 分岐より**前**
  に行う。副次フィールドは既存の空投票分岐と同値
  (`accepted = false`、`agreeing_methods = 0`、
  `position_spread = 0`、`position = None`、`Confidence::Low`)
- Rule: 投票が空かつ曖昧証拠なし → 既存分岐を不変で維持。
  投票非空かつ `agreeing < min_agreeing_methods` → 既存の
  `AmbiguousAlignment` + "only N method(s) agree (need M)" を
  維持し、曖昧証拠が除外されていた場合のみ末尾へ
  `"; N method(s) excluded as ambiguous"` を付加する
- Rule: 曖昧証拠の集計は merged 後の証拠列に対して行う。
  `min_agreeing_methods = 1` 下でも曖昧除外は適用されるが、非曖昧
  単一投票は受理し得る(契約の拒否権は投票除外を通じたもの)

### Rule C: merged_votes と best_window_match

- Rule: `merged_votes` は非曖昧の証拠を曖昧な証拠より優先して
  選択する(タプルキー `(validity != AmbiguousAlignment,
  first().score)`)。曖昧ペアの先頭スコアが高くても非曖昧ペアが
  選ばれ、**全ペアが曖昧の場合に限り** merged 証拠は曖昧のまま。
  同一クラス内の選択は既存の符号付きスコア比較を維持する
- Rule: `best_window_match` はペア単位の相関曲線で `|T| >= 2`
  ならそのペアはアンカーに寄与せず(`continue`)、**全ペアが
  同点または閾値未達の場合に限り `None`** を返す。narrow →
  ×8 slack → 全 `[lo,hi)` の3段全てに適用される

### Rule D: golden / 永続化の pin

- Rule: `pin-periodic` fixture は受理されず結果
  `validity == "ambiguous_alignment"`。厳密同点を持つ
  full_ncc / spectral_envelope の merged 証拠は
  `AmbiguousAlignment` + 候補保持で、full_ncc の `candidates` ≥2
  かつ全位置が周期(1024)の整数倍として観測可能。
  band_ncc / gcc_phat は厳密同点を持たず(帯域エッジ過渡・
  位相白色化で厳密周期が崩れる、T1 実測)非曖昧のまま — 拒否は
  「曖昧証拠の投票除外 + 非曖昧票の合意不足」の混在機構で成立
  する。判定は実行間および CPU dispatch(scalar / SIMD)間で
  決定的
- Rule: `pin-periodic-control` は正しい位置で受理(既存
  tolerance 64)。`case_repeated_source`・`case_loop_material`
  ほか受理系 golden は全て受理を維持し、受理系では全メソッドの
  merged 証拠が非曖昧(ペアレベルの曖昧が `merged_votes` で
  解決される限り AC 違反としない)
- Rule: `pinned_low_coherence_no_inflated_lag` は不受理・
  `agreeing_methods < 2` を維持
- Rule: 全メソッドの `method_version` と合議の
  `algorithm_version` が `"2"` で永続化される
- Rule: `tests/prepared.rs` の厳密同点(`template`/`−template`
  2出現)・周期 sin テンプレート fixture は新意味論(証拠
  `AmbiguousAlignment` + 候補保持)で pass する —
  `position == 6952` の候補観測断言と locate==prepared 同一性
  断言は validity を含めて存続する

### Rule E: corr0 完了証明

- Rule: `spectral_envelope` の逐次 NCC(`methods.rs:539-547`)を
  `dsp::corr0` へ置換した後も、pin-periodic golden の拒否が
  構造的に維持される(厳密同点 → 曖昧検出 → 拒否。argmax の
  偶然散乱への依存が解消されている)

## Examples

- Example(厳密同点): raw 曲線の2位置が厳密に等しい最大値を持つ
  入力(周期埋込み・`template`/`−template` ペア)を与えると、
  証拠は `AmbiguousAlignment` + 候補保持 + "indistinguishable" を
  含む reject_reason となる
- Example(平坦 plateau): `is_peak` の非対称条件が末尾1点へ潰す
  幅 ≥2 の等値 plateau を持つ曲線は、plateau 内サンプルが全て
  argmax 足り得るため曖昧 — 幅3 plateau は "3 positions" と報告
  される(サンプル数計数)
- Example(非曖昧の単峰): 競合する別位置を持たない幅広・平坦
  頭部の単一ピークや、上位候補が意味ある分離を持つ曲線は
  曖昧と誤判定されない
- Example(混在の空投票): 曖昧証拠と `InsufficientSupport` が混在
  して投票が空になった場合でも、分類は `AmbiguousAlignment`
  ("all voting methods produced ambiguous (tied) peaks")となり
  `LowCorrelation`/`InsufficientSupport` に落ちない
- Example(merged 優先): 同一メソッド内で曖昧ペアの先頭スコアが
  非曖昧ペアより高くても、merged 証拠は非曖昧ペアになる
- Example(best_window_match): 第3段(全 `[lo,hi)` 探索)でループ
  素材に遭遇し全ペアが同点となったウィンドウはアンカー `None`
  となり、occurrence 毎アンカーへのフォールバックに落ちる
  (受諾済み退化経路)
- Example(完了証明): corr0 置換後の `pin-periodic` で各 lag が
  厳密 1.0 同点になっても、stable sort の先頭 lag への argmax
  固定は起こらず全メソッドが `AmbiguousAlignment` を報告して
  不受理となる

## Guarantees

- 周期・ループ・縮退入力への拒否が、カーネルの丸め挙動ではなく
  構造的な同点検出に基づくこと — `dsp::corr0` のような決定的
  カーネル導入後も偽受理が再現されない
- 曖昧証拠でも候補列と `reject_reason` が evidence.json に
  永続化され、「どの位相が競合したか」を事後検査できる
  (マーカー句 "indistinguishable" で機械的識別可能)
- 曖昧起因の不受理が `LowCorrelation`/`InsufficientSupport` と
  区別され、`reject_reason` から原因を辿れる
- 受理系入力(正当な複数出現・ループ素材・非周期対照)で全
  メソッドの merged 証拠が非曖昧を維持し、曖昧性の false
  positive で受理を失わない
- dispatch 経路(scalar / SIMD)間で判定が一致すること —
  `eps ± dispatch_deviation` の irreducible 不安定帯を除く
  (帯の存在は契約として記述済み)

## Non-goals

- `rel_eps`/`abs_floor` の最終値自体の保証 — 値の確定と実測
  記録は `locate-argmax-tie-eps-calibration.md` の責務。本仕様は
  判定構造のみを pin する
- 周期性・自己相関モデルによる位相同値性の意味的検証
  (Option C)・位置間隔の周期性項(E-3) — 判定式で不足が判明
  した場合のエスカレーション先として留保
- `peaks_from_correlation` のピーク抽出意味・coarse 探索経路
  (`propose_coarse`/`refine_coarse`)の挙動
- `merged_votes` の符号付きスコア選択クオーク(候補空 0.0 が
  負スコアに勝つ)・`best_window_match` の最終選択が符号付き
  スコア基準である非対称 — 既知挙動として変更しない
- `LowCorrelation` 候補保持証拠が緩い閾値で投票し得る設定
  クオーク(spec Req 3 観察事項)
- 合議クラスタリング(tolerance クラスタ・contested 判定)の
  リファクタ正当性
- issue #42 / #52 の性能・カーネル作業本体

## Risk

- この保証を失うと、周期入力への拒否が再び浮動小数点ノイズの
  偶然に戻り、決定的カーネル導入(#42 系)のたびに偽受理が
  再現する — Phase 0 の先行ゲートが消える
- 候補保持や reject_reason pin を落とすと、不受理の原因が
  「弱い」のか「曖昧」なのかを evidence.json から判別できなく
  なり、provenance が失われる
- 分類優先順位を落とすと曖昧起因の拒否が `LowCorrelation` に
  誤分類され、「証拠が弱い」と「位置が一意でない」の区別が
  消費者から見えなくなる
- plateau を cluster 計数に変えたり判定を peak-list 検査に移すと、
  単一 plateau 曲線の曖昧検出を見落とし契約と矛盾する
