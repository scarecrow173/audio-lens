---
id: "PLAN-034Fo8JsxsjegvrZWwahTO"
type: "plan"
status: "approved"
title: "locate 合議 argmax 曖昧性検出の実装"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
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
  implements:
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
  implemented-by:
    - "docs/tasks/locate-argmax-tie-eps-characterization.md"
    - "docs/tasks/locate-argmax-ambiguity-detection.md"
    - "docs/tasks/locate-spectral-envelope-corr0.md"
    - "docs/tasks/locate-argmax-ambiguity-closeout.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/multi-method-locate-consensus.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/locate-argmax-ambiguity.md"
    - "docs/test-specs/locate-argmax-tie-eps-calibration.md"
  references:
    - "docs/discovery/locate-consensus-argmax-ambiguity-contract.md"
    - "docs/tasks/al-locate-vec-migration.md"
    - "docs/impl/ir/al-locate-dsp-re-export-vec-migration.md"
  defers: []
  deferred-by: []
---

# locate 合議 argmax 曖昧性検出の実装 Implementation Plan

## Goal

approved design `docs/designs/locate-consensus-argmax-ambiguity.md`
(DESIGN-034Fn25qh0mE8cz5ymYKfD) に基づき、locate 合議の argmax
曖昧性を第一級契約として実装する。`methods.rs` にサンプルレベル
同点集合判定 `argmax_tie_count`(`TIE_REL_EPS`/`TIE_ABS_FLOOR` private
const)を追加し `evidence_from_corr` に曖昧分岐(候補保持 +
"indistinguishable" `reject_reason`)を設け、`consensus::evaluate` の
投票フィルタ拡張・空投票時の分類優先順位・reason 文言、`stage.rs`
の `merged_votes` タプルキー非曖昧優先と `best_window_match`
ペア単位同点ガード、全 `method_version`/`algorithm_version` の
`"2"` バンプまでを T2 で land させ、T3 で `spectral_envelope` 逐次
NCC の `dsp::corr0` 移行(残存許容解消の完了証明)を行う。
`TIE_REL_EPS`/`TIE_ABS_FLOOR` の最終値は T1 特性計測
(受理系 fixture の `M − max_{i≠argmax}|corr[i]|` マージン分布と
al-math 内 scalar↔dispatch 偏差実測)で確定する — eps 実現可能
窓が存在しないと T1 が示した場合は実装せず spec Req 7 の
エスカレーション(E-3 / Option C)へ回す。

## Scope

単一の実装ストリームとして扱い、design §8 の T1–T4 を4タスクに
写す。T1(特性計測)は T2(実装)のゲート、T3(corr0 移行)は T2 の
契約定着を前提とする完了証明、T4 が計測・IR・closeout — 全て同一の
critical path(判定式と eps 値の共有)上にあり分割しない。

- In scope(design Scope を継承): `crates/al-locate/src/methods.rs`
  (`argmax_tie_count` + `TIE_REL_EPS`/`TIE_ABS_FLOOR` +
  `evidence_from_corr` 曖昧分岐 + 全4メソッド `version()` `"2"`)、
  `crates/al-locate/src/consensus.rs`(投票フィルタ拡張・空投票分類
  優先・reject_reason 注記)、`crates/al-locate/src/stage.rs`
  (`merged_votes` タプルキー・`best_window_match` ペアガード・
  `algorithm_version` 呼出し更新)、`crates/al-locate/tests/golden.rs`
  (契約 pin 更新 + 証拠レベル pin 追加)、
  `crates/al-locate/tests/prepared.rs`(意味論シフト追随 +
  `evaluate(.., "test", "2")`)、`crates/al-math` の既存
  `#[cfg(test)]` モジュール内への特性計測テスト追加、
  `docs/impl/ir/` への実装記録
- Out of scope(spec/design の Out of Scope を継承):
  `peaks_from_correlation` 自体の変更と `propose_coarse`
  (`methods.rs:74`)・`refine_coarse`(`methods.rs:109`)への適用、
  Option C / E-3(T1 エスカレーション先として留保)、al-math
  dispatch override seam 新設、`Validity` 新 variant・
  `propagate_validity` 変更、合議クラスタリングのリファクタ、
  `merged_votes` の符号付きスコア選択クオーク(候補空 0.0 が
  負スコアに勝つ既知挙動 — 記録のみ)、issue #42 / #52 の性能・
  カーネル作業本体

## Upstream Documents

- `docs/specs/locate-consensus-ambiguity-contract.md`(approved,
  SPEC-034FlHW8GmjrKvTJlf0UWw): Req 1-11 の pin 済み契約(判定式
  `T = { i : |corr[i]| >= M − tie_window }`・`|T| >= 2` で曖昧、
  分岐順序、投票フィルタ順序、分類優先順位、merged_votes・
  best_window_match 規則、reject_reason 文言、バージョン方針)
  と Acceptance Criteria。eps の最終値のみ T1 実測へ委任
- `docs/adr/locate-argmax-ambiguity-representation.md`(accepted,
  ADR-034FlIKrgxUPy1AQk3WGlb): 表現 B-1(`AmbiguousAlignment`
  流用)・検出位置 D-1(`evidence_from_corr` raw 曲線)・判定式
  E-2(rank-1 anchored 相対 eps 窓)の確定記録と dispatch 不変性の
  normative 要求
- `docs/discovery/locate-consensus-argmax-ambiguity-contract.md`
  (DISC-034Fl0OeHprFoFHitMqBid): 確認済みコード事実と選択肢
  マトリクスの出典(背景。directional な上流は spec/ADR/design)

## Design Inputs

- `docs/designs/locate-consensus-argmax-ambiguity.md`(approved):
  本計画の正本。`argmax_tie_count` のコード形(サンプル数計数・
  `|corr|` 大きさ比較・NaN/±inf 扱い)、`evidence_from_corr` の
  分岐置換構造、`evaluate` の `n_ambiguous` 集計と4段の分類
  優先順位、`merged_votes` タプルキー、`best_window_match`
  `continue` ガード、version バンプの全対象箇所(4 impl +
  `stage.rs:172/671/1037` + `consensus.rs` 内テスト +
  `prepared.rs:135`)、T1 計測表(マージン分布・ショルダーデルタ・
  `spectral_envelope` 慢性曖昧・同点構造・dispatch 偏差)、
  golden/prepared の意味論シフト、受諾事項(第3段全滅退化・
  "all" 文言の近似・plateau サンプル計数)

## File Map

| Path | Responsibility | Task |
|------|----------------|------|
| `crates/al-locate/src/methods.rs` | `TIE_REL_EPS`/`TIE_ABS_FLOOR` private const + `pub(crate) fn argmax_tie_count`(同点判定の唯一の実装)+ `evidence_from_corr` 曖昧分岐 + 4 `version()` `"2"` + ユニットテスト。`:539-547` 逐次 envelope NCC → `dsp::corr0` + 残存許容コメント除去 | T1, T2, T3 |
| `crates/al-locate/src/consensus.rs` | `n_ambiguous` 集計 + validity フィルタ拡張 + 空投票の曖昧優先分岐 + 合意不足分岐への `"; N method(s) excluded as ambiguous"` 付加 + モジュール内テストの `evaluate(.., "locate", "2")` 更新 + 新ユニットテスト | T2 |
| `crates/al-locate/src/stage.rs` | `merged_votes` タプルキー(非曖昧優先)+ `best_window_match` の `argmax_tie_count(&corr) >= 2` → `continue` ガード + `algorithm_version` `"1"`→`"2"`(`:172`/`:671`/`:1037`)+ ユニットテスト | T2 |
| `crates/al-locate/tests/golden.rs` | `pinned_periodic_everywhere_not_accepted_at_wrong_phase` の reject_reason 断言を新文言へ更新 + 全4メソッド merged 証拠の `AmbiguousAlignment` + 候補保持 pin 追加 | T2, T3 |
| `crates/al-locate/tests/prepared.rs` | 厳密同点(`template`/`−template` 2出現)・周期 sin テンプレート fixture の証拠 `Valid`→`AmbiguousAlignment`+候補保持への意味論シフト追随 + `:135` の `evaluate(.., "test", "2")` | T1, T2 |
| `crates/al-locate/tests/locate.rs` | T1 の計測対象として参照(`different_rates_locate_without_false_drift` ~:488、`repeated_source_reports_second_occurrence` ~:440)。コード変更なし | T1 |
| `crates/al-math/src/correlation.rs` `mod tests` | `Correlation` カーネルの scalar↔dispatch 偏差を実運用 FFT サイズ(~64k-point RealFFT 規模)で特性計測 | T1 |
| `crates/al-math/src/filter.rs` `mod tests` | `Envelope` カーネルの scalar↔dispatch 偏差を実運用 fft_size で特性計測 | T1 |
| `crates/al-core/src/types.rs` | `AmbiguousAlignment` の doc コメントへメソッド証拠用途の明記(必要に応じて。variant 追加なし) | T2 |
| `docs/impl/ir/<record>.md` | T1 実測値(margin 分布・dispatch 偏差・確定 eps)、T2/T3 差分と golden 結果、T4 回帰計測の記録 | T1–T4 |

変更しない: `peaks_from_correlation` 本体、`propose_coarse` /
`refine_coarse`、`candidate.rs` の `MethodEvidence` 構造、
`Validity` variant 集合、`propagate_validity`
(`al-analysis/src/records.rs:79`)、occurrence クラスタリング
(`stage.rs:703-741` の Valid フィルタは不変で正しい)、
`residual.rs:321` の `#[cfg(test)]` stub `algorithm_version: "0"`、
`ConsensusConfig`(eps は設定面に露出しない)、al-math の
`dispatch` 可視性・override seam。

## Implementation Sequence

| Task | 内容 | Depends on | Blocks |
|------|------|------------|--------|
| T1 | 特性計測(コード変更前): (a) al-locate 側 — 全受理系 fixture(golden 群 + `tests/locate.rs` 受理系 + `tests/prepared.rs`)でメソッド毎に `M − max_{i≠argmax}\|corr[i]\|` マージン分布を計測し、`spectral_envelope` の noise 系入力での慢性曖昧頻度と周期 fixture の同点構造を記録;(b) al-math 側 — `ncc`/`gcc_phat`/`spectral_envelope` の実運用 FFT サイズでの scalar↔dispatch 偏差上限を既存 `#[cfg(test)]` モジュール内テストで実測。`max(dispatch 偏差) << rel_eps << min(マージン)` の窓を示して `TIE_REL_EPS`/`TIE_ABS_FLOOR` を確定。窓不存在なら spec Req 7 エスカレーション | — | T2 |
| T2 | 実装: `argmax_tie_count` + 2 const(T1 確定値)、`evidence_from_corr` 曖昧分岐、`evaluate` フィルタ/分類/文言、`merged_votes` タプルキー、`best_window_match` ガード、version `"2"` スイープ、ユニットテスト(flat plateau 曖昧・幅広平坦頭部の単峰は非曖昧・符号 ±1.0 同点)、golden/prepared 追随 | T1 | T3 |
| T3 | corr0 移行(完了証明): `methods.rs:539-547` 逐次 envelope NCC → `dsp::corr0`、残存許容コメント除去、pin-periodic golden で厳密同点 → 構造的曖昧検出 → 拒否維持を検証 | T2 | T4 |
| T4 | 計測・IR・closeout: bench/回帰確認、Implementation Record 確定(T1 実測・eps 確定値・T2/T3 差分・文言変更の意図的契約変更記録)、関連 doc 更新、最終検証 | T3 | — |

並列化可能な部分は T1 内の al-locate 計測と al-math 計測のみ
(別 crate・別ファイル)。T2–T4 は同一ファイル群を直列に積む。

## Dependencies and Risks

依存: 外部依存の追加なし。`pulp::Scalar` は al-math の crate 内
テストから既に到達可能(`lib.rs` テストの `pulp::Scalar.vectorize`
先例)。`dsp::corr0` は `dsp.rs:38` で re-export 済み。

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | eps 実現可能窓が存在しない(受理系 fixture の最小マージン ≦ dispatch 偏差、または帯域制限ソースのショルダーデルタ ~1e-6..1e-3 が候補レンジと衝突) | med | high | T1 を実装ゲートとし、窓不存在なら T2 に進まず spec Req 7 経路(E-3 拡張または Option C)で spec 改訂へエスカレーション。T1 コミットは計測テストのみで revert 不要に残せる |
| 2 | `spectral_envelope` が noise/チャープ/広帯域 SFX で慢性曖昧(ほぼ全 lag 準同点)となり投票を恒常停止 | med | high | T1 で `tests/locate.rs` の非 golden 受理系(チャープ ~:488・ノイズ埋込み ~:440)を含めて頻度を実測。示された場合は eps 再較正ではなく spec 改訂(AC「受理系では全メソッド merged 非曖昧」の再検討を含む) |
| 3 | `best_window_match` 第3段(全 `[lo,hi)` argmax)で全ペア同点 → ウィンドウアンカー全滅、occurrence 毎アンカーへ退化して time_map が drift/jitter 観測を失う | med | med | design §5 の受諾事項として明記済み。golden/acceptance で time_map 相当経路の回帰を監視し、退化が観測されたら IR に記録 |
| 4 | reject_reason 文言変更による golden 破壊("method(s) agree" pin) | high | low | 意図的な契約変更として `pinned_periodic_*` の断言を新 pin 文言へ更新し、決定文言と理由を変更記録(spec Req 10・design §6) |
| 5 | `tests/prepared.rs` の厳密同点/周期テンプレートで証拠 validity が `Valid`→`AmbiguousAlignment` へ意味的に変わり同一性断言が崩れる | med | low | 候補保持により `position == 6952` 観測断言と locate==prepared 同一性は validity 含め存続(design §7 分析済み)。受理可否比較は「同じ分類に落ちる」形で存続。意味変化は変更記録へ |
| 6 | `min_agreeing_methods = 1` 下で非曖昧の単一投票が受理される(曖昧証拠の拒否権は投票除外のみ) | low | low | spec Req 4 末項の受諾済み意味論 — 設定の意図として記録 |
| 7 | T2 の golden/既存テスト追随漏れ | med | low | `cargo test -p al-locate` 全体を T2 の verification に含め、失敗は契約変更か回帰かを分類してから修正 |

### Rollback Strategy

- 対象: 全変更はコード・テスト・文書の追加/置換で不可逆なデータ
  変換なし。`method_version`/`algorithm_version` `"2"` は永続化
  意味論の区別子であり、pre-release 方針により既存成果物は
  再生成する(互換層なし)
- Trigger: T1 が eps 窓不存在を示した場合(実装中止)、T2/T3 の
  ゲート失敗で原因切り分けに時間を要する場合
- Procedure: 当該タスクのコミットを `git revert`。コミット境界は
  T1(特性計測テストのみ — 現行挙動を変えないため revert 不要で
  残置可)/T2(契約実装一式)/T3(corr0 置換 — 独立コミット)で
  分離する。T3 revert 時は残存許容コメントも復元される
- Verification: revert 後に `mise exec -- cargo test -p al-locate`
  (T3 revert 時は `pinned_periodic_everywhere_not_accepted_at_wrong_phase`
  が現行意味で pass)が green

## Task Breakdown

### T1: 特性計測テスト(コード変更前) — `docs/tasks/locate-argmax-tie-eps-characterization.md`

al-locate 側で受理系 fixture(golden: 非周期 control・
repeated source・loop material、`tests/locate.rs` 受理系 —
`polarity_inverted_source_is_still_located` :420 を含む)と
拒否系/混同リスク対象(golden: periodic rejection・
low-coherence・silence、`tests/prepared.rs` の厳密同点・
周期 fixture)のメソッド毎マージン `M − max_{i≠argmax}|corr[i]|`
分布を計測し、
al-math 側で `ncc`/`gcc_phat`/`spectral_envelope`
カーネルの実運用 FFT サイズ scalar↔dispatch 偏差を実測。
`TIE_REL_EPS`/`TIE_ABS_FLOOR` を確定し、窓不存在なら spec Req 7
エスカレーション。

### T2: argmax 曖昧性検出の実装 — `docs/tasks/locate-argmax-ambiguity-detection.md`

`argmax_tie_count` + `evidence_from_corr` 曖昧分岐(候補保持・
"indistinguishable" 文言)+ `evaluate` の投票除外・空投票優先・
reason 注記 + `merged_votes` タプルキー + `best_window_match`
ペア単位ガード + version `"2"` スイープ + ユニットテスト群 +
golden/prepared 追随。

### T3: spectral_envelope の dsp::corr0 移行 — `docs/tasks/locate-spectral-envelope-corr0.md`

`methods.rs:539-547` の逐次内積/ノルムを `dsp::corr0` へ置換し
残存許容コメントを除去。pin-periodic golden で厳密同点 →
構造的曖昧検出 → 拒否維持を検証(TASK-034EpPesSpbjbAQBsmjvJx の
残存許容解消)。

### T4: 計測・IR・closeout — `docs/tasks/locate-argmax-ambiguity-closeout.md`

回帰計測(bench・golden)、Implementation Record 確定(T1 実測・
eps 確定値・意図的契約変更の記録)、関連 doc 更新、最終 workspace
検証。

## Verification

| spec AC / 設計検証(要旨) | 検証方法 | 担当 |
|---|---|---|
| T1 計測が実装に先行し `rel_eps`/`abs_floor` が実測記録と共に確定 | 特性計測テスト追加 + IR 実測転記 + git 履歴(計測が実装に先行) | T1 |
| pin-periodic が不受理・`validity == "ambiguous_alignment"`・全4メソッド merged 証拠が `AmbiguousAlignment`+候補保持・full_ncc 候補 ≥2 かつ周期倍数・実行間/dispatch 間で決定的 | `cargo test -p al-locate --test golden` + 証拠レベル pin | T2 |
| pin-periodic-control が tolerance 64 で受理、repeated/loop ほか受理系 golden が受理維持・全メソッド merged 非曖昧(false positive なし) | golden 全件 + merged 証拠 pin | T2 |
| `pinned_low_coherence_no_inflated_lag` が不受理・`agreeing_methods < 2` 維持 | 既存 golden | T2 |
| ユニットテスト: 厳密同点・近同点・平坦 plateau が曖昧+候補保持、単一候補・意味ある分離・幅広平坦頭部の単峰が非曖昧、全証拠曖昧で合議 `AmbiguousAlignment`(LowCorrelation/InsufficientSupport に落ちない)、符号 ±1.0 同点が曖昧 | `cargo test -p al-locate` モジュール内テスト | T2 |
| reject_reason pin: 証拠 "indistinguishable"、空投票結果は "method(s) agree" を含まない専用文言、合意不足+曖昧除外時 "; N method(s) excluded as ambiguous" | golden pin + consensus ユニットテスト | T2 |
| `best_window_match` が全段で同点ペアを非寄与とし全ペア同点/閾値未達時のみ `None` | stage ユニットテスト + golden | T2 |
| `tests/prepared.rs` が新意味論で pass・`evaluate(.., "test", "2")` 記録、全 `method_version`/`algorithm_version` が `"2"` で永続化 | `cargo test -p al-locate --test prepared` + コード検査 | T2 |
| T3: corr0 置換後も周期 fixture の拒否が構造的に維持 | `cargo test -p al-locate --test golden pinned_periodic` + 残存許容コメント除去確認 | T3 |
| `cargo test -p al-locate`・`cargo test -p al-math` green、bench/回帰計測と IR 記録 | 各検証コマンド + IR 記録 | T1–T4, T4 確定 |

行動レベルの保証 intent は `docs/test-specs/locate-argmax-ambiguity.md`
(契約検証)と `docs/test-specs/locate-argmax-tie-eps-calibration.md`
(eps 較正の特性計測)に記録し、task 側は `verified-by` で参照する。

## Open Questions / Gaps

- `TIE_REL_EPS`/`TIE_ABS_FLOOR` の具体値は T1 実測で確定する
  (spec 候補レンジ: `rel_eps` 相対 1e-9..1e-6、`abs_floor` ~1e-9)。
  実測が窓不存在を示した場合のエスカレーション先は spec Req 7
  (E-3 / Option C)に規定済み — 本 plan はその分岐を含まない
- 判定反転の irreducible 不安定帯 `eps ± dispatch_deviation` は
  契約として正直に記述する(spec Req 6a)。帯幅の実測値は T1 の
  dispatch 偏差計測で確定する
- `docs/impl/ir/` の記録ファイル名・分割は T1 でドラフトを起こす
  際に既存 IR 規約(`docs/impl/ir/` 既存レコード)に倣う

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(T1→T2→T3→T4 の直列依存。
  T1 の計測結果が T2 の const 値を決めるため並列化しない)
- Independent work streams: なし(T1 内の al-locate 計測と al-math
  計測は並行可能だが同一タスク内の2面であり分離しない)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- design の §1-§8(判定式・分岐構造・分類優先・merged_votes キー・
  best_window_match ガード・version スイープ・T1 計測表・
  T2/T3 輪郭)を正本として転記済み。レビュアは (a) T1 の
  ゲート条件(`max(dispatch 偏差) << eps << min(マージン)`)
  と escalation 経路が spec Req 6-7 と一致すること、(b)
  Verification matrix が spec Acceptance Criteria を網羅して
  いること、(c) `merged_votes`/`best_window_match` の既知の
  非対称・クオーク(大きさ基準 vs 符号付き選択、候補空 0.0)が
  契約対象外として正しく除外されていることを確認すること
- plan の status は draft — レビュー承認後に approved へ進め、
  task の `--plan` ゲート(TASK-DOC-GATE-001)は承認後に有効化
  される。task 文書は plan への `implements`/`depends-on` を
  既に記録済み(front matter 作成時に plan が draft だったため
  スクリプトの gate を経由せず手動で記録)
