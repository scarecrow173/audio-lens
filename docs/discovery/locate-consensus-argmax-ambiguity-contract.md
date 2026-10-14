---
id: "DISC-034Fl0OeHprFoFHitMqBid"
type: "discovery"
status: "resolved"
title: "Locate consensus argmax ambiguity contract"
created: "2026-10-13"
updated: "2026-10-14"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/al-locate-vec-migration.md"
    - "docs/impl/ir/al-locate-dsp-re-export-vec-migration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/discovery/remaining-issues-roadmap.md"
  derived-by:
    - "docs/specs/locate-consensus-ambiguity-contract.md"
    - "docs/adr/locate-argmax-ambiguity-representation.md"
    - "docs/designs/locate-consensus-argmax-ambiguity.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Locate consensus argmax ambiguity contract

## Exploration Goal

Issue #59(Phase 0)。locate の合議が「各メソッドの argmax = 先頭候補」を
投票として扱う現状で、周期入力への拒否が**逐次 NCC の浮動小数点
丸め誤差による偶然のピーク散乱**に依存している。`dsp::corr0` など
決定的なカーネルへ置換すると厳密同点が生じ、stable sort により
argmax が最小 lag に倒れて複数メソッドが誤った位置で合意し、
偽受理が再現される(実コードの残存許容コメント
`methods.rs:539-543`、TASK-034EpPesSpbjbAQBsmjvJx / PR #49 で
置換が明示的に保留された経緯あり)。

本探索の問いは: 「位置が特定できる」の必要条件を、最大スコアだけでなく
**ピークの一意性・分離度**でどう契約化するか — 曖昧性をどこで検出し、
どう表現し、合議がどう扱うか。これは #42(FFT カーネル変更)・
#40(coarse-to-fine)など数値挙動を変える全作業の先行ゲート。

## Key Issues and Assumptions

コード監査で確認した事実(2026-10-13、main `705a37c` 時点):

### 現行メカニズム(confirmed)

- `peaks_from_correlation`(`methods.rs:198-253`):
  `|corr|` の局所最大を抽出(端点・単一要素含む)、背景は median |corr|
  (下限 1e-12)、`|score|` 降順の **stable sort**(total_cmp)で、
  **厳密同点は index 順を保持 → 最小 lag が勝つ**。`peak_ratio_12` は
  先頭候補にのみ `|top|/|second|` として記録される(Option、第2ピーク
  不在時は None)。
  - `is_peak` は `left <= v && right < v` の非対称条件で、隣接する
    等値 plateau は末尾1点のみピーク化。周期同点は谷を挟んだ**別々の
    局所最大**なので候補として残る。
- `evidence_from_corr`(`methods.rs:255-289`): 4分岐。
  ピークなし → `InsufficientSupport`(候補空)、非有限/非正 →
  `LowCorrelation`(候補空)、p2b 不足 → `LowCorrelation`(**候補は保持**)、
  以外 → `Valid`。**非 Valid validity + 候補保持の先例が既に存在する**。
- 全4メソッド(`full_ncc`/`band_ncc`/`gcc_phat`/`spectral_envelope`)が
  `evidence_from_corr` に集約される → ここに曖昧性検出を置けば
  全メソッド・全チャンネルペアに一様適用できる。
- `consensus::evaluate`(`consensus.rs:65-212`):
  - 投票フィルタは `validity != ComputationFailed` のみ →
    `AmbiguousAlignment` を含む他の非 Valid でも候補があれば投票する。
  - `votes.is_empty()` → `LowCorrelation`(Valid 証拠あり)または
    `InsufficientSupport` → **曖昧メソッドを全て投票除外すると
    `AmbiguousAlignment` にならず golden が壊れる**。空投票時の
    分類に曖昧性を考慮した分岐が必要。
  - `reject_reason` "only {n} method(s) agree (need {m})" は golden が
    文字列で pin している。
- `merged_votes`(`stage.rs:450-464`): メソッド毎に最强チャンネルペアを
  スコアで選択、validity は見ない → 曖昧マークは evidence と共に
  合議へ届く(構造上問題なし)。
- occurrence クラスタリング(`stage.rs:703-741`): `pair.evidence.validity
  == Validity::Valid` のみが位置を寄与 → メソッド証拠を非 Valid に
  マークすれば occurrence 側でも自動的に除外される(一貫)。
- `best_window_match`(`stage.rs:228-256`): ウィンドウアンカー/
  occurrence 検証の別 argmax 経路。期待位置近傍の狭い探索域内の同点は
  位置が既に拘束されているため害は小さい — 契約適用範囲は要 spec 判断。

### Validity と下流消費者(confirmed)

- `al_core::Validity`(`al-core/src/types.rs:69-92`)は 11 variant。
  `AmbiguousAlignment` は既存で "The alignment underpinning the
  measurement is ambiguous." — メソッド証拠の曖昧性に流用可能な意味論。
- `propagate_validity`(`al-analysis/records.rs:76-87`)は
  `AmbiguousAlignment | LowCorrelation → AmbiguousAlignment` に写すが、
  `_ => ComputationFailed` の catch-all があり、**新 variant を追加すると
  下流で暗黙に `ComputationFailed` へ劣化する**。追加するなら同関数の
  明示マッピングが必須。
- 永続化: result.json `{"result":…,"derived":…}` +
  evidence.json `{"evidence": pairs}`(per-pair 全保持)。
  `MethodEvidence.validity` は snake_case でシリアライズされる。
- al-viz `report.html`: `ev.validity`・`peak_ratio_12` を表示するのみ
  (ロジックなし) → 新 variant 名もそのまま描画される。安全。
- al-analysis 側は result レベルの `accepted`/`validity` のみ消費
  (`stage.rs:539`、`expensive/ir.rs:118`)。

### 重要な緊張: 正当な複数出現との区別(confirmed — 本探索の中心論点)

- `case_repeated_source`(3箇所埋込み・受理必須)と `case_loop_material`
  (等間隔3箇所・受理+occurrences 必須)は、**複数の ~1.0 スコアピークを
  持つ正当な入力**である。素朴な「上位Kピーク同率 → 曖昧」判定は
  これらを誤って曖昧扱いし、受理を壊す false positive を生む。
- 区別の手がかり:
  - 周期曖昧: 同点差は FP ノイズ級(逐次 ~1e-15〜1e-12、corr0 では厳密
    等値)、位置は信号周期の整数倍で dense に分布
  - 正当な複数出現: スコア差はノイズ床の差で **~1e-3 級と意味ある分離**、
    位置間隔は必ずしも等間隔でない(loop_material は偶発的に等間隔=4000)
  - → 「同点」を認める eps の較正が主たる識別子。eps は dispatch 間
    FP 差(実測 ~1e-13 級以下)より十分大きく、意味ある分離(~1e-3)より
    十分小さい必要がある。**位置間隔の周期性は補助信号になり得るが、
    loop_material の等間隔例が示すように単独では偽陽性を生む**。
- golden が pin する契約(`golden.rs`):
  - `pinned_periodic_everywhere_not_accepted_at_wrong_phase`:
    不受理 + `validity=="ambiguous_alignment"` + reject_reason 部分文字列
    "method(s) agree" + full_ncc 候補 ≥2 かつ全位置が周期倍数
    (候補 observability)+ 非周期対照は受理
  - `pinned_low_coherence_no_inflated_lag`: `agreeing_methods < 2`、
    envelope 以外の先頭スコア < 0.85

## Alternatives and Comparison

ダッシュボード採用案(B)を軸に、表現・検出位置・判定式の3軸で分解した
比較を記す(各軸は直交に選択可能)。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. argmax の決定的タイブレーク(同点なら最小 lag/prior 順で確定) | 差分最小・再現性改善 | 誤った候補を**自信付きで受理**する。構造問題の解消にならず dashboard でも非採用 | no |
| B. ピーク曖昧性を第一級出力化し曖昧メソッドを投票除外 | 誤受理を構造的に抑止、周期・ループ素材へ一般化、証跡(evidence/reject_reason)を保持 | 検出閾値・合議分岐・golden の契約更新が要る | yes |
| C. 周期モデル/自己相関で位相同値性を検証 | 「同一事象の位相」と「別出現」を意味的に最も正確に区別 | 計算量・閾値・モデルが増え調整コスト大。#59 スコープ過剰 | maybe(B で不足が判明した場合の強化予備) |

表現軸(B の内部選択):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| B-1. `MethodEvidence.validity = AmbiguousAlignment` 流用 | 既存 variant で意味論が一致、`propagate_validity` が既に正しく写す、occurrence 除外と一貫、シリアライズ語彙を変えない | 結果レベルの ambiguous とメソッドレベルを同一語で表す(意味は近いが層が違う) | yes |
| B-2. 新 Validity variant(例 `AmbiguousEvidence`)追加 | メソッド/結果の層を語彙で区別可能 | `propagate_validity` の catch-all で `ComputationFailed` へ暗黙劣化(同時修正必須)、永続化語彙が増える | maybe |
| B-3. `MethodEvidence` に別フラグ/reason フィールド追加 | validity 語彙を汚さない | 構造体・serde・消費者の全変更、表現ドリフト( validity と flag の二重表現) | no |

検出位置軸:

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| D-1. `evidence_from_corr` / `peaks_from_correlation` で検出 | 全4メソッド・全ペアに一様適用、候補保持の既存先例(LowCorrelation)と同型 | per-pair と merged の両方に効く点のみ注意(merged は clone なので自動伝播) | yes |
| D-2. consensus 側で検出 | メソッド証拠を変えない | メソッド毎の候補分布しか見えず判定材料は同じ、evidence への記録(provenance)がない、occurrence 側の Valid フィルタと不整合 | no |

判定式軸(詳細は spec/ADR で確定、T1 計測が前提):

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| E-1. `peak_ratio_12 ≈ 1.0`(上位2ピーク比のみ) | 既存フィールド・viz 表示済み | 周期同点は多数ピークで top-2 だけでは分布を表現しきれない、None 時の扱いが要る | maybe(必要条件として利用) |
| E-2. 上位候補の top |score| からの相対 eps 内カウント | 「argmax が統計的に一意でない」を直接表現、K と eps で粒度調整可能 | eps 較正が必須(T1 で実測) | yes |
| E-3. スコア同点 + 位置間隔の周期性/稠密性 | repeated/loop の等間隔偽陽性を追加抑制 | 間隔だけでは loop_material を誤判定し得る、判定が複合化 | maybe(E-2 との AND 候補) |

## Tentative Conclusions and Hypotheses

- **(confirmed)** 現行の周期拒否は全メソッドで偶然である。
  FFT 経路も lag 毎に丸め誤差が異なり近接同点を出すため argmax が散り、
  tolerance 256 内に 2 メソッド揃わず `AmbiguousAlignment` になる。
  逐次 envelope はノイズがやや大きいだけで本質は同じ。
- **(confirmed)** `corr0` 化で envelope の位相整列 lag への入力
  (32bin envelope)が bit-identical になり**厳密同点**が生じ、
  stable sort で argmax が最小整列 lag に固定される。
- **(hypothesis)** 方向は B: 曖昧性をメソッド証拠レベルで検出し、
  候補は保持したまま validity で投票から除外する。
  判定式の主軸は E-2(top |score| からの相対 eps 内の候補数/分布)、
  eps は T1 特性計測で「dispatch 間 FP 差の最大値より十分大きく、
  golden 受理系の最小ピーク分離より十分小さい」範囲として較正する。
- **(hypothesis)** 表現は B-1(`AmbiguousAlignment` 流用)が本命。
  `propagate_validity` の catch-all 危険を回避でき、既存の意味論・
  永続化語彙・occurrence 除外と整合する。
- **(confirmed)** 合議側は2箇所の変更が必要: (a) 投票フィルタを
  `ComputationFailed` のみから曖昧証拠も除外へ拡張、(b) 空投票/不足時の
  分岐が `AmbiguousAlignment` を返すよう分類規則の更新
  (現状は `LowCorrelation`/`InsufficientSupport` に落ちる)。
- **(hypothesis)** golden の `reject_reason` "method(s) agree" pin は、
  分類分岐を調整するか、より正確な文言へ**意図的に更新するか**の
  どちらかが要る — spec で決定し、偶然への依存でなく契約として残す。
- **(confirmed)** `corr0` 移行は契約確立後の T3 で実施し、
  「厳密同点 → 構造的曖昧検出 → 拒否維持」を検証することで本 issue の
  完了証拠とする(TASK-034EpPesSpbjbAQBsmjvJx の残存許容の解消)。

## Open Questions

- **Q1(eps 較正)**: 「同点」とみなす相対 eps の具体的値。T1 で
  全 golden fixture の `peak_ratio_12`/上位Kスコア差分布と
  dispatch(scalar vs SIMD)間偏差を実測し、
  `max(dispatch差) << eps << min(意味ある分離)` の範囲を定める。
  → spec で数値を確定。
- **Q2(正当な等価繰返し)**: 完全に同じ内容が完全に等しいスコアで
  複数出現する入力(例: ノイズ床なしの gain 1.0 多重埋込み)は、
  「位相曖昧」か「正当な複数出現」か。現行は argmax→occurrence 回収で
  受理される。score-tie だけでは区別不能なケースに対し、
  位置間隔の周期性(E-3)や出現回数の上限を加えるか、それとも
  「厳密同点の複数出現は曖昧として拒否」を契約として受け入れるか。
  → spec/ADR の最重要決定点。T1 で repeated/loop/periodic 各 fixture の
  実スコア差を測り、score-only で分離可能か確認する。
- **Q3(validity 表現)**: `AmbiguousAlignment` 流用(B-1)か新 variant
  (B-2)か。新 variant の場合 `propagate_validity` の明示マッピングが
  必須。 → ADR で確定。
- **Q4(合議分類)**: 曖昧証拠除外で投票が空/不足になった場合の
  結果分類(`AmbiguousAlignment` 固定でよいか)と
  `reject_reason` の文言(golden 互換性の取り扱い)。 → spec で確定。
- **Q5(best_window_match)**: アンカー/occurrence 検証の狭域 argmax にも
  同契約を適用するか、期待位置拘束下では対象外とするか。 → spec で範囲確定。
- **Q6(determinism)**: 同点判定が CPU dispatch(Scalar/V3-FMA 等)で
  不変であることの検証方法 — bit 一致でない値への比較なので
  eps 設計と T1 の dispatch 両経路計測で担保する形でよいか。 → test-spec で確定。

## Promotion Candidates

- [x] spec-doc needed: 曖昧性の定義(判定式・eps・候補保持)・合議の
  投票除外/分類規則・`reject_reason` 契約・golden 更新方針・
  `best_window_match` 適用範囲・AC(周期不受理/非周期受理/dispatch 不変)
- [x] adr-doc needed: 数値受理セマンティクスと証拠 validity 語彙の
  変更(表現 B-1 vs B-2)、検出位置(D-1)、任意タイブレークの禁止、
  corr0 移行のゲート条件。公開/永続化される `MethodEvidence` の
  意味論に触れるため ADR 必須

## Sources and Relations

- Issue: [#59 合議argmaxのFPノイズ依存を解消](https://github.com/scarecrow173/audio-lens/issues/59)
- Evidence: [PR #49 — corr0 置換の意図的見送り](https://github.com/scarecrow173/audio-lens/pull/49)
- 上位整理: `docs/discovery/remaining-issues-roadmap.md`(#59 を Phase 0、
  Option B「ピーク曖昧性を第一級出力化」採用、選択肢 A/C の記録)
- 残存許容の記録: `docs/tasks/al-locate-vec-migration.md`、
  `docs/impl/ir/al-locate-dsp-re-export-vec-migration.md`
  (envelope NCC を逐次のまま残した決定と理由)
- ダッシュボード原資料: `C:\Users\akiyoshi\Downloads\audio_lens_issue_dashboard.html`
  (issue #59 の decision/steps/tests/options)
- コード監査対象: `crates/al-locate/src/{methods,consensus,stage,candidate,residual}.rs`、
  `crates/al-core/src/types.rs`、`crates/al-analysis/src/records.rs`、
  `crates/al-viz/src/report.html`、`crates/al-locate/tests/golden.rs`
