---
id: "SPEC-034FlHW8GmjrKvTJlf0UWw"
type: "spec"
status: "approved"
title: "Locate 合議におけるピーク曖昧性契約"
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
    - "../adr/locate-argmax-ambiguity-representation.md"
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
---

# Locate 合議におけるピーク曖昧性契約

## What Should Be Built

locate の合議(consensus)において、「メソッド証拠の argmax が統計的に
一意でない」状態を**ピーク曖昧性(peak ambiguity)として第一級の
契約**にする。具体的には次のふるまいを契約として定める。

- メソッド証拠(`MethodEvidence`)が、較正された相対スコア
  イプシロン内で上位候補が識別不能である場合、非 Valid の曖昧状態
  としてマークされる。表現は既存 variant
  `Validity::AmbiguousAlignment` の `MethodEvidence.validity` への
  流用とする(表現選択は ADR ADR-034FlIKrgxUPy1AQk3WGlb で確定・
  記録済み)。
- 曖昧とマークされた証拠でも候補列は保持され、evidence の
  observability が失われない。
- 合議は曖昧証拠を投票から除外し、曖昧性が不受理の原因である
  結果は `AmbiguousAlignment` として分類される。
- argmax の決定的タイブレーク(最小 lag 等)を受理・confidence の
  根拠にすることは禁止する。
- 曖昧性の検出は CPU dispatch(scalar / SIMD)に対して不変である。

本 spec は WHAT(契約として何が保証されるか)を定める。検出位置・
判定式・分類優先順位・reject_reason 文言は契約として pin し、
epsilon の最終値のみ T1 特性計測を経て実装記録へ委ねる。

## Why Now

現状、周期入力(キャプチャ全体が同一パターンの繰り返し)への拒否は、
**各メソッドの argmax が浮動小数点丸め誤差で偶然散らばる**ことへの
依存にすぎない(探索 DISC-034Fl0OeHprFoFHitMqBid で確認済み)。
`spectral_envelope` の逐次 NCC を `dsp::corr0` のような決定的
カーネルへ置換すると、位相整列 lag への入力が bit-identical になり
**厳密同点**が生じる。`peaks_from_correlation` の stable sort は
厳密同点を index 順で解くため argmax が最小 lag に固定され、複数
メソッドが誤った位相で合意して**偽受理が再現される**
(`methods.rs:539-543` の残存許容コメント、PR #49 で置換が明示的に
保留された経緯)。

「位置が特定できる」正しさが FP ノイズに依存する状態は、FFT
カーネル変更(#42)や coarse-to-fine(#40)など数値挙動を変える
全ての作業の先行ゲートである。issue #59 は残課題ロードマップの
Phase 0 として、曖昧性を構造的に検出・拒否する契約を先に確立する。

## Users and Value

- `al-locate` 実装者・数値カーネル移行者: 周期入力の拒否が
  カーネルの丸め挙動ではなく契約で保証されるため、`corr0` 等への
  置換を安全に進められる
- 解析結果の消費者(al-analysis・al-viz・エクスポート利用者):
  不受理の理由が「スコア不足」と「argmax の曖昧さ」で区別され、
  `evidence` 内の候補列・`validity`・`reject_reason` から原因を
  辿れる
- レビュア・デバッグ利用者: 曖昧証拠でも候補が永続化されるため、
  「どの位相が競合したか」を evidence.json から検査できる

## Scope

### In Scope

- メソッド証拠レベルの曖昧性検出契約 — 検出位置は
  `evidence_from_corr`(`methods.rs:255`)内で raw 相関曲線に対する
  判定のみとし、`peaks_from_correlation` 自体は変更しない
  (全4メソッドが `evidence_from_corr` に集約されるため全
  メソッド・全チャンネルペアに一様適用される)
- `MethodEvidence` の曖昧性表現(`Validity::AmbiguousAlignment`
  流用)と候補保持の契約
- 合議の投票フィルタ拡張(曖昧証拠の除外)と、全メソッド曖昧を
  含む不受理時の結果分類規則
- `merged_votes`(`stage.rs:450-464`)のメソッド内チャンネルペア
  集約規則(非曖昧証拠の優先選択)
- `best_window_match`(`stage.rs:228-256`)のウィンドウアンカー
  argmax への同点ガード(同点時はアンカーを返さない)
- 曖昧経路の `reject_reason` 契約(明示的な文言として pin)
- dispatch 不変性を満たす相対 epsilon + 絶対床の較正要件(T1
  特性計測タスクで
  `max(dispatch 間偏差) << eps << min(意味ある分離)` の範囲を
  実測確定)
- 契約定着後の完了証明としての `spectral_envelope` 逐次 NCC の
  `dsp::corr0` 置換(周期拒否が構造的に維持されることの検証)
- 変更対象ファイル:
  `crates/al-locate/src/{methods,consensus,candidate,stage}.rs`、
  `crates/al-locate/tests/golden.rs`、必要に応じて
  `crates/al-core/src/types.rs` の doc コメント

### Out of Scope

- 周期性・自己相関モデルによる位相同値性の意味的検証
  (探索 Option C — 判定式で不足が判明した場合の強化予備として
  留保。Req 7 のエスカレーション経路を参照)
- `peaks_from_correlation` 自体の変更および、その他の呼出し経路
  (`propose_coarse` `methods.rs:74`、`refine_coarse` `methods.rs:109`)
  への曖昧性契約の適用 — 曖昧性判定は `evidence_from_corr` と
  `best_window_match` のみに適用し、coarse 探索の候補列挙には
  影響させない
- `al-math` の dispatch(`pulp`)override seam の追加 —
  `al-math` の `dispatch` は `pub(crate)` であり al-locate 側から
  到達不能。dispatch 不変性は eps 較正と al-math 自身の scalar↔
  SIMD 等価テストで担保する(Req 6)
- 合議クラスタリング(tolerance 内クラスタ・contested 判定等)の
  広範なリファクタ
- issue #52 / #42 の性能・カーネル作業本体(本契約はそれらの
  先行ゲートであり、性能作業そのものは含まない)
- `Validity` への新 variant 追加(ADR が B-1 流用を採用する
  方向。新 variant を採る場合でも `propagate_validity` の
  catch-all が `ComputationFailed` へ暗黙劣化させる問題への対処が
  必須であり、本 spec の受理基準は変わらない)
- `merged_votes` の既存の符号付きスコア選択クオーク
  (候補空ペアのスコア 0.0 が負スコアペアより優先される挙動) —
  既知の挙動として記録するのみで本契約では変更しない(観察事項)

## Requirements

1. **曖昧証拠のマーク(検出位置と判定式)**: 曖昧性の検出は
   `evidence_from_corr`(`methods.rs:255`)内で、**raw 相関曲線に
   対して**行わなければならない。`peaks_from_correlation` 内部では
   検出しない — 同関数には本契約の対象外となる呼出し
   (`propose_coarse` `methods.rs:74`、`refine_coarse` `methods.rs:109`、
   `best_window_match` `stage.rs:245`)があり、同関数は不変とする。
   判定式は次の通り pin する。
   - `M = max_i |corr[i]|` とし、同点集合を
     `T = { i : |corr[i]| >= M - tie_window }`、
     `tie_window = max(rel_eps · M, abs_floor)` と定義する。
     `|T| >= 2` ならば argmax は一意でなく、証拠は曖昧である。
   - 比較は `|corr[lag]|` の絶対値で行う(`|score|` 降順の既存
     ranking と一貫)。符号反転ペア(+1.0 / -1.0 — prepared.rs
     fixture が作る形状)は大きさで同点となり、曖昧と判定されるの
     が正しい挙動である。
   - 同点判定は全体最大値にアンカーされる(rank-1 anchored)。
     候補間の pairwise 比較ではない。
   - raw 曲線に対する判定であるため、分離した等値ピークと、
     `is_peak` の非対称条件(`left <= v && right < v`)が末尾1点へ
     潰す平坦 plateau の双方を包含する。`candidates` は
     observability のため `peaks_from_correlation` の出力を不変で
     使用する。
   - 分岐順序: 曖昧性判定は **Valid となるはずの証拠にのみ**
     適用する — no-peak・非有限/非正・p2b 不足の各 reject 分岐の
     **後**に評価され、弱さで既に棄却された曲線は
     `LowCorrelation` のままとし、曖昧へ格上げしない。
   表現は `MethodEvidence.validity = Validity::AmbiguousAlignment`
   とする(ADR ADR-034FlIKrgxUPy1AQk3WGlb で確定記録済み)。この
   判定は全 locate メソッド・全
   チャンネルペアに一様に適用されること。
2. **候補の保持**: 曖昧とマークされた証拠でも、抽出された候補列は
   `candidates` に保持されなければならない(observability — golden
   は full_ncc で周期倍数位置の候補が ≥2 観測できることを pin
   している)。非 Valid validity + 候補保持は既存の
   `LowCorrelation`(p2b 不足)分岐と同型の先例である。
3. **投票からの除外(フィルタ順序)**: 合議は曖昧とマークされた証拠を
   投票対象から除外しなければならない。`consensus::evaluate` の
   投票フィルタ順序を次の通り pin する: (1) validity 除外 —
   `ComputationFailed` **および** `AmbiguousAlignment` を除外;
   (2) メソッド毎の dedup(先着1件); (3) `candidates.first()` の
   存在; (4) `min_score` / `min_peak_to_background` 閾値。
   (観察事項: `LowCorrelation` で候補を保持した証拠は
   `min_peak_to_background` を下げれば投票し得るという既存の設定
   クオークがあるが、本契約はこれを変更しない — 対象外。)
4. **結果分類(優先順位)**: 曖昧性が不受理の原因である場合、結果の
   `validity` は `AmbiguousAlignment` でなければならない。分類の
   優先順位を次の通り pin する: 投票が空または不足の場合、**1つ
   以上のメソッド証拠が `AmbiguousAlignment` とマークされていれば**
   結果 validity は `AmbiguousAlignment` とする — この判定は既存の
   `any_valid → LowCorrelation` / `InsufficientSupport` 分岐の
   **前**に行う。根拠: 曖昧性による除外は「証拠が弱い」ではなく
   「曖昧である」と報告されなければならない。投票が非空で
   `agreeing < min_agreeing_methods` の場合(曖昧証拠が除外されて
   いる場合を含む)、既存の合意不足分岐(`AmbiguousAlignment` +
   "only N method(s) agree")はそのまま正しいが、曖昧証拠が除外
   されていた場合は `reject_reason` に曖昧性を示す注記を付加する
   (Req 10 参照)。なお `min_agreeing_methods = 1` の設定下でも
   曖昧証拠の除外は適用される — ただし min=1 では非曖昧な
   単一メソッドの投票が受理し得る。これは設定が表明した意図
   であり、本契約が保証する曖昧性の拒否権は証拠の投票除外を
   通じたものであって、他メソッドの投票そのものへの拒否権では
   ない。
5. **決定的タイブレークの禁止**: 同点を index 順・最小 lag 優先などで
   決定的に解くことを、受理や confidence の根拠にしてはならない。
   stable sort による順序付けは `candidates` 内の表示順序としてのみ
   許容される。
6. **dispatch 不変の epsilon**: 曖昧性検出は scalar と SIMD
   (V3-FMA 等)の全 dispatch 経路で同一の判定結果を返さなければ
   ならない。検証可能な形の契約は次の通りである。
   - (a) `rel_eps` は T1 特性計測で実測した **scalar↔dispatch 間
     最大偏差より厳密に大きい**値に較正しなければならない。これ
     により、irreducible な不安定帯 `eps ± dispatch_deviation` 内に
     入る入力を除き、dispatch 間での判定反転は起こり得ない — この
     不安定帯の存在は契約として正直に記述する。
   - (b) scalar↔SIMD カーネル等価性は `al-math` 自身のテスト
     (`correlation.rs`、`vec.rs`)で既に担保されており、本契約の
     検証対象は判定側の eps 較正のみである。al-locate テストから
     到達可能な dispatch override seam は存在しない(`al-math` の
     `dispatch` は `pub(crate)`)。seam の新設はスコープ外とする。
   - tie window は `max(rel_eps · |top_score|, abs_floor)` とし、
     スコアが 0 に近い曲線でも判定が発散しないよう絶対床を設ける。
     `abs_floor` の候補レンジは ~1e-9 とし、最終値は `rel_eps` と
     同様に T1 実測で確定して実装記録へ記録する。
   - `rel_eps` の初期候補レンジは**相対 1e-9..1e-6** とし、最終値は
     T1 特性計測(全 golden fixture の上位候補スコア差分布と
     scalar/SIMD 間偏差の実測)で確定し、実測データと共に実装記録へ
     記録する。
7. **退化ケースの意味論と eps 実現可能性**: 真にスコアが識別不能な
   繰り返し出現(同一内容が等しいスコアで複数回埋め込まれた入力)は、
   任意の位置を受理するのではなく**曖昧として報告する**ことを本
   契約の受け入れ済み意味論とする。受理系 golden fixture
   (`case_repeated_source`・`case_loop_material`)について、「ノイズ
   床差による ~1e-3 級の意味あるスコア分離を持つ」という記述は
   **仮説**であり、T1 特性計測が eps 確定の前に検証しなければ
   ならない。測定はメソッド毎に行うこと(`spectral_envelope` の
   正規化が最も脆弱点と見込まれる)。**緊急時方針**: T1 が
   `max(dispatch 間偏差) << eps << min(意味ある分離)` を満たす
   eps の存在を示さなかった場合、実装を進めずにエスカレーション
   する — 位置間隔の周期性/稠密性項を加えた検出への拡張(探索
   Option E-3)または Option C へのフォールバックを検討し、本 spec
   を改訂してから実装に進む。
8. **merged_votes の集約規則**: `merged_votes`(`stage.rs:450-464`)は
   メソッド内のチャンネルペアを統合する際、**非曖昧の証拠を曖昧な
   証拠より優先して選択**しなければならない — 曖昧ペアの先頭
   スコアの方が高くても `Valid` ペアが選ばれる。全ペアが曖昧の
   場合に限り、メソッドの merged 証拠は曖昧のままとなる。
   (既存の符号付きスコア選択クオーク — 候補空ペアのスコア 0.0 が
   負スコアペアより優先される挙動 — は既知の挙動として変更
   しない。対象外。)
9. **best_window_match の同点ガード**: `best_window_match`
   (`stage.rs:228-256`)は、評価した相関曲線の同点集合(Req 1 と
   同一基準)が ≥2 要素を持つ場合、`None` を返し(アンカーを寄与
   しない)なければならない — 同点のウィンドウマッチは位置の証拠
   ではなく、アンカー層にも同一契約を適用する。
   `window_anchors_in` は narrow → ×8 slack → 全 `[lo,hi)` 範囲の
   3段で探索する(`stage.rs:304-313`)が、第3段は事実上無拘束の
   argmax であり同点が害を生む。ガードは評価される曲線を持つ段の
   いずれにも適用する(narrow-first 段構成は最適化の詳細であり
   契約は段構造に依存しない)。
10. **reject_reason 契約**: 曖昧とマークされた `MethodEvidence` は、
    固定マーカー句 `"indistinguishable"` を含む `reject_reason` を
    持たなければならない — 推奨文言:
    `"top correlation peaks indistinguishable within tolerance (K positions)"`。
    evidence.json の消費者が、曖昧性と他の非 Valid 原因を機械的に
    区別できなければならない。結果レベルでは、曖昧証拠により投票
    が空になった分岐は独自の pin 文言を持つ — 推奨文言:
    `"all voting methods produced ambiguous (tied) peaks"`。"method(s)
    agree" を含んではならない(golden
    `pinned_periodic_everywhere_not_accepted_at_wrong_phase` の
    reject_reason 断言はこの新文言へ更新する — 意図的な契約変更と
    して記録する)。合意不足分岐("method(s) agree")は、曖昧証拠が
    除外されていた場合に限り `"; N method(s) excluded as ambiguous"`
    のような曖昧性注記を末尾に付加する。
11. **バージョン方針**: 永続化される意味論が変わるため、全メソッドの
    `method_version` と合議の `algorithm_version` を `"2"` へ
    バンプする。pre-release 方針により既存成果物は再生成するが、
    バージョンは再生成レコードの区別のために付ける。

## Acceptance Criteria

AC の assertion 粒度は、特に断りのない限り結果レベル
(`accepted` / `validity` / `agreeing_methods` / `reject_reason`)と
する。証拠レベルの pin が必要なものは個別に明記する。

- [ ] 周期 fixture(`pin-periodic`)が受理されず、結果の
  `validity == "ambiguous_alignment"`、full_ncc の `candidates` が
  ≥2 件かつ全位置が周期の整数倍として観測可能である。かつ同
  fixture の判定が実行間および CPU dispatch(scalar / SIMD)間で
  決定的である
- [ ] **証拠レベル pin**: 周期 fixture では全4メソッドの merged
  証拠が `AmbiguousAlignment` とマークされ、かつ `candidates` が
  保持されていること
- [ ] 非周期埋め込み対照(`pin-periodic-control`)が正しい位置で
  受理される(golden 既存 tolerance 64)
- [ ] `case_repeated_source`、`case_loop_material`、およびその他の
  受理系 golden が全て受理を維持する(曖昧性の false positive
  なし)。受理系 fixture では**全メソッドの merged 証拠が非曖昧の
  まま**であること — ペアレベルで曖昧が生じても `merged_votes`
  が非曖昧ペアへ解決する限り AC 違反とはしない
- [ ] `pinned_low_coherence_no_inflated_lag` が不受理を維持し、
  `agreeing_methods < 2` が保たれる
- [ ] 新規ユニットテスト: 厳密同点および近同点の相関曲線を与えた
  証拠が曖昧としてマークされ、かつ候補が保持されること(平坦
  plateau の同点を含む)
- [ ] 新規ユニットテスト: 全証拠が曖昧の入力に対し合議が
  `AmbiguousAlignment` を返すこと(`LowCorrelation` /
  `InsufficientSupport` に落ちない — Req 4 の優先順位)
- [ ] 新規ユニットテスト: 単一候補のみの曲線・上位候補が意味ある
  分離を持つ曲線が曖昧と誤判定されないこと
- [ ] 曖昧経路の `reject_reason` が不受理原因として曖昧性を明示する
  文言で pin されること: 証拠レベルはマーカー句
  `"indistinguishable"` を含み、投票空・曖昧起因の結果レベルは
  "method(s) agree" を含まない専用文言(推奨:
  `"all voting methods produced ambiguous (tied) peaks"`)とする。
  `pinned_periodic_everywhere_not_accepted_at_wrong_phase` の
  reject_reason 断言は新文言への更新を意図的な契約変更として
  行い、決定した文言と理由を変更記録に残す
- [ ] 完了証明(移行ステップ): 契約定着後に `spectral_envelope` の
  逐次 NCC を `dsp::corr0` へ置換しても、周期 fixture の拒否が
  構造的に維持されること(厳密同点 → 曖昧検出 → 拒否、であり
  argmax の偶然散乱への依存が解消されていること)

## Deferred Design Concerns

- Option C(周期性・自己相関モデルによる位相同値性の意味的検証):
  判定式(E-2 型の相対 epsilon)では「同一事象の位相」と「別出現」
  を意味論的に完全には区別しない。T1 が eps 実現可能範囲を示さ
  なかった場合のエスカレーション先(Req 7: E-3 拡張または
  Option C フォールバック)として留保する。無視した場合のリスクは
  等スコア多重埋め込みの受理不能(本契約では曖昧報告を受諾済み)

## Sources and Relations

- Issue: [#59 合議argmaxのFPノイズ依存を解消](https://github.com/scarecrow173/audio-lens/issues/59)
  (残課題ロードマップ Phase 0)
- 探索: `docs/discovery/locate-consensus-argmax-ambiguity-contract.md`
  (DISC-034Fl0OeHprFoFHitMqBid — 確認済みコード事実、代替案
  A/B-1..B-3/C/D-1..D-2/E-1..E-3、未決問 Q1-Q6 の出典。本 spec は
  B・B-1・D-1・E-2 の方向を契約として確定する)
- 証拠: [PR #49 — corr0 置換の意図的見送り](https://github.com/scarecrow173/audio-lens/pull/49)
- 残存許容の記録: `docs/tasks/al-locate-vec-migration.md`、
  `docs/impl/ir/al-locate-dsp-re-export-vec-migration.md`
  (`methods.rs:539-543` の逐次維持決定 — 本契約と corr0 移行
  AC により解消される)
- 表現選択(B-1 `AmbiguousAlignment` 流用 vs B-2 新 variant、検出
  位置、タイブレーク禁止)は ADR
  `docs/adr/locate-argmax-ambiguity-representation.md`
  (ADR-034FlIKrgxUPy1AQk3WGlb)で確定・記録済み
