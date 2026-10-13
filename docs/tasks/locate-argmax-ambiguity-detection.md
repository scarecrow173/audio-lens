---
id: "TASK-034Fo9dDtnY4bqipAEqfDV"
type: "task"
status: "todo"
title: "argmax 曖昧性検出の実装: evidence 分岐・投票除外・merged_votes/best_window_match・バージョン"
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
  implemented-by: []
  depends-on:
    - "docs/plans/implement-locate-argmax-ambiguity.md"
    - "docs/tasks/locate-argmax-tie-eps-characterization.md"
  blocks:
    - "docs/tasks/locate-spectral-envelope-corr0.md"
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
    - "docs/test-specs/locate-argmax-ambiguity.md"
  references: []
  defers: []
  deferred-by: []
---

# argmax 曖昧性検出の実装: evidence 分岐・投票除外・merged_votes/best_window_match・バージョン

## Work

plan T2(design §1-§6 のコード変更一式 + ユニットテスト +
golden/prepared 追随)。T1 で確定した `TIE_REL_EPS`/
`TIE_ABS_FLOOR` を使う。

**methods.rs**:

- private const `TIE_REL_EPS`/`TIE_ABS_FLOOR`(T1 確定値)と
  `pub(crate) fn argmax_tie_count(corr: &[f64]) -> usize` を追加 —
  `M = max_i |corr[i]|`、`tie_window = max(TIE_REL_EPS·M,
  TIE_ABS_FLOOR)`、`T = { i : |corr[i]| >= M − tie_window }` の
  **サンプル(index)数**を返す(design §1 のコード形そのまま。
  plateau は run に潰さない。NaN は全比較 false で T に入らない)
- `evidence_from_corr`(`methods.rs:255`)の最終 `Some(_)` アーム
  を曖昧分岐へ置き換える: `argmax_tie_count(corr) >= 2` なら
  `validity = AmbiguousAlignment`、`candidates` は
  `peaks_from_correlation` 出力を不変で保持、`reject_reason =
  "top correlation peaks indistinguishable within tolerance
  ({tied} positions)"`(マーカー句 "indistinguishable" 必須、
  positions はサンプル数)。**既存の3分岐(no-peak →
  InsufficientSupport、非有限/非正 → LowCorrelation、p2b 不足 →
  LowCorrelation 候補保持)の後に評価**し、弱さで棄却済みの
  曲線を曖昧へ格上げしない
- 全4メソッド(`FullNcc`/`BandNcc`/`GccPhat`/`SpectralEnvelope`)
  の `version()` を `"1"` → `"2"`(`:327`/`:383`/`:456`/`:510`
  付近の各 impl)
- `mod tests` ユニットテスト: 厳密同点・近同点・平坦 plateau が
  `AmbiguousAlignment`+候補保持、単一候補・意味ある分離が非曖昧、
  **幅広・平坦頭部の単一ピーク(競合する別位置を持たない単峰
  曲線)が非曖昧**、符号 ±1.0 ペアが大きさ同点で曖昧

**consensus.rs**:

- `evaluate`(:65-212): `n_ambiguous` を merged 証拠列から集計し、
  投票フィルタ(:81)の述語を `!matches!(e.validity,
  Validity::ComputationFailed | Validity::AmbiguousAlignment)` へ
  拡張。dedup・`candidates.first()`・`min_score`/
  `min_peak_to_background` の順序は不変
- 分類優先順位: (1) `votes.is_empty() && n_ambiguous >= 1` →
  `AmbiguousAlignment` + `"all voting methods produced ambiguous
  (tied) peaks"`("method(s) agree" を含まない。**既存の
  `any_valid → LowCorrelation`/`InsufficientSupport` 分岐より前**に
  置き、副次フィールドは `:136-148` の空投票分岐と同値)
  (2) 空投票・曖昧なし → 既存不変 (3) 投票非空・合意不足 →
  既存 "only N method(s) agree (need M)" を維持し
  `n_ambiguous >= 1` のときのみ `"; {n_ambiguous} method(s)
  excluded as ambiguous"` を末尾付加 (4) contested・受理は不変
- モジュール内テストの `evaluate(.., "locate", "1")` を `"2"` へ
  更新 + 新ユニットテスト: 全証拠曖昧で `AmbiguousAlignment`
  (`LowCorrelation`/`InsufficientSupport` に落ちない)、混在時の
  分類優先順位、reason 文言
- 併せて、モジュール内テストの証拠ヘルパ `ev()`
  (`consensus.rs:222`)が立てる `method_version: "1"` も `"2"`
  へ揃える — Req 11 の永続化版スコープ外の fixture データだが、
  整合のため bump する

**stage.rs**:

- `merged_votes`(:450-464): 比較キーをタプル `(e.validity !=
  Validity::AmbiguousAlignment, e.candidates.first().map(|c|
  c.score).unwrap_or(0.0))` 化し、同メソッドなら
  `key(pair) > key(m)` で置き換え(非曖昧優先。同一クラス内は
  既存の符号付きスコア比較を維持 — 候補空 0.0 クオークは対象外)
- `best_window_match`(:228-256): `let corr = ncc(template,
  &cap_ch[lo..hi], hi - lo - window)` の直後に
  `if argmax_tie_count(&corr) >= 2 { continue; }` を追加 —
  **ペア単位ガード**。`None` を返すのは全ペアが同点または閾値
  未達の場合に限る。narrow → ×8 slack → 全 `[lo,hi)` の3段全てに
  効く(`window_anchors_in` :304-313 と occurrence 検証 :811 の
  呼出し経路を通じて)
- `algorithm_version` `"1"` → `"2"`: `:172`(`marked_result`)、
  `:671`(主 evaluate 呼出し)、`:1037`(residual child evaluate
  呼出し)の3箇所
- `residual.rs:321` の `#[cfg(test)]` stub `algorithm_version: "0"`
  は**変更しない**(記録用 stub であり、永続化版は :1037 が決める)

**golden.rs / prepared.rs 追随**:

- `pinned_periodic_everywhere_not_accepted_at_wrong_phase`(:598):
  reject_reason 断言を "method(s) agree" から新 pin 文言
  ("all voting methods produced ambiguous (tied) peaks")へ更新
  (意図的契約変更 — 理由を変更記録へ)。全4メソッドの merged
  証拠が `AmbiguousAlignment` + 候補保持である証拠レベル pin を
  追加、full_ncc 候補 ≥2・周期倍数断言は維持。control は受理
  (tolerance 64)を維持
- 受理系 fixture(`pin-periodic-control`、`case_repeated_source`、
  `case_loop_material` ほか受理を維持すべき golden 全件)に対し、
  **全メソッドの merged 証拠が `AmbiguousAlignment` でない**
  ことを断言する証拠レベル pin を golden.rs に追加する。spec
  AC「受理系 fixture では全メソッドの merged 証拠が非曖昧」の
  担当は本タスク — T1 はマージン分布の計測・記録のみで、この
  AC を pin しない(ペアレベルで曖昧が生じても `merged_votes`
  が非曖昧ペアへ解決する限り AC 違反とはしない、という spec
  の但し書き通り merged レベルで見る)
- `tests/prepared.rs`: `:135` の `al_locate::evaluate(.., "test",
  "1")` → `"2"`。厳密同点(`template`/`−template` 2出現)・周期
  sin テンプレートで証拠 validity が `Valid`→`AmbiguousAlignment`
  +候補保持へ意味的に変わる — `position == 6952` 観測断言と
  locate==prepared 同一性断言は候補保持により存続、受理可否比較は
  「同じ分類に落ちる」形で存続するよう追随。意味変化は変更記録へ

**types.rs(必要時)**: `AmbiguousAlignment`(`al-core/src/types.rs
:80-81`)の doc コメントにメソッド証拠用途を明記 — variant 追加
なし、doc のみ。

### Out of scope(本タスク)

- `peaks_from_correlation` 本体・`propose_coarse`/`refine_coarse`
  への適用
- `methods.rs:539-547` の `dsp::corr0` 置換(T3)
- `Validity` 新 variant・`propagate_validity` 変更、
  `ConsensusConfig` への eps 露出、合議クラスタリングのリファクタ

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-locate-argmax-ambiguity.md`(T2)
- `docs/designs/locate-consensus-argmax-ambiguity.md` §1-§6(コード
  形・分岐順序・分類優先・キー・ガード・バージョンスイープの正本)
- `docs/specs/locate-consensus-ambiguity-contract.md` Req 1-5, 8-11
- `docs/adr/locate-argmax-ambiguity-representation.md`(B-1/D-1/E-2)
- 検証仕様: `docs/test-specs/locate-argmax-ambiguity.md`

## Dependencies

- `docs/tasks/locate-argmax-tie-eps-characterization.md`(T1):
  `TIE_REL_EPS`/`TIE_ABS_FLOOR` の確定値と eps 窓の存在確認が前提。
  T1 がエスカレーションを示した場合は本タスクを開始しない

## Affected Files

- `crates/al-locate/src/methods.rs`(const + `argmax_tie_count` +
  曖昧分岐 + version + ユニットテスト)
- `crates/al-locate/src/consensus.rs`(フィルタ・分類・文言・
  テスト更新)
- `crates/al-locate/src/stage.rs`(`merged_votes`・
  `best_window_match`・`algorithm_version`)
- `crates/al-locate/tests/golden.rs`(契約 pin 更新 + 証拠 pin)
- `crates/al-locate/tests/prepared.rs`(意味論シフト追随 + `"2"`)
- `crates/al-core/src/types.rs`(doc コメントのみ、必要時)

## Done When

- [ ] `argmax_tie_count` がサンプル数計数・`|corr|` 大きさ基準・
  rank-1 anchored で実装され、`evidence_from_corr` の曖昧分岐が
  既存 reject 分岐の後に評価される
- [ ] 曖昧証拠が `AmbiguousAlignment` + 候補保持 +
  "indistinguishable" を含む reject_reason を持つ
- [ ] `evaluate` が曖昧証拠を投票から除外し、空投票+曖昧証拠
  ≥1 で `AmbiguousAlignment` + 専用文言を既存分類より先に返す
- [ ] `merged_votes` が非曖昧ペアを優先し、全ペア曖昧時のみ
  merged が曖昧となる
- [ ] `best_window_match` が同点ペアを `continue` で非寄与とし、
  全ペア同点/閾値未達時のみ `None` を返す
- [ ] 全4メソッド `method_version` と `algorithm_version` が `"2"`
  で永続化される(4 impl + stage.rs 3箇所 + テスト記録引数)
- [ ] 新規ユニットテスト(flat plateau 曖昧・幅広平坦頭部単峰
  非曖昧・符号 ±1.0 同点・全証拠曖昧分類・文言 pin)が pass
- [ ] golden の受理系 fixture 全件で全メソッドの merged 証拠が
  非曖昧である証拠レベル pin を追加(T1 計測ではなく本タスクの
  責務)
- [ ] golden/prepared が新契約で green

## Verification

- [ ] `mise exec -- cargo test -p al-locate` が green(新規ユニット
  テスト・golden・prepared を含む)
- [ ] `mise exec -- cargo test -p al-math` が green
- [ ] `mise exec -- cargo build --workspace` が green
- [ ] `mise exec -- cargo clippy --workspace --all-targets -- -D
  warnings`・`mise exec -- cargo fmt --all -- --check` が green
- [ ] `rg '"1"' crates/al-locate/src/stage.rs` で
  `algorithm_version` の `"1"` 残存がないこと(`residual.rs` の
  stub は対象外)
