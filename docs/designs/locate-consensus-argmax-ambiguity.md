---
id: "DESIGN-034Fn25qh0mE8cz5ymYKfD"
type: "design"
status: "approved"
title: "合議 argmax 曖昧性検出の実装設計"
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
  implements:
    - "../specs/locate-consensus-ambiguity-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/multi-method-locate-consensus.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/locate-consensus-ambiguity-contract.md"
    - "../adr/locate-argmax-ambiguity-representation.md"
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

# 合議 argmax 曖昧性検出の実装設計

## Context

issue #59(残課題ロードマップ Phase 0)。locate の合議は各メソッド証拠の
argmax(先頭候補)を投票として扱うが、周期入力への拒否が**逐次 NCC の
浮動小数点丸め誤差による偶然のピーク散乱**に依存していることが
DISC-034Fl0OeHprFoFHitMqBid のコード監査で確認済み。決定的カーネル
(`dsp::corr0`)へ置換すると厳密同点が生じ、`peaks_from_correlation` の
stable sort が argmax を最小 lag に固定して偽受理が再現される
(`methods.rs:539-543` の残存許容コメント、PR #49 で置換保留)。

承認済み spec(SPEC-034FlHW8GmjrKvTJlf0UWw)と受諾済み ADR
(ADR-034FlIKrgxUPy1AQk3WGlb)が契約を確定している: 表現 B-1
(`Validity::AmbiguousAlignment` 流用)、検出位置 D-1
(`evidence_from_corr` の raw 相関曲線)、判定式 E-2(rank-1 アンカーの
相対 eps 窓同点集合)。本設計はその契約を、既存コードの実関数・実分岐
構造へ落とす具体化である。

現行の関連構造(2026-10-14 時点の実コード):

- `peaks_from_correlation`(`methods.rs:198-253`): `|corr|` の局所最大を
  抽出し `|score|` 降順 stable sort。`is_peak` は
  `left <= v && right < v` の非対称条件(`methods.rs:207-212`)で、
  隣接する等値 plateau は末尾1点のみピーク化される
- `evidence_from_corr`(`methods.rs:255-289`): 全4メソッドの共有漏斗。
  ピークなし → `InsufficientSupport`(候補空)、非有限/非正 →
  `LowCorrelation`(候補空)、p2b 不足 → `LowCorrelation`(**候補保持**)、
  以外 → `Valid`
- `consensus::evaluate`(`consensus.rs:65-212`): 投票フィルタは
  `validity != ComputationFailed` のみ(`consensus.rs:81`)。空投票は
  `any_valid` で `LowCorrelation` / `InsufficientSupport` に分岐
  (`consensus.rs:134-149`)。合意不足は `AmbiguousAlignment` +
  "only N method(s) agree"(`consensus.rs:173-186`)
- `merged_votes`(`stage.rs:450-464`): メソッド毎に先頭候補の
  **符号付き**スコア最大のペアを選択
- `best_window_match`(`stage.rs:228-256`): 全チャンネルペアの
  `peaks_from_correlation(&corr, 1)` から最大スコアを選ぶ別 argmax
  経路。`window_anchors_in` の narrow → ×8 slack → 全 `[lo,hi)` の
  3段探索(`stage.rs:304-313`)と occurrence 検証(`stage.rs:811`)から
  呼ばれる
- `Validity::AmbiguousAlignment` は既存 variant
  (`al-core/src/types.rs:80-81`)。occurrence クラスタリング
  (`stage.rs:703-741`)は `Validity::Valid` のみを位置寄与とするため、
  証拠を非 Valid にマークすれば自動的に除外される

## Scope

### In Scope

- `methods.rs`: raw 相関曲線に対する同点集合判定の共有ヘルパと
  `evidence_from_corr` への曖昧性分岐追加、`rel_eps`/`abs_floor`
  較正定数、全4メソッドの `version()` を `"2"` へバンプ
- `consensus.rs`: 投票フィルタへの `AmbiguousAlignment` 除外追加、
  空投票時の分類優先順位、曖昧除外注記の `reject_reason` 付加
- `stage.rs`: `merged_votes` の非曖昧優先選択規則、
  `best_window_match` の同点ガード、`algorithm_version` `"1"`→`"2"`
  の呼出し更新
- `golden.rs`: 曖昧経路の `reject_reason` 契約 pin 更新と証拠レベル
  pin の追加
- `tests/prepared.rs`: `evidence_from_corr` 経路の意味論シフト
  (厳密同点・近同点の証拠が `AmbiguousAlignment` + 候補保持へ変わる)
  への追随と `evaluate(.., "test", "1")` の記録版更新(§6・§7 参照)
- `al-math` の既存 `#[cfg(test)]` モジュール内への特性計測テスト追加
  — `dispatch` は `pub(crate)`(`lib.rs:28`)で `pulp::Scalar` は
  crate 内でのみ到達可能なため、T1 の scalar↔dispatch 偏差実測は
  al-math 側へ置く。dispatch override seam の新設は引き続き対象外
- T1 特性計測テスト(eps 実現可能性の実測)、T2 実装、T3
  `spectral_envelope` 逐次 NCC の `dsp::corr0` 移行(完了証明)

### Out of Scope

spec の Out of Scope をそのまま引き継ぐ:

- `peaks_from_correlation` 自体の変更、および契約対象外の呼出し
  (`propose_coarse` `methods.rs:74`、`refine_coarse` `methods.rs:109`)
  への適用
- 周期性・自己相関モデルによる位相同値性の意味的検証(Option C)、
  位置間隔の周期性項(E-3) — T1 が eps 実現不能を示した場合の
  エスカレーション先として留保
- `al-math` の dispatch override seam 新設(`dispatch` は
  `pub(crate)` で al-locate から到達不能)
- `Validity` 新 variant、`propagate_validity` の変更
- 合議クラスタリング(tolerance クラスタ・contested 判定)のリファクタ
- `merged_votes` の既存の符号付きスコア選択クオーク(候補空ペアの
  0.0 が負スコアペアに勝つ挙動)— 既知の挙動として記録のみ
- issue #42 / #52 の性能・カーネル作業本体

## Components and Boundaries

| コンポーネント | 責務 | 変更 |
| --- | --- | --- |
| `methods.rs` | 同点集合判定の**唯一の実装**(`argmax_tie_count`)+ `evidence_from_corr` の曖昧分岐 + 較正定数 + バージョン | 改修 |
| `consensus.rs` | 投票フィルタ拡張 + 空投票分類の優先順位 + reject_reason 文言 | 改修 |
| `stage.rs` | `merged_votes` 非曖昧優先 + `best_window_match` 同点ガード + `algorithm_version` 更新 | 改修 |
| `candidate.rs` | `MethodEvidence` 構造は不変(非 Valid + 候補保持は既存先例と同型) | 不変 |
| `al-core/src/types.rs` | `AmbiguousAlignment` 流用。variant 追加なし。必要に応じ doc コメントでメソッド証拠用途を明記 | doc のみ |
| `golden.rs` | 契約 pin 更新 | 改修 |
| `tests/prepared.rs` | prepared 経路証拠の意味論シフト(厳密同点 → `AmbiguousAlignment`)への追随 | 改修 |
| `al-math` テスト | `ncc`/`gcc_phat`/`spectral_envelope` の実運用サイズ scalar↔dispatch 偏差特性計測(T1) | テスト追加 |

判定規則(`argmax_tie_count`)を `methods.rs` に集約し `pub(crate)` で
公開するのは、同じ「同点」の意味論を `evidence_from_corr` と
`best_window_match` が共有するためである — 表現ドリフト防止のため
判定式は一箇所で定義する(ADR の D-1 思想に一致)。

## Data and Control Flow

### 1. 同点集合判定(raw 相関曲線)

`methods.rs` に共有ヘルパを追加する:

```rust
/// 同点とみなす相対イプシロン。T1 特性計測で較正(最終値は実測記録)。
const TIE_REL_EPS: f64 = /* T1 で確定: 候補レンジ 1e-9..1e-6 */;
/// スコア 0 近傍で判定が発散しないための絶対床(候補 ~1e-9)。
const TIE_ABS_FLOOR: f64 = /* T1 で確定 */;

/// Rank-1 anchored の同点集合 |T| を返す。
/// T = { i : |corr[i]| >= M - tie_window },
/// tie_window = max(TIE_REL_EPS * M, TIE_ABS_FLOOR)、M = max_i |corr[i]|
pub(crate) fn argmax_tie_count(corr: &[f64]) -> usize {
    let m = corr
        .iter()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    let window = (TIE_REL_EPS * m).max(TIE_ABS_FLOOR);
    corr.iter()
        .filter(|v| v.abs() >= m - window)
        .count()
}
```

判定規則(spec Req 1 をそのまま実装):

- `M = max_i |corr[i]|`、`T = { i : |corr[i]| >= M - tie_window }`、
  `tie_window = max(rel_eps · M, abs_floor)`。`|T| >= 2` ならば
  argmax は一意でなく曖昧
- **比較は `|corr[i]|` の大きさで行う**(符号反転ペア +1.0 / −1.0 は
  大きさで同点 → 曖昧が正しい。既存の `|score|` 降順 ranking と一貫)
- 全体最大値アンカー(rank-1 anchored)であって候補間 pairwise ではない
- **`|T|` はサンプル(index)数として数える**。連続する等値 plateau
  は run として1位置に潰さない — spec の判定式は index 集合の基数を
  pin しており、plateau run を1位置に潰すと「平坦 plateau の同点が
  曖昧とマークされる」受理基準(単一 plateau のみの曲線)と矛盾する。
  かつ plateau 内ではどのサンプルも argmax 足り得るため、plateau
  そのものが位置の非一意性である。cluster 単位の計数は代替として
  検討し、上記の契約不一致を理由に棄却する
- raw 曲線への判定なので、谷を挟んだ分離同点ピークと `is_peak` の
  非対称条件が末尾1点に潰す平坦 plateau の双方を包含する
  (peak-list 検査では plateau 同点を見落とす)
- NaN は全比較で false となり T に含まれない。±inf は必ず argmax
  になるため top 候補の非有限チェック(既存分岐)が先に捕捉し、
  本判定には到達しない

### 2. `evidence_from_corr` の曖昧分岐

分岐順序は spec Req 1 の pin 通り、**Valid となるはずの証拠にのみ**
適用する — 既存の3分岐(no-peak → `InsufficientSupport`、非有限/非正
→ `LowCorrelation`、p2b 不足 → `LowCorrelation` 候補保持)の**後**で
評価し、弱さで棄却済みの曲線は `LowCorrelation` のままとし曖昧へ
格上げしない。構造は最終 `Some(_)` アームを次に置き換える:

```rust
Some(_) => {
    let tied = argmax_tie_count(corr);
    if tied >= 2 {
        MethodEvidence {
            method: method.into(),
            method_version: version.into(),
            candidates, // peaks_from_correlation の出力を不変で保持
            validity: Validity::AmbiguousAlignment,
            reject_reason: Some(format!(
                "top correlation peaks indistinguishable within tolerance ({tied} positions)"
            )),
        }
    } else {
        MethodEvidence { /* 従来通り Valid */ }
    }
}
```

- `candidates` は observability のため `peaks_from_correlation` の出力を
  不変で保持する(非 Valid + 候補保持は p2b 不足分岐の既存先例と同型、
  `methods.rs:272-280`)
- `reject_reason` は固定マーカー句 `"indistinguishable"` を含む
  (spec Req 10)。evidence.json の消費者が曖昧性を機械的に識別できる。
  `({tied} positions)` の数値は判定式通りの**サンプル(index)数**を
  報告する — 幅3の平坦 plateau は "3 positions" と表示される。
  cluster 単位ではない点は文言上の近似として受け入れ、記録に留める
- `M` は top 候補の `|score|` に一致する(グローバル最大は必ず plateau
  末尾のピークとして候補に含まれる)が、規則の単一定義を優先して
  ヘルパ内で曲線から再計算する — コストは O(n) で NCC 本体に比し
  無視できる
- 全4メソッド(`full_ncc` / `band_ncc` / `gcc_phat` /
  `spectral_envelope`)がこの漏斗を通るため、全メソッド・全
  チャンネルペアに一様適用される。`band_ncc` は累積曲線 `acc` への
  適用、`spectral_envelope` は hop 疎化曲線への適用となる。hop
  非評価 lag は 0.0 のまま残り、通常は `tie_window` を下回って T
  に入らない — ただし `M ≲ TIE_ABS_FLOOR` の低スコア曲線では
  `M - tie_window <= 0` となり未評価 lag の 0 が T に入り、曲線は
  曖昧とマークされ得る。これは無害: そのような曲線は位置の証拠を
  持たず(top |score| が投票閾値 `min_score` に到底届かず投票しない)、
  曖昧マークの有無で受理・投票結果は変わらない
- `peaks_from_correlation` 自体は不変 — `propose_coarse` /
  `refine_coarse` の coarse 候補列挙経路には契約を適用しない

### 3. `consensus::evaluate` の変更

投票フィルタの順序は spec Req 3 の pin 通り (1) validity 除外 →
(2) メソッド毎 dedup(先着1件) → (3) `candidates.first()` 存在 →
(4) `min_score`/`min_peak_to_background` 閾値。現行パイプラインが
既にこの順序なので、(1) の述語のみ拡張する:

```rust
let n_ambiguous = evidence
    .iter()
    .filter(|e| e.validity == Validity::AmbiguousAlignment)
    .count();
let votes: Vec<u64> = evidence
    .iter()
    .filter(|e| {
        !matches!(
            e.validity,
            Validity::ComputationFailed | Validity::AmbiguousAlignment
        )
    })
    .filter_map(/* メソッド dedup + candidates.first(): 不変 */)
    .filter(/* min_score / min_peak_to_background: 不変 */)
    .map(|c| c.position)
    .collect();
```

分類の優先順位(spec Req 4)は次の通り実装する:

1. `votes.is_empty()` かつ `n_ambiguous >= 1` →
   `validity = AmbiguousAlignment`、`reject_reason =
   "all voting methods produced ambiguous (tied) peaks"`(pin 文言。
   "method(s) agree" を含まない)。この判定は既存の
   `any_valid → LowCorrelation` / `InsufficientSupport` 分岐の**前**
   に置く — 曖昧性による除外は「証拠が弱い」ではなく「曖昧である」と
   報告する。副次フィールドは既存の空投票分岐と同じ値を再利用する
   (`consensus.rs:136-148` の `result(false, 0, 0, None, Confidence::Low, ..)`
   と同形): `accepted = false`、`agreeing_methods = 0`、
   `position_spread = 0`、`position = None`、`Confidence::Low`。
   なお文言の "all" は混在ケース(曖昧証拠と `InsufficientSupport`
   等が混ざる空投票)でも発火し得るため厳密には言い過ぎである —
   spec pin 文言のまま維持し、caveat として記録のみ行う
2. `votes.is_empty()` かつ曖昧証拠なし → 既存分岐を不変で維持
3. 投票非空かつ `agreeing < min_agreeing_methods` → 既存の
   `AmbiguousAlignment` + "only N method(s) agree (need M)" を維持し、
   `n_ambiguous >= 1` のときのみ末尾へ
   `"; {n_ambiguous} method(s) excluded as ambiguous"` を付加する
   (spec Req 10)
4. contested 分岐・受理分岐は不変

曖昧証拠の集計は `evaluate` に渡される merged 証拠列に対して行う
(per-pair ではなくメソッド merged 後)。`min_agreeing_methods = 1` の
設定下でも曖昧証拠の除外は適用されるが、非曖昧な単一メソッドの投票は
受理し得る — これは設定が表明した意図であり、契約が保証する拒否権は
曖昧証拠の投票除外を通じたものに限定される(spec Req 4 末項)。

`consensus.rs` モジュール内テストの `evaluate(.., "locate", "1")` 呼出し
と `tests/prepared.rs:135` の `al_locate::evaluate(.., "test", "1")`
呼出しは記録用引数であり verbatim に保持されるだけだが、一貫性のため
実装時に `"2"` へ更新する(§6 のスイープ対象)。

### 4. `merged_votes` の非曖昧優先選択

メソッド内のチャンネルペア統合(`stage.rs:450-464`)で、**非曖昧の証拠を
曖昧な証拠より優先して選択**する(spec Req 8)。比較キーをタプル化する:

```rust
let key = |e: &MethodEvidence| {
    (
        e.validity != Validity::AmbiguousAlignment,
        e.candidates.first().map(|c| c.score).unwrap_or(0.0),
    )
};
// merged.last_mut() と同メソッドなら key(pair) > key(m) で置き換え
```

- 曖昧ペアの先頭スコアが高くても非曖昧ペアが選ばれる
- **全ペアが曖昧の場合に限り** merged 証拠は曖昧のままとなる
  (spec 文言通り、`AmbiguousAlignment` 以外の非 Valid — 例えば
  `InsufficientSupport` で候補空のペア — も「非曖昧」側に含まれる。
  この場合 merged 証拠は投票も曖昧マーカーも持たないが、per-pair の
  `evidence.json` に全ペアの詳細が残るため provenance は失われない)
- 同一クラス内の選択は既存の符号付きスコア比較をそのまま維持する
  (候補空=0.0 が負スコアに勝つクオークは spec 対象外の既知挙動)

### 5. `best_window_match` の同点ガード

ウィンドウアンカー argmax(`stage.rs:228-256`)にも同一契約を適用する
(spec Req 9)。チャンネルペア毎の `ncc` 曲線評価後、ピーク抽出の前に
同点集合を検査し、`|T| >= 2` のペアは候補を寄与しない:

```rust
let corr = ncc(template, &cap_ch[lo..hi], hi - lo - window);
if argmax_tie_count(&corr) >= 2 {
    continue; // 同点のウィンドウマッチは位置の証拠ではない
}
```

- ガードはペア単位 — これは**意図的な粒度の pin** である: 同点の
  ペアはアンカーに寄与せず(`continue`)、`None` を返すのは**全ペアが
  同点または閾値未達の場合に限る**。あるペアの曲線が同点でも、他
  ペアの一意なピークはなお有効な位置証拠である(spec Req 9 にも
  同じ粒度を pin する)
- narrow → ×8 slack → 全 `[lo,hi)` の3段すべてが `best_window_match`
  を通るため、段構造に依存せず全段へ適用される(第3段は事実上
  無拘束の argmax であり、同点が最も害を生む)
- **受け入れるコスト**: 第3段はキャプチャ全域を探索するため、
  ループ内部や周期的なソースウィンドウでは全ペアが同点となって
  そのウィンドウのアンカーが全滅し得る。全ウィンドウが脱落すると
  `stage.rs:849-860` の occurrence 毎アンカーへのフォールバックに
  落ち、time_map が退化する(ウィンドウ内のドリフト・ジッタ観測を
  失う)。「同点 = 位置の証拠なし」は契約の意図通りだが、この退化
  経路の存在は受諾事項として明記する
- `peaks_from_correlation(&corr, 1)` の top-1 抽出と閾値評価は不変。
  ペア間の最良選択は既存の符号付き `c.score > b.score` /
  `min_score` 比較を維持する — 同点判定が `|corr|` 大きさ基準で
  あるのに対し最終選択が符号付きスコア基準という非対称は既存の
  挙動であり、本契約は変更しない(観察事項として記録)

### 6. バージョンバンプ(spec Req 11)

永続化される意味論が変わるため:

- 全4メソッドの `version()`: `"1"` → `"2"`
  (`methods.rs` の `FullNcc`/`BandNcc`/`GccPhat`/`SpectralEnvelope` の
  各 impl)
- 合議の `algorithm_version`: `"1"` → `"2"`。呼出し箇所は
  `stage.rs:172`(`marked_result`)、`stage.rs:671`(主 evaluate 呼出し)、
  `stage.rs:1037`(residual child evaluate 呼出し)の3箇所。テスト側の
  記録用引数も同じスイープで更新する: `consensus.rs` モジュール内
  テストの `evaluate(.., "locate", "1")`、および
  `tests/prepared.rs:135` の `al_locate::evaluate(.., "test", "1")`
- `residual.rs:321` の `algorithm_version: "0"` は `#[cfg(test)]` 内の
  stub `LocateResult`(`algorithm_id: "stub"`、`method_version: "0"`)
  の記録用フィールドであり、永続化される residual 子結果の版は
  `stage.rs:1037` の `evaluate` 呼出しが決める — **変更しない**
- pre-release 方針により既存成果物は再生成する。バージョンは再生成
  レコードの区別のためのみに付ける

### 7. T1 特性計測テスト(実装に先行)

spec Req 6-7 に従い、`rel_eps`/`abs_floor` の実現可能レンジを実装前に
実測で確定する。T1 は計測専用テスト(golden および `tests/locate.rs`
の受理系 fixture を走らせて分布を記録する characterization test)
として追加し、実装ゲートとする:

| 計測 | 目的 |
| --- | --- |
| 各受理系 fixture のメソッド毎に `M − max_{i≠argmax}\|corr[i]\|`(rank-1 の大きさと、全サンプル中の第二順位大きさの差)の分布 | 判定式 `T = { i : \|corr[i]\| >= M − tie_window }` は**全サンプル**を走査するため、拘束するマージンは「rank-1 と次位の別位置ピークの分離」ではなく第二順位の大きさとの差 — argmax ピーク自身のショルダー(隣接サンプル)を含む。`min(意味ある分離)` はこのマージンの fixture 群下限として実測する |
| ショルダーデルタ曝露の確認 | 帯域制限ソースでは argmax 直近のショルダーが極めて浅くなり得る。推定式 `1−cos(2π·fc/fs)` で、`golden.rs` `case_long_source_linear_drift`(RATE=48kHz、fc=1kHz の Biquad lowpass ノイズ、`golden.rs:393-401`)は実効帯域を考慮して ~8.6e-4 級、~35Hz 帯域制限のソースでは ~1e-6 級まで浅くなり、`rel_eps` 候補レンジ(1e-9..1e-6)に入り得る。eps の実質上限は fixture 群の最小ショルダーデルタであり、T1 で数値を確定する |
| `spectral_envelope` の慢性曖昧リスク | このメソッドは 32-bin log 振幅包絡の NCC であり、ホワイトノイズ・チャープ・広帯域 SFX では殆どの lag で包絡がほぼ同一 → \|T\| が大きく `AmbiguousAlignment` となり、実世界の広い入力クラスでメソッドが暗黙に投票を止め得る。fixture 群には非 golden の受理系テストも含める — 特に `tests/locate.rs` の `different_rates_locate_without_false_drift`(広帯域チャープ、~line 488)と `repeated_source_reports_second_occurrence`(ノイズ埋込み、~line 440)。T1 が noise 系入力での慢性曖昧を示した場合、eps の再較正ではなく spec Req 7 のエスカレーション経路を取り、spec AC「受理系 fixture では全メソッドの merged 証拠が非曖昧」の再検討を含めて spec 改訂する |
| 周期 fixture での同点構造 | 厳密同点・近同点の実形(分離ピーク vs plateau)の確認 |
| dispatch 間偏差の上限 | **al-math の `#[cfg(test)]` 内に characterization test を追加して実測する** — `dispatch` は `pub(crate)`(`lib.rs:28`)で `Correlation`/`Envelope` カーネルは非公開だが、`pulp::Scalar` は al-math 内テストから到達可能(テスト追加はスコープ内、dispatch override seam 新設は対象外)。`ncc`/`gcc_phat`/`spectral_envelope` の scalar↔dispatch 偏差を**実運用 FFT サイズ**で測る(本番 NCC は `(signal.len()+template.len()-1).next_power_of_two()` の RealFFT で、golden 規模では ~64k-point)。既存の等価アサーションは小入力中心で実運用サイズを直接カバーしない: `correlation.rs:193` は絶対 `1e-10` だが len ≤ 127(信号 ≤ 382)、`filter.rs:165/175` は絶対 `1e-10` で len ≤ 256・fft_size 256、`vec.rs:339` は `1e-12·max(1,\|a\|,\|b\|)` 相対で最大 ~1M をカバーするが vec カーネルのみ、`fft.rs` は `1e-10·(1+norm)` で n ≤ 2^10 と workspace 経路 2^14..2^16。`Correlation`/`Envelope` カーネル全体を実運用 FFT サイズで直接アサートするテストは存在せず、偏差上限は未測定 — 「~1e-13 級」という理論推定ではなく実測を根拠とする |

`max(dispatch 間偏差) << rel_eps << min(M − 次位大きさ)` を満たす eps が
存在しないと T1 が示した場合は実装を進めず、E-3 拡張または Option C
フォールバックとして spec 改訂へエスカレーションする(spec Req 7 の
緊急時方針)。

`tests/prepared.rs` の2テストは T1/T2 の意味論シフトを直接示す
観測点でもある:

- `prepared_methods_keep_all_global_evidence_exactly`
  (`prepared.rs:51-98`): `template` / `−template` の2出現を埋め込む
  ため `|corr|` が厳密同点となり、証拠は `AmbiguousAlignment` +
  候補保持となる。候補保持のため `position == 6952` の候補観測断言は
  存続し、`locate` == `locate_prepared` の同一性断言も validity を
  含めて成立し続ける
- `coarse_fallback_preserves_periodic_broad_and_threshold_adjacent_evidence`
  (`prepared.rs:101-141`): 周期 sin テンプレートのため同様に曖昧と
  なり得る。`evaluate` の受理可否比較断言は「actual と expected が
  同じ分類に落ちる」ことで存続するが、証拠 validity が `Valid` →
  `AmbiguousAlignment` へ意味的に変わる点は変更記録に残す

### 8. T2/T3 タスク輪郭

- **T2(実装)**: §1-§6 のコード変更 + ユニットテスト
  (`methods.rs` モジュール内: 厳密同点・近同点・平坦 plateau が
  `AmbiguousAlignment`+候補保持、単一候補・意味ある分離が非曖昧、
  **幅広・平坦頭部を持つ単一ピーク(競合する別位置を持たない
  単峰曲線)が非曖昧であること** — この pin は §7 のショルダー
  デルタ曝露を直接拘束し、ショルダーが `tie_window` 内に入る実
  入力では公式と矛盾し得るため eps 較正の実質上限として機能する;
  `consensus.rs` モジュール内: 全証拠曖昧で `AmbiguousAlignment`、
  混在時の分類優先順位、`reject_reason` 文言;`stage.rs`:
  `merged_votes` の非曖昧優先、`best_window_match` のペア単位
  ガード) + golden pin 更新 + `tests/prepared.rs` の意味論シフト
  追随(§7 末尾)
- **T3(完了証明)**: `SpectralEnvelope::locate` の逐次内積/ノルム
  (`methods.rs:539-547`、残存許容コメント付き)を `dsp::corr0`
  (`dsp.rs:38` で re-export 済み、`al-math/src/vec.rs:248`)へ置換し、
  残存許容コメントを除去する。厳密同点 → 構造的曖昧検出 → 拒否維持を
  `pin-periodic` golden で検証する(TASK-034EpPesSpbjbAQBsmjvJx の
  残存許容解消)

## Non-Functional Constraints

- **dispatch 不変性**: `rel_eps` は scalar↔SIMD 間最大偏差より厳密に
  大きい値に較正する。irreducible な不安定帯 `eps ± dispatch_deviation`
  内に入る入力では判定が反転し得る — この帯の存在は契約として正直に
  記述する(spec Req 6a)。al-locate 側に dispatch override seam はない。
  カーネル等価性の既存担保は `al-math` 自身のテストだが、現状の
  アサーションは小入力中心(絶対 `1e-10` だが `correlation.rs:193` は
  信号長 ≤ 382、`filter.rs` は fft_size 256 まで)で、実運用 FFT
  サイズ(~64k-point)での `ncc`/`gcc_phat`/`spectral_envelope` 偏差は
  未測定 — T1 が al-math 側の特性計測テストで実測して埋める(§7)
- **決定性**: 同一入力・同一 dispatch で `argmax_tie_count` は同一値。
  同点を index 順・最小 lag で決定的に解くことは受理/confidence の
  根拠にしてはならない — stable sort の順序は `candidates` の表示
  順序としてのみ許容される(spec Req 5)
- **性能**: 判定は曲線長 O(n) の走査2回(max + count)。NCC 本体
  (FFT)に比し無視できる。`peaks_from_correlation`・coarse 探索・
  ranking 経路は不変
- **observability**: 曖昧証拠でも `candidates` は保持され per-pair の
  `evidence.json` に永続化される。`reject_reason` のマーカー句
  `"indistinguishable"` で機械的識別が可能
- **永続化語彙**: `Validity` variant は増やさない — snake_case の
  `ambiguous_alignment` は既出。`propagate_validity`
  (`al-analysis/src/records.rs:79`)は `AmbiguousAlignment` を既に
  正しく写し、occurrence クラスタリングは Valid のみを位置寄与と
  するため下流は無変更で一貫する
- **設定面**: `rel_eps`/`abs_floor` は `ConsensusConfig` には入れない
  (下記 Risks and Trade-offs で根拠を記述)

## Risks and Trade-offs

- **eps 実現可能性**: eps の上限を拘束するのは rank-1 と第二順位
  大きさの差(argmax 自身のショルダーを含む)であり、受理系
  fixture で十分なマージンが確保できるかは現時点で仮説。帯域制限
  ソースのショルダーデルタ(~1e-3..1e-6 級)は eps 候補レンジ
  (1e-9..1e-6)と重なり得る(§7)。T1 が
  `max(dispatch 間偏差) << eps << min(M − 次位大きさ)` の範囲を
  示さない場合は実装せず spec 改訂へエスカレーションする
  (緊急時方針。E-3 または Option C)
- **`spectral_envelope` の慢性曖昧**: 32-bin log 振幅包絡 NCC は
  平坦スペクトル入力(ホワイトノイズ・チャープ・広帯域 SFX)で
  殆どの lag が準同点となり、現実入力の広いクラスでメソッドが
  恒常的に `AmbiguousAlignment` となって投票を止め得る。T1 が
  これを示した場合は eps 再較正ではなく spec Req 7 のエスカレー
  ション経路を取る — spec AC「受理系 fixture では全メソッドの
  merged 証拠が非曖昧」の再検討を含む(§7)
- **`tests/prepared.rs` の意味論シフト**: `template`/`−template`
  出現による厳密同点・周期 sin テンプレートにより証拠 validity が
  `Valid` → `AmbiguousAlignment` + 候補保持へ変わる。同一性・候補
  観測断言は候補保持により存続するが、意味の変化は変更記録に残す
  (§7 末尾)
- **縮退入力の受諾済み意味論**: 等スコアで複数回埋め込まれた真に
  識別不能な繰返しは「正当な複数出現」ではなく曖昧として報告される。
  spec Req 7 で受け入れ済みの契約意味論
- **plateau のサンプル計数**: `|T|` をサンプル数で数えるため、
  同点 plateau が幅2以上の曲線は plateau のみでも曖昧となる。
  cluster 計数への代替は spec の判定式(index 集合基数)と受理基準
  (平坦 plateau の曖昧 pin)に反するため棄却した
- **定数の配置(ConsensusConfig でなく methods.rs の const)**: 値は
  ユーザー設定ではなく T1 で較正される契約定数であり、
  `ConsensusConfig` は `params` へ serde 永続化されるため実装詳細を
  設定面へ露出させない。`LocateMethod::locate` のシグネチャに設定を
  通す経路もないため、const + 共有ヘルパが最小かつ唯一の定義点と
  なる(KISS)
- **`merged_votes` の全ペア曖昧条件**: 非曖昧(他の非 Valid を含む)
  ペアが1つでもあれば merged は非曖昧となるため、メソッドの曖昧性が
  結果レベルで見えない組合せがあり得る。spec Req 8 が pin する規則に
  忠実に従い、per-pair の `evidence.json` で詳細を辿れることを根拠に
  受け入れる
- **reject_reason 文言の変更**: golden
  `pinned_periodic_everywhere_not_accepted_at_wrong_phase` が pin する
  "method(s) agree" 断言を新文言へ更新する — 意図的な契約変更として
  変更記録に残す(spec Req 10)
- **`min_agreeing_methods = 1` の解釈**: 曖昧除外は適用されるが、
  非曖昧の単一投票は受理し得る。契約の拒否権は証拠の投票除外を通じた
  ものであり、他メソッドの投票への拒否権ではない

## Deferred Design Concerns

- Option C(周期性・自己相関モデルによる位相同値性の意味的検証)、
  E-3(位置間隔の周期性項)— T1 で eps 実現不能が判明した場合の
  エスカレーション先として spec が留保。無視した場合のリスクは
  等スコア多重埋め込みの受理不能(本契約では曖昧報告を受諾済み)
- `merged_votes` の符号付きスコア選択クオーク(候補空 0.0 > 負
  スコア)— spec が対象外とした既知挙動。`Valid` ペアの top スコアが
  負の場合、候補空ペア(0.0 扱い)に敗れて証拠が捨てられ得るため、
  極性反転のみの内容では偽棄却があり得る。観察事項として記録のみ
- `best_window_match` の最終選択は符号付き `c.score > b.score` /
  `min_score` 比較であり、同点判定の `|corr|` 大きさ基準とは非対称
  (極性反転ペアは同点判定では大きさ同点・最終選択では負スコア扱い)
  — 既存挙動として変更しない(§5)
- `LowCorrelation` で候補保持されたペアが、閾値を緩めた設定下で
  メソッドの曖昧証拠を覆い隠して投票し得る設定依存の穴 — spec
  Req 3 が対象外として記録済み。本契約は変更しない
- 将来 B-2(新 `Validity` variant)へ移行する場合は
  `propagate_validity`(`records.rs:85`)の `_ => ComputationFailed`
  catch-all への明示アーム追加が必須(ADR 記録済み)

## Verification Notes

- [ ] T1 特性計測テストが実装に先行して存在し、受理系 fixture
  (golden + `tests/locate.rs` の非 golden 受理系を含む)のメソッド毎
  `M − max_{i≠argmax}|corr[i]|` マージン分布(ショルダーデルタを
  含む)、`spectral_envelope` の noise 系入力での曖昧頻度、および
  al-math 内 characterization test の実運用サイズ dispatch 偏差実測
  から `rel_eps`/`abs_floor` の最終値が実測記録と共に確定されている
- [ ] `pin-periodic` が受理されず `validity == "ambiguous_alignment"`、
  全4メソッドの merged 証拠が `AmbiguousAlignment` + `candidates`
  保持 + `reject_reason` が "indistinguishable" を含む。full_ncc
  候補 ≥2 かつ全位置が周期倍数として観測可能。判定が実行間・
  dispatch 間で決定的
- [ ] `pin-periodic-control` が tolerance 64 で受理を維持し、
  `case_repeated_source`/`case_loop_material` ほか受理系 golden で
  全メソッドの merged 証拠が非曖昧のまま受理される(false positive
  なし)
- [ ] `pinned_low_coherence_no_inflated_lag` が不受理・
  `agreeing_methods < 2` を維持する
- [ ] ユニットテスト: 厳密同点・近同点・平坦 plateau で証拠が
  `AmbiguousAlignment`+候補保持;単一候補・意味ある分離の曲線、
  および幅広・平坦頭部の単一ピーク(競合する別位置を持たない
  単峰曲線)は非曖昧;全証拠曖昧の入力で合議が
  `AmbiguousAlignment` を返す
  (`LowCorrelation`/`InsufficientSupport` に落ちない)
- [ ] 曖昧経路の `reject_reason` pin: 証拠レベルは
  "indistinguishable"、空投票・曖昧起因の結果レベルは
  "method(s) agree" を含まない専用文言、合意不足+曖昧除外時は
  "; N method(s) excluded as ambiguous" 付加
- [ ] `best_window_match` が narrow/×8/全範囲の全段で同点ペアを
  アンカー非寄与とし、全ペアが同点または閾値未達のときのみ
  `None` を返す(ペア単位ガード)
- [ ] `tests/prepared.rs` が新意味論(厳密同点証拠の
  `AmbiguousAlignment`+候補保持)で合格し、
  `al_locate::evaluate(.., "test", "2")` が記録される
- [ ] 全メソッド `method_version` と合議 `algorithm_version` が
  `"2"` で永続化される
- [ ] T3: `spectral_envelope` の `dsp::corr0` 置換後も周期 fixture の
  拒否が構造的に維持される(残存許容コメント除去)
- [ ] `mise exec -- cargo test -p al-locate` と
  `mise exec -- cargo test -p al-math`(特性計測テストを含む)が合格
