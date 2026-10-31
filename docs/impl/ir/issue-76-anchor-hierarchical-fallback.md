---
id: "IMPL-76AnchorHierFB"
type: "impl"
status: "completed"
title: "Issue #76 window_anchors_in 階層フォールバック (al-locate)"
created: "2026-10-31"
updated: "2026-10-31"
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
  related:
    - "docs/impl/ir/perf-reeval-2026-10.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "decimation=8 + K=256 + strict-tie + near-gate margin"
    rejected:
      - "3% near-tie competition guard (fallback ~97% に縮退)"
      - "decimation=4 (低速化し周期性双ピークを解決せず)"
---

# Issue #76 window_anchors_in 階層フォールバック

## Summary

`window_anchors_in` の widen/full フォールバック(全 window の
~44%/~18% が到達、anchor CPU の ~90%)を、間引き信号上の粗 NCC +
原レート近傍精密化の2段構成へ置き換えた。粗探索は位置の候補
提案のみに使い、受理判定(スコア・peak_to_background・同点
ガード)はすべて原レートの精密化結果で行う。canonical で
anchor 計測 ~252s → ~68s(約 3.7 倍)、locate 270s → 88s、
golden 9 ラン全件 0 diff で契約を維持した。

## Context

- 再評価(`perf-reeval-2026-10`): canonical で anchor ~252s /
  locate ~270s(~93%)。full shape(tpl=512, sig≈2.4M)の単発
  `plan.ncc` は ~210ms、16 並列帯域競合下で実効 ~2.76s。
- 置換構想: ~4.19M FFT の全域 NCC を ~524K FFT の粗 NCC +
  少数近傍 NCC + 標本背景推定へ置き換える。

## Implementation

### Changes

**`crates/al-locate/src/stage.rs`**
- `AnchorPyramid`: 受理済み (source, capture) ペアごとに 1 回
  構築する間引き済み全チャンネル表現。`ResamplePlan` の
  アンチエイリアス間引きを使用し `up/down` 有理比で座標変換
  (`orig↔coarse`)する。窓ワーカー間で読み取り専用共有。
  `hier_calls` / `exact_fallbacks[7]` の atomic カウンタを持ち、
  理由別の縮退頻度を instrumentation で観測する。
- `anchor_pyramid()`: `1/HIER_DECIMATION` (= 1/8) レートで
  構築。`refine_radius = 2·plan.radius() + rate/coarse_rate`
  で FIR 半径・位相量子化・座標マッピング誤差を内包する
  近傍幅とする。
- `pair_match()`: 従来の直接ペア評価を関数化(全域 NCC →
  `argmax_tie_count` 同点ガード → `peaks_from_correlation` トップ
  ピーク)。挙動は従来と同一。
- `pair_match_hier()`: 階層ペア評価。
  1. 探索域に対応する粗信号区間で NCC → 上位
     `HIER_REFINE_PEAKS`(=256)ピークを候補化
  2. 各候補を原レート ±refine_radius の近傍 NCC で精密化
  3. 精密化位置を `2·refine_radius` 幅でクラスタリングし、
     同一局所ピークの重複候補を併合
  4. トップ候補の `corr0` ストライド標本で背景 median を推定
  5. 受理閾値との境界判定(near-gate)
- `best_window_match()`: `hier` 引数追加。探索域長が
  `HIER_MIN_SIGNAL`(128K samples)以上かつ粗テンプレ長が
  `HIER_MIN_COARSE_TEMPLATE`(8)以上のとき階層経路、それ以外は
  直接経路。narrow 層は常に直接(`None`)。
- `window_anchors_in()`: `hier` 引数を widen/full 層へ伝播。
  呼出側(`run`)は受理ペアで `anchor_pyramid` を構築して渡す。
  narrow 層の探索域・受理閾値・MAX_WINDOWS・オフセット格子は
  不変更。呼出ごとに tier 計数・anchor 数・経過時間・理由別
  fallback 計数を `info!` イベントとして出力する。
- `sampled_ncc_background()`: ラグ軸ストライド標本
  (`HIER_BG_SAMPLES` = 16K 点)の `corr0` median。
  `peaks_from_correlation` が全曲線から求める背景の標本推定
  (全曲線計算を避けるための近似)。
- `crates/al-locate/src/methods.rs`: `TIE_REL_EPS` /
  `TIE_ABS_FLOOR` を `pub(crate)` 化(精密化同点窓の再利用)。

### 縮退条件(exact fallback の理由コード)

曖昧・境界ケースでは `pair_match` と同一の直接全域 NCC へ縮退し、
既存判定を厳密に再現する。理由コード:
`0=shape/適格外`、`1=粗同点`、`2=粗候補なし`、`3=精密化候補なし`、
`4=精密化厳密同点(距離分離)`、`5=背景未満`、`6=受理閾値境界`。

### 段構成・パラメータの根拠

- **間引き率 1/8**: 粗テンプレ 64 samples(512/8)で粗 NCC の
  FFT を ~1/8 へ。`ResamplePlan` は `radius = 256`(入力単位)で
  十分なエイリアス抑止。dec=4 も試したが周期性双ピークの
  取りこぼしを解決できず ~1.7 倍低速だったため不採用
  (下記 rejected 参照)。
- **精密化近傍 `±(2·256 + 8)`**: FIR 半径(±256)+ 座標マッピング
  量子化(±8)を内包し、粗ピーク位置の原レート真値を確実に含む。
- **K=256 候補**: 音楽信号の周期性で粗曲線が自己相似ピークに
  埋まるため浅い K では真ピークを取りこぼす。K=48 で観測
  3 乖離のうち 2 件を解消、K=256 で候補カバレッジを確保した。
  1 候補の精密化は ~60µs 程度で、K=256 でも ~15ms/評価と
  全域 NCC(~250ms)より依然 1 桁小さい。
- **厳密同点のみで曖昧性を判定**: `argmax_tie_count` と同じ
  `tie_window = max(TIE_REL_EPS·M, TIE_ABS_FLOOR)`(1e-11)を
  クラスタ間で適用。「近いが等しくない」競合は曖昧性ではなく
  粗カバレッジの問題であり、near-tie 縮退では解決できない
  (実測で ~97% が縮退し高速化が消えたため採用しない)。
- **near-gate マージン**: 標本背景は全曲線 median と ~0.2% 程度
  ずれるため、p2b が `min_peak_to_background` の ±2%
  (`HIER_P2B_GATE_MARGIN`)、score が `min_score` の ±1e-9
  (`HIER_SCORE_GATE_EPS`)に入る場合のみ direct へ縮退する。
  canonical で ~8% が発動するが、それ以外は標本推定で受理
  判定を完結できる。

## Validation and Evidence

- [x] `cargo check -p al-locate` / `cargo fmt --all -- --check` /
  `cargo clippy --workspace --all-targets -- -D warnings` pass
- [x] 単体テスト(`stage::tests`)5件 pass
  - `hierarchical_match_agrees_with_direct_on_embedded`: 遠方
    埋め込みで直接/階層の位置・スコア一致
  - `hierarchical_match_rejects_pure_noise_like_direct`: 純
    ノイズで受理判定一致(None)
  - `window_anchors_identical_with_and_without_hierarchy`: 遠隔
    オフセットの全窓でアンカー列(件数・位置・スコア)一致
  - `sampled_background_approximates_full_curve_median`: 標本
    背景が全曲線 median の近似に留まる
  - 乖離窓(`src=1124640` の p2b 境界ケース)は実信号の一時
    診断で再現・修正を確認し、最終的には 9 ラン golden 0 diff で
    検証(診断コードは外部ファイル依存のため未収録)
- [x] 9ラン golden 比較(`tools/baseline/capture.mjs` +
  `compare.mjs`、閾値 rel=1e-9/abs=1e-12)で **全件 0 diff**
  - canonical / variant-desync / variant-clipping / variant-reverb /
    variant-filter / variant-clicks / small: いずれも base76 比
    0 diff(anchor 位置・score・consensus・support 件数すべて一致)
  - resume: base76 比 0 diff、canonical 比 0 diff
  - aa(決定性確認ラン): base76 比 0 diff、canonical 比 0 diff
- [x] 性能計測(`tools/perf-reeval/capture.mjs`、同一ツール同一
  手順で base/after を取得)

  | run | 指標 | base | after | 倍率 |
  | --- | --- | --- | --- | --- |
  | canonical | e2e wall | 368.0s | 186.8s | ~2.0x |
  | canonical | locate stage | 270.2s | 87.6s | ~3.1x |
  | canonical | `window_anchors_in` 合計 | ~252.0s | ~68.0s | ~3.7x |
  | small | e2e wall | 126.9s | 85.3s | ~1.5x |
  | small | locate stage | 111.8s | 71.1s | ~1.6x |
  | small | `window_anchors_in` 合計 | ~51.5s | ~11.8s | ~4.4x |

  canonical の `window_anchors_in` 内訳(2 ペア分):
  - `hier_calls` = 1196 / 1176、`exact_fallbacks` = 96 / 116
    (~8-10%)、`anchors` = 468(base と一致)
  - 理由別: `fb_coarse_tie` = 16 / 16、`fb_near_gate` = 80 / 100、
    `fb_shape` = `fb_coarse_empty` = `fb_refined_empty` =
    `fb_refined_tie` = `fb_background` = 0
  - `elapsed_ms` = 32.1s / 35.8s(対 base 130.2s / 121.8s)

### 採択・棄却した設計の記録

- **棄却: 上位候補の「競合過多」縮退(10%/3% near-tie 窓)**:
  粗曲線のトップ N に近値ピークが並ぶ場合に direct へ縮退する
  ガードを試したが、BGM の自己相似性により canonical で
  ~97-98% が縮退し `elapsed` が ~126s に逆戻り。近いが等しく
  ないピークの存在は曖昧性ではなく、契約に関係しないため
  厳密同点(1e-11)のみを残した。
- **棄却: decimation=4**: 周期性双ピーク(0.9788 vs 0.9778 の
  双極)の粗カバレッジ改善を狙ったが取りこぼしは解消せず、
  pair あたり ~38s(dec=8 は ~22s)へ低速化。乖離の真因は粗
  カバレッジではなく p2b 閾値境界(下記)だったため不採用。
- **発見・解決: p2b 閾値のナイフエッジ**: 初期実装で
  `src=1124640` の anchor が widen 層で別位置へ乖離した。
  旧実装は全曲線 median の背景で p2b=1.999(<2.0)となり widen
  を拒否して full へ昇格、新実装の標本背景では 2.003(≥2.0)
  で widen 受理となる境界ケースだった。`HIER_P2B_GATE_MARGIN`
  (±2%)内なら direct へ縮退する near-gate を追加して解決。

## Risks or Follow-ups

- 粗上位 K=256 に真ピークが入らない理論的余地は残るが、
  K 到達率の計測・9 ラン golden 0 diff・単体一致テストで
  実用上の一致を確認済み。縮退は曖昧・境界ケースのみに限定
  される設計であり、誤受理より縮退を優先する。
- `peak_width` は近傍曲線上の半値幅であり従来の全曲線幅とは
  意味論が微妙に異なるが、anchor の受理判定・永続値には使わ
  れない(Anchor は source/capture/score のみ保持)。
- `sampled_ncc_background` は標本推定のため、near-gate 帯の外
  でも全曲線 median と微小差がありうるが、受理判定が閾値から
  離れた領域でのみ使われるよう境界帯は direct 縮退で保護した。
- 隣接 anchor 予測による fallback 頻度削減(仕様の第2優先)は
  今回未実施。anchor CPU が ~90% → 大幅低減した現状では
  セマンティクス変更リスクに見合わない。
- plan shape 量子化(効果 ~2% 見込み)は仕様どおり単独実施せず。
