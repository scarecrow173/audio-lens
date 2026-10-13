---
id: "ADR-034FlIKrgxUPy1AQk3WGlb"
type: "adr"
status: "accepted"
title: "合議 argmax 曖昧性の証拠レベル表現"
created: "2026-10-13"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/audio-lens/issues/59"
    - "https://github.com/scarecrow173/audio-lens/pull/49"
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
  related:
    - "simd-reduction-lane-fma.md"
    - "multi-method-locate-consensus.md"
    - "../specs/locate-consensus-ambiguity-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/locate-consensus-argmax-ambiguity-contract.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../tasks/al-locate-vec-migration.md"
    - "../impl/ir/al-locate-dsp-re-export-vec-migration.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 合議 argmax 曖昧性の証拠レベル表現

## Context and Problem Statement

locate の合議は各メソッド証拠の argmax(先頭候補)を投票として扱う。
現状、周期入力への拒否は逐次 NCC の浮動小数点丸め誤差による
**偶然のピーク散乱**に依存している — `dsp::corr0` などの決定的
カーネルに置換すると厳密同点が生じ、`peaks_from_correlation` の
stable sort(`crates/al-locate/src/methods.rs:221`)により argmax が
最小 lag に固定され、複数メソッドが誤った位置で合意して偽受理が
再現される。このため spectral-envelope NCC の `corr0` 置換は
TASK-034EpPesSpbjbAQBsmjvJx / PR #49 で明示的に保留されている
(`methods.rs:539-543` の残存許容コメント)。

「位置が特定できる」の必要条件を、最大スコアだけでなく
**ピークの一意性・分離度**で契約化する必要がある — 曖昧性を
どこで検出し、どう表現し、合議がどう扱うか。これは #42
(FFT カーネル変更)・#40(coarse-to-fine)など数値挙動を変える
全作業の先行ゲートであり、永続化される `MethodEvidence`
(evidence.json / result.json)の意味論に触れるため ADR で記録する。

## Decision Drivers

* 偽受理の抑止が浮動小数点ノイズの偶然ではなく構造的な検出に
  基づくこと(決定的カーネル下でも成立すること)
* 曖昧性の証跡(同点候補群と理由)が永続化成果物に残り、
  デバッグ・検証に使えること
* 正当な複数出現(`case_repeated_source` / `case_loop_material`、
  ピーク間スコア差は ~1e-3 級の意味ある分離という**仮説** —
  T1 特性計測で検証予定)を誤って曖昧扱いしないこと
* 永続化語彙(`Validity` の snake_case シリアライズ)と下流消費者
  (`propagate_validity`、occurrence クラスタリング、al-viz)との整合
* CPU dispatch バックエンド(Scalar / SIMD)間で曖昧性判定が
  不変であること

## Considered Options

上位選択(ダッシュボード Option A/B/C):

* A. argmax の決定的タイブレーク(同点なら最小 lag / prior 順で確定)
* B. ピーク曖昧性を第一級出力化し、曖昧メソッドを合議投票から除外
* C. 周期モデル / 自己相関による位相同値性の検証

表現軸(B の内部選択):

* B-1. `MethodEvidence.validity` に既存 `Validity::AmbiguousAlignment`
  を流用
* B-2. 新 `Validity` variant(例 `AmbiguousEvidence`)を追加
* B-3. `MethodEvidence` に独立した曖昧性フラグ / reason フィールドを追加

検出位置軸:

* D-1. `evidence_from_corr`(全4メソッドの共有漏斗)内で raw 相関
  曲線に対して検出 — `peaks_from_correlation` は不変
* D-2. 合議(`consensus::evaluate`)側で検出

判定式軸:

* E-1. `peak_ratio_12 ≈ 1`(上位2ピーク比のみ)で同点判定
* E-2. raw 相関曲線上で `max_i |corr[i]|` から eps 窓内の同点集合
  を数える
* E-3. スコア同点 AND 位置間隔の周期性/稠密性の複合判定

## Decision Outcome

Chosen option: "B. ピーク曖昧性を第一級出力化"(表現 B-1 + 検出位置
D-1 + 判定式 E-2 の組合せ)。曖昧性をメソッド証拠の第一級状態と
して表現し、偶然の FP ノイズではなく構造的な判定で偽受理を抑止
するため。

具体的な契約:

* **表現**: `MethodEvidence.validity` に既存の
  `Validity::AmbiguousAlignment`("The alignment underpinning the
  measurement is ambiguous.", `crates/al-core/src/types.rs:80-81`)を
  流用する。候補リストは保持したまま、`reject_reason` で同点で
  ある旨を説明する(非 Valid validity + 候補保持は
  `evidence_from_corr` の p2b 不足分岐、`methods.rs:272-280` の
  既存先例と同型)。
* **検出位置**: `evidence_from_corr`(`methods.rs:255`)内で、
  raw 相関曲線に対して判定する。`peaks_from_correlation` は
  **不変** — 同関数には契約対象外の呼出し(`propose_coarse`
  `methods.rs:74`、`refine_coarse` `methods.rs:109`、
  `best_window_match` `stage.rs:245`)がある。全4メソッド
  (`full_ncc` / `band_ncc` / `gcc_phat` / `spectral_envelope`)が
  この漏斗を通るため、全メソッド・全チャンネルペアへ一様に
  適用される。曖昧性判定は Valid となるはずの証拠にのみ適用
  され(no-peak・非有限・p2b 不足の各 reject 分岐の後)、
  弱さで棄却済みの曲線は `LowCorrelation` のままとなる。
* **合議**: 曖昧とマークされた証拠を投票から除外し
  (`consensus.rs:81` の `validity != ComputationFailed` フィルタを
  拡張)、曖昧性に起因する不受理(投票空/不足)は
  `AmbiguousAlignment` として分類する(現状は
  `LowCorrelation` / `InsufficientSupport` に落ちる)。
* **禁止事項**: argmax の決定的タイブレーク(同点時の最小 lag /
  prior 順への確定)を確信の根拠としてはならない。
* **ゲート条件**: spectral-envelope NCC の `dsp::corr0` 移行は
  本契約の確立(厳密同点 → 構造的曖昧検出 → 拒否維持の検証)を
  前提とする。

### Consequences

* Good, because 周期入力への拒否が FP ノイズの偶然ではなく構造的
  検出に基づくものとなり、`dsp::corr0` 等の決定的カーネル導入が
  可能になる(#42・#40 等の先行ゲートを解除)。
* Good, because 同点候補群と `reject_reason` が evidence.json /
  result.json に永続化され、provenance が保存される。
* Good, because B-1 では下流が変更不要:
  `propagate_validity`(`crates/al-analysis/src/records.rs:79`)は
  `AmbiguousAlignment | LowCorrelation → AmbiguousAlignment` に既に
  写し、occurrence クラスタリング(`crates/al-locate/src/stage.rs:703-741`)
  は `Validity::Valid` のみを位置寄与とし、al-viz
  (`crates/al-viz/src/report.html:702`)は `ev.validity` をそのまま
  描画する。シリアライズ語彙(`snake_case` の
  `ambiguous_alignment`)も変わらない。
* Bad, because 厳密同点で繰り返し出現する縮退入力(例: ノイズ床
  なしの gain 1.0 多重埋込み)は曖昧として報告される — 正当な
  複数出現との区別が score 同点だけでは不能な場合の挙動であり、
  文書化された契約として受け入れる。
* Bad, because 曖昧起因の拒否では `reject_reason` の文言が変わり
  得る。golden が pin する "method(s) agree" 文字列の変更は
  意図的な契約変更として扱い、spec で確定する。
* Bad, because 合議の分類規則に曖昧性を考慮した分岐が増える
  (曖昧証拠除外で投票が空/不足になった場合に `AmbiguousAlignment`
  を返す経路 — 空/不足時に曖昧証拠が ≥1 あれば
  `any_valid → LowCorrelation` / `InsufficientSupport` の判定より
  先に `AmbiguousAlignment` へ分類する)。
* Bad, because `merged_votes`(`stage.rs:450-464`)のメソッド内
  チャンネルペア統合の順序規則が変わる — 現行は先頭スコアのみで
  選択するが、非曖昧の証拠を曖昧な証拠より優先して選ぶ規則へ
  変更する(曖昧ペアのスコアが高くても `Valid` ペアを選ぶ)。

### Confirmation

数値決定性の要求: 曖昧性の判定結果は CPU dispatch バックエンド
(Scalar / V3-FMA 等)間で同一でなければならない。「同点」とみなす
スコア許容窓は `tie_window = max(rel_eps · M, abs_floor)`(M は
raw 曲線の最大 |corr|)とし、`rel_eps` は
`max(dispatch 間 FP 偏差) << rel_eps << min(意味あるピーク分離)` の
範囲として較正される。この eps 意味論は実装に対して normative で
ある(具体値は T1 特性計測を経て spec で確定)。al-locate 側から
到達可能な dispatch override seam は存在せず(`al-math` の
`dispatch` は `pub(crate)`)、dispatch 不変性は eps の較正
(irreducible な `eps ± dispatch_deviation` 不安定帯を除く反転
不可能性)と al-math 自身の scalar↔SIMD 等価テストで担保する —
al-math への seam 追加は本決定のスコープ外とする。

## Implementation Plan

* Affected paths: `crates/al-locate/src/methods.rs`
  (`evidence_from_corr` の曖昧性検出分岐 — `peaks_from_correlation`
  は不変)、`crates/al-locate/src/consensus.rs`(投票フィルタ拡張 +
  空投票/不足時の分類分岐)、`crates/al-locate/src/stage.rs`
  (`merged_votes` の非曖昧優先選択、`best_window_match` の同点
  ガード)、`crates/al-locate/tests/golden.rs`(契約 pin の更新)、
  将来の `methods.rs` spectral-envelope NCC `dsp::corr0` 移行
* Patterns to follow: 非 Valid validity + 候補保持の既存先例
  (`evidence_from_corr` の p2b 不足分岐が `LowCorrelation` +
  candidates 保持、`methods.rs:272-280`)、`Validity` の意味論
  ドキュメント(`types.rs:62-92`)
* Patterns to avoid: argmax の決定的タイブレークを確信の根拠に
  すること、`Validity` とは別の並行フラグで曖昧性を二重表現する
  こと、曖昧検出をメソッド個別実装や合議側へ分散させること
* Dependencies or configuration: 判定式(E-2 同点集合・相対 eps +
  絶対床)と `reject_reason` 文言は spec で確定する(数値確定は
  T1 特性計測が前提)。`best_window_match` への適用は spec で
  同点ガードとして確定済み。

将来 B-2 方向(新 `Validity` variant)へ移行する場合の注意:
`propagate_validity`(`records.rs:85`)の `_ => ComputationFailed`
catch-all が新 variant を暗黙に `ComputationFailed` へ劣化させる。
variant を追加する際は同関数への明示アーム追加が必須である。

## Verification

* [ ] 曖昧と判定された `MethodEvidence` が
  `validity == ambiguous_alignment`・候補保持・`reject_reason` 付きで
  evidence.json に永続化される
* [ ] 曖昧証拠が合議の投票(`consensus.rs` のフィルタ)から除外され、
  曖昧起因の不受理が `AmbiguousAlignment` に分類される
* [ ] `pinned_periodic_everywhere_not_accepted_at_wrong_phase` 相当の
  golden が決定的カーネル下でも不受理を維持する
* [ ] `case_repeated_source` / `case_loop_material` は受理を維持する
  (正当な複数出現の誤判定なし)
* [ ] Scalar / SIMD dispatch 両経路で同一入力の曖昧性判定が一致する
* [ ] `dsp::corr0` 移行は本契約の検証完了後にのみ実施される

## Pros and Cons of the Options

### A. argmax の決定的タイブレーク(同点なら最小 lag / prior 順で確定)

* Good, because 差分が最小で、結果の再現性は改善される。
* Bad, because 誤った候補を**自信付きで受理**する — 構造問題の
  解消にならず、ダッシュボードの選択肢検討でも非採用。

### B. ピーク曖昧性を第一級出力化(採用: B-1 + D-1)

* Good, because 誤受理を構造的に抑止し、周期・ループ素材へ
  一般化でき、証跡(evidence / reject_reason)を保持する。
* Bad, because 検出閾値・合議分岐・golden の契約更新が要る。

### B-1. `Validity::AmbiguousAlignment` 流用(採用)

* Good, because 意味論が一致する("The alignment underpinning the
  measurement is ambiguous."、`types.rs:80-81`)。
  `propagate_validity` が既に正しく写し、occurrence クラスタリングの
  Valid フィルタと一貫し、シリアライズ語彙を変えず、viz も変更不要。
* Bad, because 結果レベルの ambiguous とメソッドレベルを同一語で
  表す(意味は近いが層が違う)。

### B-2. 新 `Validity` variant(例 `AmbiguousEvidence`)

* Good, because メソッド/結果の層を語彙で区別できる。
* Bad, because `propagate_validity` の `_ => ComputationFailed`
  catch-all(`records.rs:85`)で暗黙に `ComputationFailed` へ劣化する
  ため同時の明示マッピング修正が必須となり、永続化語彙も増える。
  層区別の価値が現時点のコストを上回らないため見送る — 将来
  variant を追加する場合は `propagate_validity` への明示アームが
  必須。

### B-3. `MethodEvidence` への独立フラグ / reason フィールド

* Good, because `Validity` 語彙を汚さない。
* Bad, because 構造体・serde・消費者の全変更が要り、validity と
  flag の二重表現による表現ドリフトを生む。

### C. 周期モデル / 自己相関による位相同値性の検証

* Good, because 「同一事象の位相」と「別出現」を意味的に最も
  正確に区別できる。
* Bad, because 計算量・閾値・モデルが増え調整コストが大きく
  #59 スコープを超過する — E-2 型のスコア同点検出で不足が判明
  した場合の将来強化候補として保留する。

### D-1. `evidence_from_corr` で検出(採用)

* Good, because 全4メソッド・全チャンネルペアへ一様適用でき、
  候補保持の既存先例(`LowCorrelation`)と同型で、evidence への
  provenance が得られる。raw 相関曲線への判定なので、`is_peak`
  の非対称条件が潰す平坦 plateau 同点も捕捉できる。
* Bad, because per-pair と merged の両方に効く — merged は clone で
  自動伝播するが、`merged_votes` の選択規則を「非曖昧優先」へ
  変更する必要がある(Consequences 参照)。
* Bad, because `peaks_from_correlation` 内では検出できない —
  coarse 探索(`propose_coarse` / `refine_coarse`)と
  `best_window_match` が同関数を共有するため、判定を funnel の
  呼出し側である `evidence_from_corr` に置く分岐構成が要る。

### D-2. 合議側で検出

* Good, because メソッド証拠を変えない。
* Bad, because メソッド毎の候補分布しか見えず判定材料は D-1 と
  同じであり、evidence への記録(provenance)がなく、occurrence 側の
  Valid フィルタと不整合になる。

### E-1. `peak_ratio_12 ≈ 1`(上位2ピーク比のみ)

* Good, because 既存フィールドを使え、viz も既に表示している。
* Bad, because top-2 のみを見るため平坦 plateau や 3方向以上の
  同点構造を見落とし、第2ピーク不在時は `None` で判定不能になる。
  不採用。

### E-2. raw 曲線上の eps 窓同点集合(採用)

* Good, because 「argmax が統計的に一意でない」を直接表現し、
  plateau と多方向同点の双方を包含する。eps は T1 特性計測で
  dispatch 偏差に対して較正可能(絶対床 `abs_floor` で 0 近傍も
  安定)。
* Bad, because eps の較正が必須で、受理系 fixture の実スコア分離
  (~1e-3 級)は現時点で仮説 — T1 で検証し、範囲が成立しない
  場合は E-3 拡張または Option C へのフォールバックとして spec
  改訂を伴うエスカレーションとする。

### E-3. スコア同点 AND 位置間隔の周期性/稠密性

* Good, because repeated/loop の等間隔配置による偽陽性を追加で
  抑制し得る。
* Bad, because 判定が複合化して調整コストが増える。延期された
  フォールバックとする — スコアのみの基準(E-2)が正当な繰返しと
  位相曖昧を分離できないと T1 が示した場合にのみ再検討する。

## More Information

* 上流探索: `../discovery/locate-consensus-argmax-ambiguity-contract.md`
  (DISC-034Fl0OeHprFoFHitMqBid) — 確認済みコード事実、選択肢
  マトリクス(A/B/C・B-1..B-3・D-1/D-2・E-1..E-3)、eps 較正と
  正当な複数出現の緊張に関する Open Questions
* 上位整理: `../discovery/remaining-issues-roadmap.md` の #59 節
  (Option A/B/C、B 採用、Phase 0)
* Issue: [#59 合議argmaxのFPノイズ依存を解消](https://github.com/scarecrow173/audio-lens/issues/59)
* 証拠: [PR #49 — `dsp::corr0` 置換の意図的見送り](https://github.com/scarecrow173/audio-lens/pull/49)
* 本 ADR が refine する決定: `multi-method-locate-consensus.md`
  (複数手法の合議による受理)
* dispatch 間数値等価の前提: `simd-reduction-lane-fma.md`
  (許容誤差による等価性定義)
* 残存許容の経緯: `../tasks/al-locate-vec-migration.md`、
  `../impl/ir/al-locate-dsp-re-export-vec-migration.md`
* 判定式(eps・絶対床・同点集合基準)・`reject_reason` 文言・
  `best_window_match` 同点ガード・golden 更新方針は spec で確定
  する(本 ADR の対象外)
